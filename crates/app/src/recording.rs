use crate::studio::{EditorTab, Studio};
use dawwny_audio::{LiveEvent, RecordedEvent};
use dawwny_core::{Clip, Note, Project, SessionStore, Track, new_id};
use std::collections::BTreeMap;

struct HeldNote {
    start: f64,
    velocity: f32,
    down: bool,
}
pub struct Take {
    track: Track,
    start: f64,
    song_end: f64,
    notes: Vec<Note>,
    held: BTreeMap<(u8, u8), HeldNote>,
    sustain: [bool; 17],
    capacity: usize,
    overflow: bool,
}
#[derive(Default)]
pub struct RecordingState {
    pub take: Option<Take>,
    pub warning: Option<String>,
    pub recovery: Option<Project>,
}

impl Take {
    pub fn matches_track(&self, track: Option<&Track>) -> bool {
        track.is_some_and(|track| track.id == self.track.id)
    }
    fn new(track: Track, start: f64, song_end: f64, capacity: usize) -> Self {
        Self {
            track,
            start,
            song_end,
            capacity,
            notes: Vec::new(),
            held: BTreeMap::new(),
            sustain: [false; 17],
            overflow: false,
        }
    }
    fn close(&mut self, key: (u8, u8), beat: f64) {
        if let Some(held) = self.held.remove(&key) {
            let duration = (beat - held.start)
                .max(1.0 / 960.0)
                .min(self.song_end - held.start);
            if duration > 0.0 {
                self.notes.push(Note {
                    id: new_id(),
                    pitch: key.1,
                    start: held.start,
                    duration,
                    velocity: held.velocity,
                });
            }
        }
    }
    fn event(&mut self, recorded: RecordedEvent) {
        if !recorded.beat.is_finite() {
            self.overflow = true;
            return;
        }
        let beat = recorded.beat.clamp(self.start, self.song_end);
        match recorded.event {
            LiveEvent::NoteOn {
                channel,
                pitch,
                velocity,
            } if channel <= 16 && pitch <= 127 && velocity.is_finite() && velocity > 0.0 => {
                let key = (channel, pitch);
                self.close(key, beat);
                if self.notes.len() + self.held.len() < self.capacity && beat < self.song_end {
                    self.held.insert(
                        key,
                        HeldNote {
                            start: beat,
                            velocity: velocity.min(1.0),
                            down: true,
                        },
                    );
                } else {
                    self.overflow = true;
                }
            }
            LiveEvent::NoteOff { channel, pitch } if channel <= 16 => {
                if self.sustain[channel as usize] {
                    if let Some(held) = self.held.get_mut(&(channel, pitch)) {
                        held.down = false;
                    }
                } else {
                    self.close((channel, pitch), beat);
                }
            }
            LiveEvent::Sustain { channel, down } if channel <= 16 => {
                self.sustain[channel as usize] = down;
                if !down {
                    let released: Vec<_> = self
                        .held
                        .iter()
                        .filter(|(key, note)| key.0 == channel && !note.down)
                        .map(|(key, _)| *key)
                        .collect();
                    for key in released {
                        self.close(key, beat);
                    }
                }
            }
            LiveEvent::AllOff => {
                while let Some(key) = self.held.keys().next().copied() {
                    self.close(key, beat);
                }
                self.sustain.fill(false);
            }
            _ => {}
        }
    }
    fn finish(mut self, beat: f64) -> (Track, Option<Clip>, bool) {
        self.event(RecordedEvent {
            beat,
            event: LiveEvent::AllOff,
        });
        if self.notes.is_empty() {
            return (self.track, None, self.overflow);
        }
        let start = (self.start / 4.0).floor() * 4.0;
        let end = self
            .notes
            .iter()
            .map(|note| note.start + note.duration)
            .fold(start, f64::max);
        let length = ((end / 4.0).ceil() * 4.0).min(self.song_end) - start;
        for note in &mut self.notes {
            note.start -= start;
        }
        self.notes
            .sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
        (
            self.track,
            Some(Clip {
                id: new_id(),
                name: "Recorded take".into(),
                start,
                length,
                notes: self.notes,
            }),
            self.overflow,
        )
    }
}

impl Studio {
    pub fn recording(&self) -> bool {
        self.recording.take.is_some()
    }

