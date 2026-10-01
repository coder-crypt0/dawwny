fn main() -> anyhow::Result<()> {
    let mut project = dawwny_core::demo_project();
    project.name = "Dawn collection".into();
    for (index, id) in [
        (0, "amber_keys"),
        (1, "factory.dusk-wash.10"),
        (2, "copper_bass"),
        (5, "glass_orbit"),
    ] {
        let preset = dawwny_core::sound_preset(id).unwrap();
        project.tracks[index].instrument = dawwny_core::Instrument::Synth;
        project.tracks[index].name = preset.name;
        project.tracks[index].patch = preset.patch;
    }
    let path = std::env::args()
        .nth(1)
        .unwrap_or("examples/dawn-showcase.dawwny.json".into());
    dawwny_core::save_project(std::path::Path::new(&path), &project)?;
    println!("Wrote {path}");
    Ok(())
}
