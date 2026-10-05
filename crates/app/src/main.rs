#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod about;
mod cli;
mod design;
mod keyboard;
mod library;
mod recording;
mod samples;
mod sections;
mod sound_editor;
mod studio;
mod views;

fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let headless = args.iter().any(|arg| arg == "--mcp");
    if let Err(error) = run(args) {
        eprintln!("dawwny: {error:#}");
        if !headless {
            rfd::MessageDialog::new()
                .set_title("Unable to open dawwny")
                .set_description(format!("{error:#}"))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn run(args: Vec<std::ffi::OsString>) -> anyhow::Result<()> {
    let options = match cli::parse(args)? {
        cli::Launch::Mcp { project, exports } => return dawwny_mcp::serve_stdio(project, exports),
        cli::Launch::Help => {
            println!(
                "dawwny [--project FILE] [--no-audio] [--view piano|sound|mixer] [--expanded] [--soundfont FILE]\ndawwny --mcp [--project FILE] [--export-dir DIRECTORY]\nOne native executable for the studio and local stdio MCP. Default data: the user's application data directory."
            );
            return Ok(());
        }
        cli::Launch::Studio(options) => options,
    };
    let mut studio = studio::Studio::new(options.project, options.no_audio)?;
    studio.tab = options.view;
    studio.editor_expanded = options.expanded;
    if let Some(file) = options.soundfont {
        studio.samples.open = true;
        studio.samples.request(file.canonicalize()?);
    }
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
