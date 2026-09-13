use eframe::egui;

#[derive(Default)]
struct Studio {
    project: dawwny_core::Project,
}

impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("dawwny");
            ui.label("Native studio · foundation");
            ui.separator();
            ui.text_edit_singleline(&mut self.project.name);
            ui.add(egui::Slider::new(&mut self.project.tempo, 30.0..=300.0).text("BPM"));
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "dawwny",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<Studio>::default())),
    )
}
