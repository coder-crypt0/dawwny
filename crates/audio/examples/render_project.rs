use anyhow::{Context, Result, ensure};
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() == 4,
        "Usage: render_project <project.dawwny.json> <output.wav> <output.mid>"
    );
    let project = dawwny_core::load_project(Path::new(&args[1]))
        .with_context(|| format!("Could not load {}", args[1]))?;
    dawwny_core::export_midi(&project, Path::new(&args[3]))?;
    let start = std::time::Instant::now();
    let stats = dawwny_audio::render_wav(&project, Path::new(&args[2]), 48_000)?;
    println!(
        "Rendered {} bars at {} BPM; frames={}, peak={:.5}, stolen_voices={}, seconds={:.2}",
        project.length_bars,
        project.tempo,
        stats.frames,
        stats.peak,
        stats.stolen_voices,
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
