mod studio;
mod views;

use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut path = PathBuf::from("sessions/untitled.dawwny.json");
    let mut no_audio = false;
    let mut view = studio::EditorTab::Piano;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => {
                path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--project requires a file path"))?
                    .into()
            }
            "--no-audio" => no_audio = true,
            "--view" => {
                view = match args.next().as_deref() {
                    Some("piano") => studio::EditorTab::Piano,
                    Some("sound") => studio::EditorTab::Sound,
                    Some("mixer") => studio::EditorTab::Mixer,
                    _ => anyhow::bail!("--view must be piano, sound, or mixer"),
                }
            }
            "--help" | "-h" => {
                println!(
                    "dawwny [--project FILE] [--no-audio]\nNative studio. The local MCP server can open the same project file."
                );
                return Ok(());
            }
            _ => anyhow::bail!("Unknown option: {arg}"),
        }
    }
    let mut studio = studio::Studio::new(path, no_audio)?;
    studio.tab = view;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([940.0, 580.0])
            .with_maximized(true)
            .with_icon(std::sync::Arc::new(studio_icon()))
            .with_title("dawwny — native studio"),
        renderer: eframe::Renderer::Glow,
        vsync: true,
        ..Default::default()
    };
    eframe::run_native(
        "dawwny",
        options,
        Box::new(|cc| {
            views::theme(&cc.egui_ctx);
            Ok(Box::new(studio))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Unable to open native window: {e}"))
}

fn studio_icon() -> eframe::egui::IconData {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let lit = [(5, 7), (10, 13), (15, 20), (20, 12), (25, 6)]
                .iter()
                .any(|&(cx, h)| x >= cx && x < cx + 2 && y >= (32 - h) / 2 && y < (32 + h) / 2);
            rgba.extend_from_slice(if lit {
                &[203, 231, 143, 255]
            } else {
                &[21, 23, 27, 255]
            });
        }
    }
    eframe::egui::IconData {
        rgba,
        width: 32,
        height: 32,
    }
}
