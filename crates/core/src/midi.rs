use crate::*;
use anyhow::{Context, Result, bail, ensure};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
    num::{u4, u7, u15, u24},
};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fs::File,
    io::Read,
    path::Path,
};

pub fn export_midi(project: &Project, path: &Path) -> Result<()> {
    validate(project)?;
    let ticks = 960.0;
    let end = (project.length_bars as f64 * 4.0 * ticks).round() as u32;
    let mut tracks = vec![vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(
                (60_000_000.0 / project.tempo).round() as u32,
            ))),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        },
        TrackEvent {
            delta: end.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]];
    let mut melodic = 0;
    for track in &project.tracks {
        let channel = if track.instrument == Instrument::Drums {
            9
        } else {
            let n = melodic % 15;
            melodic += 1;
            if n >= 9 { n + 1 } else { n }
        };
        let channel = u4::new(channel);
        let program = match track.instrument {
            Instrument::Keys => 4,
            Instrument::Pad => 89,
            Instrument::Bass => 38,
            Instrument::Lead => 81,
            Instrument::Drums => 0,
        };
        let mut events = vec![];
        for clip in &track.clips {
            for note in &clip.notes {
                if note.velocity == 0.0 {
                    continue;
                }
                let on = ((clip.start + note.start) * ticks).round() as u32;
                let off = (((clip.start + note.start + note.duration) * ticks).round() as u32)
                    .max(on + 1);
                events.push((
                    on,
                    1,
                    note.pitch,
                    (note.velocity * 127.0).round().clamp(1.0, 127.0) as u8,
                ));
                events.push((off, 0, note.pitch, 0));
            }
        }
        events.sort_by_key(|e| (e.0, e.1, e.2));
        let mut out = vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::TrackName(track.name.as_bytes())),
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::ProgramChange {
                        program: u7::new(program),
                    },
                },
            },
        ];
        let mut last = 0;
        for (at, on, pitch, velocity) in events {
            out.push(TrackEvent {
                delta: (at - last).into(),
                kind: TrackEventKind::Midi {
                    channel,
                    message: if on == 1 {
                        MidiMessage::NoteOn {
                            key: u7::new(pitch),
                            vel: u7::new(velocity),
                        }
                    } else {
                        MidiMessage::NoteOff {
                            key: u7::new(pitch),
                            vel: u7::new(0),
                        }
                    },
                },
            });
            last = at;
        }
        out.push(TrackEvent {
            delta: end.saturating_sub(last).into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        tracks.push(out);
    }
    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(ticks as u16))),
        tracks,
    };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    smf.save(path)?;
    Ok(())
}

