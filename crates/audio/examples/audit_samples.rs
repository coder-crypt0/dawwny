//! Audit an explicitly supplied SF2 bank and write a short four-family audition session.
use anyhow::{Result, ensure};
use dawwny_audio::{Renderer, compile, load_sample_bank};
use dawwny_core::{Clip, Instrument, Note, Project, SampleInstrument, SynthPatch, Track};
use std::path::PathBuf;
fn main() -> Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("Pass an absolute .sf2 path"));
    let bank = load_sample_bank(&path)?;
    let mut peaks = Vec::new();
    for preset in &bank.presets {
        let percussion = preset.bank == 128;
        let pitches = if percussion {
            [35, 38, 42]
        } else {
            [48, 60, 72]
        };
        let project = Project {
            name: "Sample audit".into(),
            tempo: 120.0,
            length_bars: 1,
            tracks: vec![Track {
                id: "t".into(),
                name: preset.name.clone(),
                color: [145, 172, 215],
                instrument: Instrument::Sampler,
                gain: 0.6,
                pan: 0.0,
                mute: false,
                solo: false,
                patch: SynthPatch {
                    reverb: 0.0,
                    delay: 0.0,
                    sample: Some(SampleInstrument {
                        file: bank.file.to_string_lossy().into_owned(),
                        bank: preset.bank,
                        program: preset.program,
                    }),
                    ..Default::default()
                },
                clips: vec![Clip {
                    id: "c".into(),
                    name: "Audit".into(),
                    start: 0.0,
                    length: 4.0,
                    notes: pitches
                        .into_iter()
                        .enumerate()
                        .map(|(i, pitch)| Note {
                            id: format!("n{i}"),
                            pitch,
                            start: i as f64 * 0.5,
                            duration: 0.4,
                            velocity: 0.75,
                        })
                        .collect(),
                }],
            }],
            ..Default::default()
        };
        let mut renderer = Renderer::new(compile(&project, 16000)?);
        let mut peak = 0.0_f32;
        let mut energy = 0.0;
        for _ in 0..24000 {
            for sample in renderer.next_frame() {
                ensure!(sample.is_finite(), "Non-finite preset {}", preset.name);
                peak = peak.max(sample.abs());
                energy += sample * sample;
            }
        }
        if energy < 1e-6 {
            println!(
                "SILENT at audit pitches: {} ({}:{})",
                preset.name, preset.bank, preset.program
            );
        }
        peaks.push(peak);
    }
    println!(
        "PASS: {} presets render finite samples; bank estimate {:.1} MiB; non-silent {}; peak {:.4}",
        bank.presets.len(),
        bank.storage_bytes as f64 / 1048576.0,
        peaks.iter().filter(|p| **p > 0.001).count(),
        peaks.into_iter().fold(0.0_f32, f32::max)
    );
    Ok(())
}
