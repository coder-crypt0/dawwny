fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or("exports/velvet-dawn.wav".into());
    let start = std::time::Instant::now();
    let stats = dawwny_audio::render_wav(
        &dawwny_core::demo_project(),
        std::path::Path::new(&path),
        48_000,
    )?;
    println!(
        "frames={} peak={:.6} stolen_voices={} render_seconds={:.3}",
        stats.frames,
        stats.peak,
        stats.stolen_voices,
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
