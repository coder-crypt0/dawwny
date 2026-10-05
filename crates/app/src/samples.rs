use crate::studio::{EditorTab, Studio};
use dawwny_audio::{SampleBank, load_sample_bank};
use dawwny_core::{Instrument, SampleInstrument};
use eframe::egui::{self, Color32};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
};

#[derive(Default)]
pub struct SampleLibrary {
    pub open: bool,
    pub bank: Option<Arc<SampleBank>>,
    query: String,
    selected: Option<(u16, u8)>,
    requested: Option<PathBuf>,
    observed: Option<SampleInstrument>,
    job: Option<mpsc::Receiver<Result<Arc<SampleBank>, String>>>,
    error: Option<String>,
}
impl SampleLibrary {
    pub fn request(&mut self, file: PathBuf) {
        self.requested = Some(file.clone());
        self.error = None;
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(load_sample_bank(&file).map_err(|e| format!("{e:#}")));
        });
    }
    pub fn poll(&mut self) {
        let Some(job) = &self.job else {
            return;
        };
        match job.try_recv() {
            Ok(Ok(bank)) => {
                self.selected = self
                    .observed
                    .as_ref()
                    .filter(|s| std::path::Path::new(&s.file) == bank.file)
                    .map(|s| (s.bank, s.program))
                    .or_else(|| bank.presets.first().map(|p| (p.bank, p.program)));
                self.bank = Some(bank);
                self.query.clear();
                self.job = None;
            }
            Ok(Err(e)) => {
                self.error = Some(e);
                self.job = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.error = Some("Sample loader stopped unexpectedly".into());
                self.job = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    pub fn loading(&self) -> bool {
        self.job.is_some()
    }
}
impl Studio {
    pub fn sample_housekeeping(&mut self) {
        self.samples.poll();
        let sample = self
            .project
            .tracks
            .get(self.selected_track)
            .filter(|t| t.instrument == Instrument::Sampler)
            .and_then(|t| t.patch.sample.as_ref())
            .cloned();
        if sample != self.samples.observed {
            self.samples.observed = sample.clone();
            if let Some(sample) = sample {
                self.samples.selected = Some((sample.bank, sample.program));
                let path = PathBuf::from(&sample.file);
                if self.samples.requested.as_ref() != Some(&path) {
                    self.samples.request(path);
                }
            }
        }
    }
    pub fn load_sample(&mut self, bank: Arc<SampleBank>, preset: (u16, u8)) {
        if !bank.presets.iter().any(|p| (p.bank, p.program) == preset) {
            self.fail("Choose an existing sample preset".into());
            return;
        }
        if self.project.tracks.is_empty() {
            self.add_track(Instrument::Synth);
        }
        let ti = self.selected_track;
        let sample = SampleInstrument {
            file: bank.file.to_string_lossy().into_owned(),
            bank: preset.0,
            program: preset.1,
        };
        self.edit(|p| {
            p.tracks[ti].instrument = Instrument::Sampler;
            p.tracks[ti].patch.sample = Some(sample);
        });
        self.tab = EditorTab::Sound;
    }
    pub fn sample_window(&mut self, ctx: &egui::Context) {
        if !self.samples.open {
            return;
        }
        let mut open = true;
        let mut import = false;
        let mut use_preset = None;
        let mut play_keys = false;
        egui::Window::new("Sample instruments").open(&mut open).default_width(570.0).default_height(510.0).min_width(440.0).resizable(true).show(ctx,|ui| {
            ui.horizontal(|ui| {
                if ui.button("Import SoundFont…").on_hover_text("Choose a local SF2 bank. Samples stay outside the executable.").clicked() {import=true;}
                if ui.button("Musical typing").clicked() {play_keys=true;}
                if self.samples.loading() {ui.spinner();ui.label("Loading samples…");}
            });
            if let Some(e)=&self.samples.error {ui.colored_label(Color32::from_rgb(235,145,130),e);}
            let Some(bank)=&self.samples.bank else {
                ui.add_space(28.0);ui.heading("Bring your instruments");
                ui.label("Import an SF2 bank for sampled piano, orchestral instruments, guitars, drums, and more. Choose a preset, use it on a track, then play with your keyboard or MIDI controller.");
                ui.add_space(12.0);ui.label("The sound quality and articulations come from the bank you import.");return;
            };
            ui.add_space(8.0);ui.heading(&bank.name);
            ui.label(egui::RichText::new(format!("{} presets · {:.1} MiB shared samples",bank.presets.len(),bank.storage_bytes as f64/1048576.0)).small().weak());
            ui.label(egui::RichText::new(bank.file.file_name().unwrap_or_default().to_string_lossy()).small().weak()).on_hover_text(bank.file.display().to_string());
            ui.add(egui::TextEdit::singleline(&mut self.samples.query).hint_text("Find piano, strings, guitar, drum…").desired_width(f32::INFINITY));
            let query=self.samples.query.to_lowercase();
            let rows:Vec<_>=bank.presets.iter().filter(|p|query.split_whitespace().all(|term|p.name.to_lowercase().contains(term))).collect();
            ui.add_space(5.0);
            egui::ScrollArea::vertical().id_salt("sample_presets").max_height(330.0).show_rows(ui,30.0,rows.len(),|ui,range|{
                for i in range {
                    let p=rows[i];let id=(p.bank,p.program);
                    ui.horizontal(|ui| {
                        let r=ui.selectable_label(self.samples.selected==Some(id),&p.name);
                        if r.clicked(){self.samples.selected=Some(id);}
                        if r.double_clicked(){use_preset=Some(id);}
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ui.label(egui::RichText::new(format!("{} / {:03}",if p.bank==128 {"Kit".into()}else{format!("Bank {}",p.bank)},p.program)).small().weak());});
                    });
                }
            });
            if rows.is_empty(){ui.label("No presets match this search.");}
            ui.separator();
            let track=self.project.tracks.get(self.selected_track).map_or("a new track",|t|t.name.as_str());
            if ui.add_enabled(!self.samples.loading() && self.samples.selected.is_some(),egui::Button::new(format!("Use on {track}"))).clicked(){use_preset=self.samples.selected;}
            ui.label(egui::RichText::new("Preserves notes, mix and effects. Double-click a preset to use it.").small().weak());
        });
        self.samples.open = open;
        if play_keys {
            self.keyboard.open = true;
        }
        if import
            && let Some(file) = rfd::FileDialog::new()
                .add_filter("SoundFont sample bank", &["sf2"])
                .pick_file()
        {
            self.samples.request(file);
        }
        if let Some(preset) = use_preset
            && let Some(bank) = self.samples.bank.clone()
        {
            self.load_sample(bank, preset);
        }
    }
}

#[cfg(test)]
#[path = "../../audio/tests/support/mod.rs"]
mod support;
#[cfg(test)]
mod tests {

    use super::support;
    use super::*;
    #[test]
    fn importing_and_selecting_a_sample_preserves_the_composition_and_undo() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.sf2");
        support::bank(&file);
        let bank = load_sample_bank(&file).unwrap();
        let mut studio = Studio::new(dir.path().join("session.json"), true).unwrap();
        studio.new_document(support::project(&file));
        let before = studio.project.clone();
        studio.samples.bank = Some(bank.clone());
        studio.samples.open = true;
        studio.load_sample(bank, (0, 24));
        assert_eq!(studio.project.tracks[0].clips, before.tracks[0].clips);
        assert_eq!(
            studio.project.tracks[0]
                .patch
                .sample
                .as_ref()
                .unwrap()
                .program,
            24
        );
        assert!(studio.commit());
        studio.history(false);
        assert_eq!(studio.project.tracks, before.tracks);
        let ctx = egui::Context::default();
        crate::views::theme(&ctx);
        let output = ctx.run(egui::RawInput::default(), |ctx| studio.sample_window(ctx));
        assert!(!output.shapes.is_empty());
    }
}