pub fn import_midi(path: &Path) -> Result<Project> {
    let mut bytes = vec![];
    File::open(path)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 4 * 1024 * 1024,
        "MIDI file exceeds 4 MiB limit"
    );
    let smf = Smf::parse(&bytes).context("Invalid MIDI file")?;
    ensure!(
        smf.header.format != Format::Sequential,
        "MIDI type 2 independent patterns are not supported"
    );
    let tpq = match smf.header.timing {
        Timing::Metrical(v) if v.as_int() > 0 => v.as_int() as f64,
        _ => bail!("SMPTE/zero-resolution MIDI timing is not supported"),
    };
    ensure!(
        smf.tracks.iter().map(Vec::len).sum::<usize>() <= 200_000,
        "MIDI event limit exceeded"
    );
    let mut project = Project {
        id: new_id(),
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported MIDI")
            .chars()
            .take(80)
            .collect(),
        tempo: 120.0,
        ..Default::default()
    };
    let mut tempo_us = None;
    let mut max_tick = 0;
    let mut note_count = 0;
    for (index, events) in smf.tracks.iter().enumerate() {
        let mut tick = 0u64;
        let mut name = format!("MIDI {}", index + 1);
        let mut notes: BTreeMap<u8, Vec<Note>> = BTreeMap::new();
        let mut open: HashMap<(u8, u8), VecDeque<(u64, u8)>> = HashMap::new();
        let mut programs = HashMap::new();
        for event in events {
            tick = tick
                .checked_add(event.delta.as_int() as u64)
                .context("MIDI time overflow")?;
            max_tick = max_tick.max(tick);
            ensure!(tick as f64 / tpq <= 1024.0 + 1e-8, "MIDI exceeds 256 bars");
            match event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(value)) => {
                    let v = value.as_int();
                    ensure!(v > 0, "Invalid zero MIDI tempo");
                    if tick > 0 && tempo_us.unwrap_or(500_000) != v {
                        bail!("Tempo maps are not supported; import a constant-tempo MIDI file");
                    }
                    if tempo_us.is_some_and(|old| old != v) {
                        bail!("Multiple MIDI tempos are not supported");
                    }
                    tempo_us = Some(v);
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(n, d, _, _)) => ensure!(
                    n == 4 && d == 2,
                    "Only 4/4 MIDI files are supported in this foundation"
                ),
                TrackEventKind::Meta(MetaMessage::TrackName(v)) => {
                    name = String::from_utf8_lossy(v)
                        .chars()
                        .filter(|c| !c.is_control())
                        .take(72)
                        .collect();
                }
                TrackEventKind::Midi { channel, message } => {
                    let channel = channel.as_int();
                    match message {
                        MidiMessage::ProgramChange { program } => {
                            programs.insert(channel, program.as_int());
                        }
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            open.entry((channel, key.as_int()))
                                .or_default()
                                .push_back((tick, vel.as_int()));
                        }
                        MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, .. } => {
                            if let Some((start, velocity)) = open
                                .get_mut(&(channel, key.as_int()))
                                .and_then(VecDeque::pop_front)
                            {
                                note_count += 1;
                                ensure!(note_count <= 32768, "MIDI exceeds note limit");
                                notes.entry(channel).or_default().push(Note {
                                    id: new_id(),
                                    pitch: key.as_int(),
                                    start: start as f64 / tpq,
                                    duration: ((tick - start) as f64 / tpq).max(1.0 / 960.0),
                                    velocity: velocity as f32 / 127.0,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        ensure!(
            open.values().all(VecDeque::is_empty),
            "MIDI contains notes without note-off events"
        );
        let count = notes.len();
        for (channel, mut channel_notes) in notes {
            channel_notes.sort_by(|a, b| a.start.total_cmp(&b.start));
            let length = channel_notes
                .iter()
                .map(|n| n.start + n.duration)
                .fold(4.0, f64::max)
                .div_euclid(4.0)
                * 4.0;
            let actual = channel_notes
                .iter()
                .map(|n| n.start + n.duration)
                .fold(4.0, f64::max);
            let length = if length + 1e-8 < actual {
                length + 4.0
            } else {
                length
            };
            let instrument = if channel == 9 {
                Instrument::Drums
            } else {
                match programs.get(&channel).copied().unwrap_or(0) {
                    32..=39 => Instrument::Bass,
                    80..=87 => Instrument::Lead,
                    88..=95 => Instrument::Pad,
                    _ => Instrument::Keys,
                }
            };
            project.tracks.push(Track {
                id: new_id(),
                name: if count > 1 {
                    format!(
                        "{} / {}",
                        if name.is_empty() { "MIDI" } else { &name },
                        channel + 1
                    )
                } else if name.is_empty() {
                    "MIDI".into()
                } else {
                    name.clone()
                },
                color: [
                    [165, 144, 230],
                    [83, 180, 181],
                    [223, 161, 91],
                    [130, 176, 235],
                    [207, 119, 151],
                ][project.tracks.len() % 5],
                instrument,
                gain: 0.65,
                pan: 0.0,
                mute: false,
                solo: false,
                patch: SynthPatch::default(),
                clips: vec![Clip {
                    id: new_id(),
                    name: "Imported phrase".into(),
                    start: 0.0,
                    length,
                    notes: channel_notes,
                }],
            });
        }
    }
    project.tempo = 60_000_000.0 / tempo_us.unwrap_or(500_000) as f64;
    project.length_bars = ((max_tick as f64 / tpq / 4.0).ceil() as u32).max(1);
    for t in &project.tracks {
        for c in &t.clips {
            project.length_bars = project.length_bars.max((c.length / 4.0).ceil() as u32);
        }
    }
    validate(&project)?;
    Ok(project)
}
