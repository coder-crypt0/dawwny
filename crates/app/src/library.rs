use crate::studio::{EditorTab, Studio};
use dawwny_core::{Clip, Instrument, Note, Project, SoundInfo, Track};
use eframe::egui::{self, Color32, Vec2};
use std::{collections::BTreeSet, io::Write, path::PathBuf};

pub struct LibraryState {
    pub query: String,
    pub category: String,
    pub only_favorites: bool,
    pub favorites: BTreeSet<String>,
    pub selected: Option<String>,
    pub filtered: Vec<usize>,
    pub error: Option<String>,
    file: PathBuf,
}
impl LibraryState {
    pub fn new(file: PathBuf) -> Self {
        let (favorites, error) = if file.exists() {
            match std::fs::metadata(&file).and_then(|m| {
                if m.len() > 1024 * 1024 {
                    Err(std::io::Error::other("Favorites file is too large"))
                } else {
                    std::fs::read(&file)
                }
            }) {
                Ok(bytes) => match serde_json::from_slice::<BTreeSet<String>>(&bytes) {
                    Ok(mut values) => {
                        values
                            .retain(|id| dawwny_core::sound_catalog().iter().any(|s| &s.id == id));
                        (values, None)
                    }
                    Err(e) => (
                        BTreeSet::new(),
                        Some(format!("Could not load favorites: {e}")),
                    ),
                },
                Err(e) => (
                    BTreeSet::new(),
                    Some(format!("Could not load favorites: {e}")),
                ),
            }
        } else {
            (BTreeSet::new(), None)
        };
        let mut state = Self {
            query: String::new(),
            category: String::new(),
            only_favorites: false,
            favorites,
            selected: Some("amber_keys".into()),
            filtered: Vec::new(),
            error,
            file,
        };
        state.filter();
        state
    }
    pub fn filter(&mut self) {
        let query = self.query.to_lowercase();
        let terms: Vec<_> = query.split_whitespace().collect();
        self.filtered = dawwny_core::sound_catalog()
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                if !self.category.is_empty() && s.category != self.category {
                    return false;
                }
                if self.only_favorites && !self.favorites.contains(&s.id) {
                    return false;
                }
                let text = format!(
                    "{} {} {} {}",
                    s.name,
                    s.family,
                    s.category,
                    s.tags.join(" ")
                )
                .to_lowercase();
                terms.iter().all(|term| text.contains(term))
            })
            .map(|(i, _)| i)
            .collect();
    }
    fn favorite(&mut self, id: String) {
        let previous = self.favorites.clone();
        if !self.favorites.remove(&id) {
            self.favorites.insert(id);
        }
        let result = (|| -> anyhow::Result<()> {
            if let Some(parent) = self.file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let parent = self
                .file
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new("."));
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            temporary.write_all(&serde_json::to_vec(&self.favorites)?)?;
            temporary.as_file().sync_all()?;
            temporary.persist(&self.file)?;
            Ok(())
        })();
        if result.is_err() {
            self.favorites = previous;
        }
        self.error = result
            .err()
            .map(|e| format!("Favorites were not saved: {e}"));
        self.filter();
    }
}
fn audition(info: &SoundInfo) -> Project {
    let preset = dawwny_core::sound_preset(&info.id).expect("Catalog preset exists");
    let chord = matches!(
        info.category.as_str(),
        "Pad" | "Keys" | "Organ" | "Brass" | "Strings"
    );
    let notes = if chord { vec![0, 7, 12] } else { vec![0] };
    Project {
        name: format!("Preview {}", info.name),
        tempo: 120.0,
        length_bars: 2,
        master_gain: 0.65,
        tracks: vec![Track {
            id: "preview-track".into(),
            name: preset.name,
            color: [145, 172, 215],
            instrument: Instrument::Synth,
            gain: 0.65,
            pan: 0.0,
            mute: false,
            solo: false,
            patch: preset.patch,
            clips: vec![Clip {
                id: "preview-clip".into(),
                name: "Audition".into(),
                start: 0.0,
                length: 8.0,
                notes: notes
                    .into_iter()
                    .enumerate()
                    .map(|(i, offset)| Note {
                        id: format!("preview-note-{i}"),
                        pitch: info.audition_pitch + offset,
                        start: 0.0,
                        duration: 6.0,
                        velocity: 0.75,
                    })
                    .collect(),
            }],
        }],
        ..Default::default()
    }
}
impl Studio {
    pub fn preview_sound(&mut self, id: &str) {
        let Some(info) = dawwny_core::sound_catalog().iter().find(|s| s.id == id) else {
            return;
        };
        if self.audio.is_none() {
            self.fail(
                "Preview needs an audio device. Editing and WAV export remain available.".into(),
            );
            return;
        }
        self.stop();
        let result = (|| -> anyhow::Result<()> {
            let mut preview = dawwny_audio::AudioEngine::new()?;
            preview.play(&audition(info), false)?;
            self.preview = Some(preview);
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.status = format!("Previewing {}", info.name);
                self.error = false;
            }
            Err(e) => self.fail(format!("Preview failed: {e}")),
        }
    }
    pub fn load_sound(&mut self, id: &str) {
        let Some(preset) = dawwny_core::sound_preset(id) else {
            return;
        };
        if self.project.tracks.is_empty() {
            self.add_track(Instrument::Synth);
        }
        let track = self.selected_track;
        self.edit(|project| {
            project.tracks[track].instrument = Instrument::Synth;
            project.tracks[track].patch = preset.patch;
        });
        self.tab = EditorTab::Sound;
    }
    pub fn library_panel(&mut self, ctx: &egui::Context) {
        let mut preview = None;
        let mut load = None;
        let mut favorite = None;
        egui::SidePanel::left("library").default_width(294.0).width_range(260.0..=370.0).resizable(true)
            .frame(egui::Frame::new().fill(Color32::from_rgb(30,33,40)).corner_radius(16).inner_margin(14).outer_margin(egui::Margin{left:10,right:4,top:4,bottom:8}))
            .show(ctx,|ui| {
                ui.horizontal(|ui|{ui.heading("Sound library");ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if ui.button("×").on_hover_text("Hide library · L").clicked(){self.show_library=false;}});});
                ui.label(egui::RichText::new("2,310 sounds · Dawn collection").small().weak());
                let mut changed=ui.add(egui::TextEdit::singleline(&mut self.library.query).hint_text("Search sounds, textures…").desired_width(f32::INFINITY)).changed();
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("category").width(143.0).selected_text(if self.library.category.is_empty(){"All instruments"}else{&self.library.category}).show_ui(ui,|ui| {
                        changed|=ui.selectable_value(&mut self.library.category,String::new(),"All instruments").changed();
                        for category in dawwny_core::SOUND_CATEGORIES{changed|=ui.selectable_value(&mut self.library.category,(*category).into(),*category).changed();}
                    });
                    changed|=ui.toggle_value(&mut self.library.only_favorites,"★").on_hover_text("Favorites only").changed();
                });
                if changed{self.library.filter();}
                ui.add_space(3.0);
                ui.label(egui::RichText::new(format!("{} results",self.library.filtered.len())).small().weak());
                let selected=self.library.selected.as_ref().and_then(|id|dawwny_core::sound_catalog().iter().find(|s|&s.id==id));
                // Details stay visible below the virtualized list even with thousands of results.
                egui::TopBottomPanel::bottom("sound_details").frame(egui::Frame::new().inner_margin(egui::Margin{top:12,..Default::default()})).show_inside(ui,|ui| {
                    if let Some(info)=selected {
                        ui.label(egui::RichText::new(&info.name).strong());
                        ui.label(egui::RichText::new(&info.description).small().weak());
                        ui.horizontal(|ui| {
                            if ui.button("▶ Preview").clicked(){preview=Some(info.id.clone());}
                            if ui.button("Use sound").clicked(){load=Some(info.id.clone());}
                            if self.preview.as_ref().is_some_and(|p|p.is_playing()) && ui.button("■").clicked(){self.preview.as_mut().unwrap().stop();}
                        });
                    }else{ui.label(egui::RichText::new("Select a sound to preview it. Double-click to use it on the selected track.").small().weak());}
                    if let Some(error)=&self.library.error{ui.colored_label(Color32::LIGHT_RED,error);}
                });
                egui::ScrollArea::vertical().id_salt("sound_results").auto_shrink([false,false]).show_rows(ui,46.0,self.library.filtered.len(),|ui,rows| {
                    for row in rows {
                        let info=&dawwny_core::sound_catalog()[self.library.filtered[row]];
                        ui.push_id(&info.id,|ui| {
                            ui.horizontal(|ui| {
                                let starred=self.library.favorites.contains(&info.id);
                                if ui.add(egui::Button::new(if starred{"★"}else{"☆"}).frame(false)).on_hover_text("Favorite").clicked(){favorite=Some(info.id.clone());}
                                let selected=self.library.selected.as_ref()==Some(&info.id);
                                let (rect,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),46.0),egui::Sense::click());
                                if selected || response.hovered() {
                                    ui.painter().rect_filled(rect,9,if selected {Color32::from_rgb(63,75,47)}else{Color32::from_rgb(40,44,52)});
                                }
                                let mut content=ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(9.0,5.0))).layout(egui::Layout::top_down(egui::Align::Min)));
                                content.spacing_mut().item_spacing.y=2.0;
                                content.add(egui::Label::new(egui::RichText::new(&info.name).size(12.5)).truncate());
                                content.add(egui::Label::new(egui::RichText::new(&info.category).small().weak()).truncate());
                                response.widget_info(||egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel,ui.is_enabled(),selected,&info.name));
                                let response=response.on_hover_text(&info.name);
                                if response.clicked(){self.library.selected=Some(info.id.clone());}
                                if response.double_clicked(){load=Some(info.id.clone());}
                            });
                        });
                    }
                });
            });
        if let Some(id) = favorite {
            self.library.favorite(id);
        }
        if let Some(id) = preview {
            self.preview_sound(&id);
        }
        if let Some(id) = load {
            self.load_sound(&id);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn library_filters_favorites_and_auditions_without_project_edits() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("favorites.json");
        let mut state = LibraryState::new(file.clone());
        assert_eq!(state.filtered.len(), 2310);
        state.category = "Bass".into();
        state.query = "focused echo".into();
        state.filter();
        assert_eq!(state.filtered.len(), 6);
        let id = dawwny_core::sound_catalog()[state.filtered[0]].id.clone();
        state.favorite(id.clone());
        state.only_favorites = true;
        state.filter();
        assert_eq!(state.filtered.len(), 1);
        assert!(LibraryState::new(file).favorites.contains(&id));
        let info = dawwny_core::sound_catalog()
            .iter()
            .find(|s| s.id == id)
            .unwrap();
        let preview = audition(info);
        dawwny_core::validate(&preview).unwrap();
        let mut app = Studio::new(dir.path().join("session.json"), true).unwrap();
        let notes = app.project.tracks[0].clips.clone();
        app.load_sound(&id);
        assert_eq!(notes, app.project.tracks[0].clips);
        assert!(app.commit());
        assert_eq!(app.project.tracks[0].instrument, Instrument::Synth);
    }
}
