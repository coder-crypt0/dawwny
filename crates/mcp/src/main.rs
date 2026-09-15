use rmcp::{ServiceExt, transport::stdio};
use std::path::PathBuf;
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let mut project = PathBuf::from("sessions/untitled.dawwny.json");
    let mut export_dir = PathBuf::from("exports");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => {
                project = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--project requires a file path"))?
                    .into()
            }
            "--export-dir" => {
                export_dir = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--export-dir requires a directory"))?
                    .into()
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
    dawwny_core::SessionStore::new(project.clone()).initialize(&dawwny_core::demo_project())?;
    dawwny_mcp::build_server(project, export_dir)
        .serve(stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
