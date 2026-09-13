use crate::*;
use anyhow::{Result, ensure};
use std::collections::HashSet;

pub const MAX_NOTES: usize = 32_768;

fn text(value: &str, label: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= 128 && !value.chars().any(char::is_control),
        "{label} must be 1–128 bytes without control characters"
    );
    Ok(())
}
fn range(value: f32, min: f32, max: f32, label: &str) -> Result<()> {
    ensure!(
        value.is_finite() && (min..=max).contains(&value),
        "{label} must be finite and between {min} and {max}"
    );
    Ok(())
}
fn id<'a>(value: &'a str, ids: &mut HashSet<&'a str>) -> Result<()> {
    text(value, "ID")?;
    ensure!(ids.insert(value), "Duplicate ID: {value}");
    Ok(())
}
pub fn validate(p: &Project) -> Result<()> {
    ensure!(
        p.schema_version == 1,
        "Unsupported project schema version {}",
        p.schema_version
    );
    text(&p.id, "Project ID")?;
    text(&p.name, "Project name")?;
    ensure!(
        p.tempo.is_finite() && (30.0..=300.0).contains(&p.tempo),
        "Tempo must be 30–300 BPM"
    );
    ensure!(
        (1..=256).contains(&p.length_bars),
        "Project length must be 1–256 bars"
    );
    range(p.master_gain, 0.0, 1.0, "Master gain")?;
    ensure!(
        p.tracks.len() <= 32,
        "A session can contain at most 32 tracks"
    );
    ensure!(
        p.sections.len() <= 128,
        "At most 128 arrangement sections are supported"
    );
    for s in &p.sections {
        text(&s.name, "Section name")?;
        ensure!(
            s.length_bars > 0
                && s.start_bar
                    .checked_add(s.length_bars)
                    .is_some_and(|end| end <= p.length_bars),
            "Section extends beyond project end"
        );
    }
    let mut ids = HashSet::new();
    let mut notes = 0usize;
    let mut clips = 0usize;
    for t in &p.tracks {
        id(&t.id, &mut ids)?;
        text(&t.name, "Track name")?;
        range(t.gain, 0.0, 1.0, "Track gain")?;
        range(t.pan, -1.0, 1.0, "Pan")?;
        let s = &t.patch;
        range(s.attack, 0.001, 5.0, "Attack")?;
        range(s.decay, 0.001, 5.0, "Decay")?;
        range(s.sustain, 0.0, 1.0, "Sustain")?;
        range(s.release, 0.001, 5.0, "Release")?;
        range(s.cutoff, 20.0, 20_000.0, "Filter cutoff")?;
        range(s.reverb, 0.0, 1.0, "Reverb")?;
        range(s.delay, 0.0, 1.0, "Delay")?;
        clips += t.clips.len();
        ensure!(clips <= 128, "At most 128 clips per session");
        for c in &t.clips {
            id(&c.id, &mut ids)?;
            text(&c.name, "Clip name")?;
            ensure!(
                c.start.is_finite()
                    && c.length.is_finite()
                    && c.start >= 0.0
                    && c.length > 0.0
                    && c.start + c.length <= p.length_bars as f64 * 4.0 + 1e-8,
                "Clip must lie inside the project timeline"
            );
            notes += c.notes.len();
            ensure!(notes <= MAX_NOTES, "At most {MAX_NOTES} notes per session");
            for n in &c.notes {
                id(&n.id, &mut ids)?;
                ensure!(n.pitch <= 127, "MIDI pitch must be 0–127");
                ensure!(
                    n.start.is_finite()
                        && n.duration.is_finite()
                        && n.start >= 0.0
                        && n.duration >= 1.0 / 960.0
                        && n.start + n.duration <= c.length + 1e-8,
                    "Note must fit inside its clip and last at least 1/960 beat"
                );
                range(n.velocity, 0.0, 1.0, "Velocity")?;
            }
        }
    }
    Ok(())
}
