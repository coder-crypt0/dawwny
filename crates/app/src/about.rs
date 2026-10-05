use crate::studio::Studio;
use eframe::egui;

const NOTICES: &[u8] = include_bytes!("../assets/licenses.zip");
const LICENSE: &str = include_str!("../../../LICENSE");

impl Studio {
    pub fn about_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_about;
        egui::Window::new("About dawwny")
            .open(&mut open)
            .default_width(480.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading(format!("dawwny {}", env!("CARGO_PKG_VERSION")));
                ui.label("A native music studio that your agents can play.");
                ui.add_space(8.0);
                ui.hyperlink_to("Source & releases", "https://github.com/coder-crypt0/dawwny");
                ui.label("The studio, synthesizers and local MCP server are included in this executable.");
                ui.separator();
                egui::CollapsingHeader::new("MIT license").show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| { ui.label(LICENSE); });
                });
                ui.label("Third-party licenses, font notices and dependency credits are embedded in the app.");
                if ui.button("Save third-party license notices…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("License archive", &["zip"])
                        .set_file_name("dawwny-licenses.zip")
                        .save_file()
                {
                    match std::fs::write(path, NOTICES) {
                        Ok(()) => self.status = "License notices saved".into(),
                        Err(error) => self.fail(format!("Could not save license notices: {error}")),
                    }
                }
            });
        self.show_about = open;
    }
}
