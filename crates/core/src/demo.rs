use crate::*;

fn note(id: String, pitch: u8, start: f64, duration: f64, velocity: f32) -> Note {
    Note {
        id,
        pitch,
        start,
        duration,
        velocity,
    }
}

/// A small, original arrangement: Dm9 – Bbmaj7 – Fmaj7 – Cadd9.
pub fn demo_project() -> Project {
    let mut p = Project {
        id: "velvet-dawn".into(),
        name: "Velvet Dawn".into(),
        ..Default::default()
    };
    p.sections = vec![
        Section {
            name: "INTRO".into(),
            start_bar: 0,
            length_bars: 4,
        },
        Section {
            name: "POCKET".into(),
            start_bar: 4,
            length_bars: 4,
        },
        Section {
            name: "LIFT".into(),
            start_bar: 8,
            length_bars: 4,
        },
        Section {
            name: "AFTERGLOW".into(),
            start_bar: 12,
            length_bars: 4,
        },
    ];
    let chords = [
        [62, 65, 69, 72],
        [58, 62, 65, 69],
        [60, 65, 69, 72],
        [60, 64, 67, 74],
    ];
    let roots = [38, 34, 41, 36];
    let specs = [
        ("Felt keys", Instrument::Keys, [168, 143, 223], 0.52),
        ("Cloud pad", Instrument::Pad, [85, 173, 174], 0.32),
        ("Round bass", Instrument::Bass, [212, 163, 97], 0.70),
        ("Pocket kit", Instrument::Drums, [124, 167, 212], 0.68),
        ("Dust hats", Instrument::Drums, [161, 191, 121], 0.32),
        ("Glass notes", Instrument::Lead, [209, 124, 154], 0.35),
    ];
    for (ti, (name, instrument, color, gain)) in specs.into_iter().enumerate() {
        let mut patch = SynthPatch::default();
        match instrument {
            Instrument::Pad => {
                patch.attack = 0.7;
                patch.release = 1.8;
                patch.cutoff = 2400.0;
                patch.reverb = 0.5;
            }
            Instrument::Bass => {
                patch.cutoff = 1300.0;
                patch.sustain = 0.7;
                patch.reverb = 0.0;
                patch.release = 0.13;
            }
            Instrument::Lead => {
                patch.cutoff = 5800.0;
                patch.delay = 0.28;
                patch.reverb = 0.3;
            }
            Instrument::Drums => {
                patch.reverb = 0.07;
                patch.release = 0.1;
            }
            _ => {
                patch.cutoff = 5200.0;
                patch.reverb = 0.25;
                patch.sustain = 0.24;
                patch.decay = 0.7;
            }
        }
        let mut track = Track {
            id: format!("track-{ti}"),
            name: name.into(),
            color,
            instrument,
            gain,
            pan: if ti == 0 {
                -0.12
            } else if ti == 5 {
                0.22
            } else {
                0.0
            },
            mute: false,
            solo: false,
            patch,
            clips: vec![],
        };
        for section in 0..4 {
            if (ti == 2 || ti == 3 || ti == 4) && section == 0 {
                continue;
            }
            if ti == 5 && section < 2 {
                continue;
            }
            let mut clip = Clip {
                id: format!("clip-{ti}-{section}"),
                name: match ti {
                    0 => "Dm9 · Bb · F · C",
                    1 => "Slow bloom",
                    2 => "Root motion",
                    3 => "Backbeat",
                    4 => "Swung eighths",
                    _ => "Answer phrase",
                }
                .into(),
                start: section as f64 * 16.0,
                length: 16.0,
                notes: vec![],
            };
            let mut add = |pitch, start, duration, velocity| {
                let id = format!("note-{ti}-{section}-{}", clip.notes.len());
                clip.notes.push(note(id, pitch, start, duration, velocity));
            };
            for bar in 0..4 {
                let b = bar as f64 * 4.0;
                match ti {
                    0 => {
                        for (k, &pitch) in chords[bar].iter().enumerate() {
                            add(pitch, b + k as f64 * 0.012, 1.55, 0.60 - k as f32 * 0.025);
                            add(pitch, b + 2.52 + k as f64 * 0.008, 1.25, 0.42);
                        }
                    }
                    1 => {
                        for &pitch in &chords[bar][..3] {
                            add(pitch - 12, b, 3.8, 0.46);
                        }
                    }
                    2 => {
                        add(roots[bar], b, 1.65, 0.79);
                        add(roots[bar], b + 2.0, 0.62, 0.57);
                        add(roots[bar] + 12, b + 3.25, 0.5, 0.48);
                    }
                    3 => {
                        add(36, b, 0.2, 0.88);
                        add(38, b + 1.012, 0.18, 0.73);
                        add(36, b + 2.5, 0.2, 0.66);
                        add(38, b + 3.018, 0.18, 0.78);
                        if bar == 3 {
                            add(38, b + 3.75, 0.12, 0.28);
                        }
                    }
                    4 => {
                        for h in 0..8 {
                            add(
                                if h == 7 && bar % 2 == 1 { 46 } else { 42 },
                                b + h as f64 * 0.5 + if h % 2 == 1 { 0.045 } else { 0.0 },
                                0.12,
                                if h % 2 == 0 { 0.56 } else { 0.31 },
                            );
                        }
                    }
                    _ => {
                        let melody = [[74, 77, 76], [74, 72, 69], [72, 76, 77], [74, 71, 69]][bar];
                        for (j, pitch) in melody.into_iter().enumerate() {
                            add(pitch, b + 0.5 + j as f64, 0.68, 0.48 + j as f32 * 0.035);
                        }
                    }
                }
            }
            track.clips.push(clip);
        }
        p.tracks.push(track);
    }
    p
}
