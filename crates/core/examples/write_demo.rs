fn main() -> anyhow::Result<()> {
    dawwny_core::save_project(
        std::path::Path::new("examples/demo.dawwny.json"),
        &dawwny_core::demo_project(),
    )
}
