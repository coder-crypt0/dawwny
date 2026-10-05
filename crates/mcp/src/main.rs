use std::path::PathBuf;
fn main() -> anyhow::Result<()> {
    let mut project: Option<PathBuf> = None;
    let mut export_dir: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => {
                project = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--project requires a file path"))?
                        .into(),
                )
            }
            "--export-dir" => {
                export_dir = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--export-dir requires a directory"))?
                        .into(),
                )
            }
            "--help" | "-h" => {
                eprintln!(
                    "dawwny-mcp [--project FILE] [--export-dir DIRECTORY]\nLocal stdio MCP. stdout is reserved for JSON-RPC."
                );
                return Ok(());
            }
            _ => anyhow::bail!("Unknown option: {arg}"),
        }
    }
    let project = match project {
        Some(path) => path,
        None => dawwny_core::default_data_directory()?.join("sessions/untitled.dawwny.json"),
    };
    let export_dir = match export_dir {
        Some(path) => path,
        None => dawwny_core::default_data_directory()?.join("exports"),
    };
    dawwny_mcp::serve_stdio(project, export_dir)
}