    pub fn save_recovered_take(&mut self) {
        let Some(project) = &self.recording.recovery else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("dawwny take", &["json"])
            .set_file_name("recovered-take.dawwny.json")
            .save_file()
        {
            if path.exists() {
                self.fail("Choose a new file for the recovered take".into());
                return;
            }
            match SessionStore::new(path).initialize(project) {
                Ok(_) => {
                    self.recording.recovery = None;
                    self.recording.warning = None;
                    self.status = "Recovered take saved".into();
                    self.error = false;
                }
                Err(error) => self.fail(format!("Could not save the recovered take: {error}")),
            }
        }
    }
    pub fn toggle_recording(&mut self) {
        if self.recording() {
            self.finish_recording();
            if let Some(audio) = &mut self.audio {
                audio.pause();
            }
            return;
        }
        if self.recording.recovery.is_some() {
            self.fail("Save the recovered take in Keys before starting another recording".into());
            return;
        }
        if !self.commit() {
            return;
        }
        self.recording.warning = None;
        let Some(track) = self.project.tracks.get(self.selected_track).cloned() else {
            self.fail("Add and select an instrument track before recording".into());
            return;
        };
        let clips = self
            .project
            .tracks
            .iter()
            .map(|track| track.clips.len())
            .sum::<usize>();
        let notes = self
            .project
            .tracks
            .iter()
            .flat_map(|track| &track.clips)
            .map(|clip| clip.notes.len())
            .sum::<usize>();
        if clips >= 128 || notes >= 32768 {
            self.fail("The session has reached its clip or note limit".into());
            return;
        }
        self.sync_audio();
        let Some(audio) = &mut self.audio else {
            self.fail("Recording needs an available audio output".into());
            return;
        };
        // A linear take avoids silently folding multiple passes into one clip.
        let was_looped = self.looped;
        self.looped = false;
        audio.set_looped(false);
        let started = audio
            .play(&self.project, false)
            .and_then(|_| audio.start_recording());
        if let Err(error) = started {
            self.looped = was_looped;
            audio.set_looped(was_looped);
            self.fail(format!("Could not start recording: {error}"));
            return;
        }
        self.recording.take = Some(Take::new(
            track,
            audio.position_beats(),
            self.project.length_bars as f64 * 4.0,
            32768 - notes,
        ));
        self.keyboard.open = true;
        self.status = "Recording · play Keys or MIDI · R or Pause finishes the take".into();
        self.error = false;
    }

    pub fn collect_recording(&mut self) {
        if let Some(audio) = &mut self.audio {
            while let Some(event) = audio.recorded_event() {
                if let Some(take) = &mut self.recording.take {
                    take.event(event);
                }
            }
        }
    }

