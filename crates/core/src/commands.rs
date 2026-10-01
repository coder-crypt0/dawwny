use crate::*;
use anyhow::{Context, Result, bail};

fn track_mut<'a>(p: &'a mut Project, id: &str) -> Result<&'a mut Track> {
    p.tracks
        .iter_mut()
        .find(|t| t.id == id)
        .context("track not found")
}
fn clip_mut<'a>(t: &'a mut Track, id: &str) -> Result<&'a mut Clip> {
    t.clips
        .iter_mut()
        .find(|c| c.id == id)
        .context("clip not found")
}
pub fn apply_commands(p: &Project, commands: &[Command]) -> Result<Project> {
    if commands.is_empty() || commands.len() > 256 {
        bail!("A transaction needs 1–256 commands");
    }
    let mut n = p.clone();
    for c in commands {
        match c {
            Command::SetTempo { tempo } => n.tempo = *tempo,
            Command::RenameProject { name } => n.name = name.clone(),
            Command::SetLength { length_bars } => n.length_bars = *length_bars,
            Command::SetMasterGain { gain } => n.master_gain = *gain,
            Command::AddTrack { track } => n.tracks.push(track.clone()),
            Command::RemoveTrack { track_id } => n.tracks.retain(|t| t.id != *track_id),
            Command::ApplySoundPreset {
                track_id,
                preset_id,
            } => {
                let preset = sound_preset(preset_id)
                    .context("Unknown sound preset; use list_sounds to discover available IDs")?;
                let track = track_mut(&mut n, track_id)?;
                track.instrument = Instrument::Synth;
                track.patch = preset.patch;
            }
            Command::UpdateTrack {
                track_id,
                name,
                gain,
                pan,
                mute,
                solo,
                instrument,
                patch,
            } => {
                let t = track_mut(&mut n, track_id)?;
                if let Some(v) = name {
                    t.name = v.clone()
                }
                if let Some(v) = gain {
                    t.gain = *v
                }
                if let Some(v) = pan {
                    t.pan = *v
                }
                if let Some(v) = mute {
                    t.mute = *v
                }
                if let Some(v) = solo {
                    t.solo = *v
                }
                if let Some(v) = instrument {
                    t.instrument = *v
                }
                if let Some(v) = patch {
                    t.patch = v.clone()
                }
            }
            Command::AddClip { track_id, clip } => {
                track_mut(&mut n, track_id)?.clips.push(clip.clone())
            }
            Command::RemoveClip { track_id, clip_id } => track_mut(&mut n, track_id)?
                .clips
                .retain(|c| c.id != *clip_id),
            Command::SetClip { track_id, clip } => {
                let t = track_mut(&mut n, track_id)?;
                if let Some(x) = t.clips.iter_mut().find(|x| x.id == clip.id) {
                    *x = clip.clone()
                } else {
                    t.clips.push(clip.clone())
                }
            }
            Command::AddNotes {
                track_id,
                clip_id,
                notes,
            } => clip_mut(track_mut(&mut n, track_id)?, clip_id)?
                .notes
                .extend(notes.clone()),
            Command::RemoveNote {
                track_id,
                clip_id,
                note_id,
            } => clip_mut(track_mut(&mut n, track_id)?, clip_id)?
                .notes
                .retain(|x| x.id != *note_id),
        }
    }
    validate(&n)?;
    n.revision = p
        .revision
        .checked_add(1)
        .context("Revision limit reached")?;
    Ok(n)
}
