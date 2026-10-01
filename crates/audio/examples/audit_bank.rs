//! Deterministic factory-bank smoke audit. Does not certify subjective sound quality.
use anyhow::{Result, ensure};
use dawwny_audio::{Renderer, compile};
use dawwny_core::*;
use std::{collections::HashMap, io::Write};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let full = args.iter().any(|a| a == "--full");
    let rate = if full { 48000 } else { 16000 };
    let file = if full {
        "artifacts/bank-audit-48k.csv"
    } else {
        "artifacts/bank-audit-16k.csv"
    };
    std::fs::create_dir_all("artifacts")?;
    let mut output = std::io::BufWriter::new(std::fs::File::create(file)?);
    writeln!(
        output,
        "id,category,pitch,sample_rate,frames,peak,rms,dc,tail_rms,quantized_hash"
    )?;
    let mut hashes = HashMap::new();
    let mut worst_peak = 0.0_f32;
    let mut lowest_rms = f64::INFINITY;
    let mut max_dc = 0.0_f64;
    let start = std::time::Instant::now();
    for (index, info) in sound_catalog().iter().enumerate() {
        let preset = sound_preset(&info.id).unwrap();
        let project = Project {
            tempo: 120.0,
            length_bars: 2,
            master_gain: 0.7,
            tracks: vec![Track {
                id: "audit-track".into(),
                name: "Audit".into(),
                color: [0, 0, 0],
                instrument: Instrument::Synth,
                gain: 0.7,
                pan: 0.0,
                mute: false,
                solo: false,
                patch: preset.patch,
                clips: vec![Clip {
                    id: "audit-clip".into(),
                    name: "Audit".into(),
                    start: 0.0,
                    length: 8.0,
                    notes: vec![Note {
                        id: "audit-note".into(),
                        pitch: info.audition_pitch,
                        start: 0.0,
                        duration: 6.0,
                        velocity: 0.8,
                    }],
                }],
            }],
            ..Default::default()
        };
        let mut renderer = Renderer::new(compile(&project, rate)?);
        let frames = if full {
            renderer.total_frames()
        } else {
            (rate * 6) as u64
        };
        let mut peak = 0.0_f32;
        let mut energy = 0.0_f64;
        let mut sum = 0.0_f64;
        let mut tail = 0.0_f64;
        let mut tail_samples = 0;
        let mut hash = 0xcbf29ce484222325_u64;
        for frame in 0..frames {
            for sample in renderer.next_frame() {
                ensure!(
                    sample.is_finite(),
                    "{} produced non-finite samples",
                    info.id
                );
                peak = peak.max(sample.abs());
                energy += (sample as f64).powi(2);
                sum += sample as f64;
                if frame >= rate as u64 * 3 {
                    tail += (sample as f64).powi(2);
                    tail_samples += 1;
                }
                let quantized = (sample * 100000.0).round() as i32;
                for byte in quantized.to_le_bytes() {
                    hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
                }
            }
        }
        let rms = (energy / (frames * 2) as f64).sqrt();
        let dc = sum / (frames * 2) as f64;
        let tail_rms = (tail / (tail_samples.max(1)) as f64).sqrt();
        ensure!(rms > 0.00001, "{} is silent or nearly silent", info.id);
        ensure!(
            peak < 0.94,
            "{} is pinned near the master limiter: {peak}",
            info.id
        );
        ensure!(dc.abs() < 0.01, "{} has excessive DC: {dc}", info.id);
        if let Some(other) = hashes.insert(hash, info.id.clone()) {
            anyhow::bail!("Identical quantized audio: {} and {other}", info.id);
        }
        worst_peak = worst_peak.max(peak);
        lowest_rms = lowest_rms.min(rms);
        max_dc = max_dc.max(dc.abs());
        writeln!(
            output,
            "{},{},{},{rate},{frames},{peak:.7},{rms:.7},{dc:.7},{tail_rms:.7},{hash:016x}",
            info.id, info.category, info.audition_pitch
        )?;
        if (index + 1) % 128 == 0 {
            eprintln!("audited {}/{}", index + 1, sound_catalog().len());
        }
    }
    output.flush()?;
    println!(
        "PASS {} presets at {} Hz; highest peak={:.6}, lowest RMS={:.6}, max DC={:.6}; elapsed={:.2}s; {}",
        sound_catalog().len(),
        rate,
        worst_peak,
        lowest_rms,
        max_dc,
        start.elapsed().as_secs_f64(),
        file
    );
    Ok(())
}