    pub fn finish_recording(&mut self) {
        if !self.recording() {
            return;
        }
        let mut end = self.position();
        let mut capture_error = None;
        let mut overflow = false;
        if let Some(audio) = &mut self.audio {
            match audio.finish_recording() {
                Ok(beat) => end = beat,
                Err(error) => capture_error = Some(error.to_string()),
            }
            overflow = audio.recording_overflowed();
            audio.all_notes_off();
        }
        self.collect_recording();
        let Some(take) = self.recording.take.take() else {
            return;
        };
        let (mut original_track, clip, take_overflow) = take.finish(end);
        let Some(clip) = clip else {
            self.status = "No notes recorded".into();
            return;
        };
        original_track.clips = vec![clip.clone()];
        let backup = Project {
            id: new_id(),
            name: "Recorded take".into(),
            tempo: self.project.tempo,
            length_bars: ((clip.start + clip.length) / 4.0).ceil() as u32,
            tracks: vec![original_track.clone()],
            ..Project::default()
        };
        let directory = self
            .store
            .path()
            .parent()
            .unwrap_or(std::path::Path::new("sessions"))
            .join(".dawwny-takes");
        let path = directory.join(format!("{}.dawwny.json", clip.id));
        let backup_result = SessionStore::new(path.clone()).initialize(&backup);
        if backup_result.is_err() {
            self.recording.recovery = Some(backup);
        }
        let Some(index) = self
            .project
            .tracks
            .iter()
            .position(|track| track.id == original_track.id)
        else {
            self.fail(format!(
                "The recorded track was removed. {}",
                backup_result.map_or_else(
                    |error| format!("Take backup failed: {error}"),
                    |_| format!("Take preserved in {}", path.display())
                )
            ));
            return;
        };
        let id = clip.id.clone();
        self.edit(|project| project.tracks[index].clips.push(clip));
        if !self.project.tracks[index]
            .clips
            .iter()
            .any(|clip| clip.id == id)
        {
            self.fail(format!(
                "The take cannot fit in the current project. {}",
                backup_result.map_or_else(
                    |error| format!("Backup failed: {error}"),
                    |_| format!("Take preserved in {}", path.display())
                )
            ));
            return;
        }
        self.recording.recovery = None;
        if self.selected_track == index {
            self.selected_clip = self.project.tracks[index].clips.len() - 1;
            self.tab = EditorTab::Piano;
        }
        if capture_error.is_some() || overflow || take_overflow {
            let warning = format!(
                "Take captured with missing input events. {}",
                capture_error.unwrap_or_else(|| "Input queue or note capacity was exceeded".into())
            );
            self.recording.warning = Some(warning.clone());
            self.fail(warning);
        } else if !self.error {
            self.status = "Recorded take added · unquantized timing · Undo removes it".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(take: &mut Take, beat: f64, event: LiveEvent) {
        take.event(RecordedEvent { beat, event });
    }
    #[test]
    fn recording_preserves_velocity_sustain_overlaps_and_closes_held_notes() {
        let mut take = Take::new(dawwny_core::demo_project().tracks.remove(0), 5.0, 64.0, 10);
        event(
            &mut take,
            5.25,
            LiveEvent::NoteOn {
                channel: 0,
                pitch: 60,
                velocity: 0.8,
            },
        );
        event(
            &mut take,
            5.5,
            LiveEvent::Sustain {
                channel: 0,
                down: true,
            },
        );
        event(
            &mut take,
            6.0,
            LiveEvent::NoteOff {
                channel: 0,
                pitch: 60,
            },
        );
        event(
            &mut take,
            6.5,
            LiveEvent::NoteOn {
                channel: 1,
                pitch: 60,
                velocity: 0.3,
            },
        );
        event(
            &mut take,
            7.0,
            LiveEvent::Sustain {
                channel: 0,
                down: false,
            },
        );
        let (_, clip, overflow) = take.finish(8.0);
        let clip = clip.unwrap();
        assert!(!overflow);
        assert_eq!(clip.start, 4.0);
        assert_eq!(clip.length, 4.0);
        assert_eq!(clip.notes.len(), 2);
        assert_eq!(
            (
                clip.notes[0].start,
                clip.notes[0].duration,
                clip.notes[0].velocity
            ),
            (1.25, 1.75, 0.8)
        );
        assert_eq!(
            (
                clip.notes[1].start,
                clip.notes[1].duration,
                clip.notes[1].velocity
            ),
            (2.5, 1.5, 0.3)
        );
    }
    #[test]
    fn retriggers_limits_and_project_end_remain_valid() {
        let mut track = dawwny_core::demo_project().tracks.remove(0);
        track.instrument = dawwny_core::Instrument::Synth;
        track.clips.clear();
        let mut take = Take::new(track, 0.0, 4.0, 2);
        for beat in [1.0, 2.0, 3.0] {
            event(
                &mut take,
                beat,
                LiveEvent::NoteOn {
                    channel: 16,
                    pitch: 60,
                    velocity: 0.5,
                },
            );
        }
        let (mut track, clip, overflow) = take.finish(8.0);
        assert!(overflow);
        let clip = clip.unwrap();
        assert_eq!(clip.notes.len(), 2);
        assert_eq!(clip.notes[0].duration, 1.0);
        track.clips.push(clip);
        dawwny_core::validate(&Project {
            length_bars: 1,
            tracks: vec![track],
            ..Project::default()
        })
        .unwrap();
    }
    #[test]
    fn removed_tracks_and_backup_failures_keep_a_recoverable_performance() {
        let dir = tempfile::tempdir().unwrap();
        let mut studio = Studio::new(dir.path().join("song.json"), true).unwrap();
        let mut take = Take::new(studio.project.tracks.remove(0), 0.0, 64.0, 100);
        event(
            &mut take,
            0.25,
            LiveEvent::NoteOn {
                channel: 16,
                pitch: 64,
                velocity: 0.7,
            },
        );
        event(
            &mut take,
            1.25,
            LiveEvent::NoteOff {
                channel: 16,
                pitch: 64,
            },
        );
        std::fs::write(
            dir.path().join(".dawwny-takes"),
            "blocks the backup directory",
        )
        .unwrap();
        studio.recording.take = Some(take);
        studio.finish_recording();
        assert!(studio.error);
        let recovery = studio.recording.recovery.as_ref().unwrap();
        assert_eq!(recovery.tracks[0].clips[0].notes[0].pitch, 64);
        dawwny_core::validate(recovery).unwrap();
        studio.toggle_recording();
        assert!(!studio.recording());
        assert!(studio.status.contains("Save the recovered take"));
    }
    #[test]
    fn finishing_a_take_is_non_destructive_persistent_and_undoable() {
        let dir = tempfile::tempdir().unwrap();
        let mut studio = Studio::new(dir.path().join("song.json"), true).unwrap();
        let before = studio.project.clone();
        let mut take = Take::new(before.tracks[0].clone(), 0.0, 64.0, 100);
        event(
            &mut take,
            0.25,
            LiveEvent::NoteOn {
                channel: 16,
                pitch: 64,
                velocity: 0.7,
            },
        );
        event(
            &mut take,
            1.25,
            LiveEvent::NoteOff {
                channel: 16,
                pitch: 64,
            },
        );
        studio.recording.take = Some(take);
        studio.finish_recording();
        assert!(studio.commit());
        let saved = studio.store.load().unwrap();
        assert_eq!(
            saved.tracks[0].clips.len(),
            before.tracks[0].clips.len() + 1
        );
        assert_eq!(
            saved.tracks[0].clips[..before.tracks[0].clips.len()],
            before.tracks[0].clips
        );
        let take = saved.tracks[0].clips.last().unwrap();
        assert_eq!(take.notes[0].duration, 1.0);
        assert!(
            dir.path()
                .join(".dawwny-takes")
                .join(format!("{}.dawwny.json", take.id))
                .exists()
        );
        studio.history(false);
        assert_eq!(studio.project.tracks, before.tracks);
    }
}
