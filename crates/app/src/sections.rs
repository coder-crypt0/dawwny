use crate::{design::*, studio::Studio};
use dawwny_core::Section;
use eframe::egui;

impl Studio {
    pub fn toggle_cycle(&mut self) {
        self.finish_recording();
        self.looped = !self.looped;
        if let Some(audio) = &self.audio {
            audio.set_looped(self.looped);
        }
    }

    pub fn cycle_section(&mut self, index: usize) {
        self.finish_recording();
        let Some(section) = self.project.sections.get(index).cloned() else {
            return;
        };
        let range = section.cycle_range();
        self.edit(|p| p.cycle = Some(range));
        if self.project.cycle != Some(range) || self.error {
            return;
        }
        self.selected_section = Some(index);
        self.looped = true;
        if let Some(audio) = &self.audio {
            audio.set_looped(true);
        }
        self.seek(range.start);
        if !self.error {
            self.status = format!(
                "Cycling {} · bars {}–{}",
                section.name,
                section.start_bar + 1,
                section.start_bar + section.length_bars
            );
        }
    }

    pub fn sections_window(&mut self, ctx: &egui::Context) {
        if !self.show_sections {
            return;
        }
        let mut open = true;
        let mut sections = self.project.sections.clone();
        let original = sections.clone();
        let mut remove = None;
        let mut cycle = None;
        egui::Window::new("Arrangement sections")
            .open(&mut open)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Double-click a section above the ruler to cycle it.")
                        .color(MUTED),
                );
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        for (index, section) in sections.iter_mut().enumerate() {
                            egui::Frame::new()
                                .fill(BG)
                                .corner_radius(SURFACE_RADIUS)
                                .inner_margin(12)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut section.name)
                                                .desired_width(160.0),
                                        );
                                        if ui.button("Cycle").clicked() {
                                            cycle = Some(index);
                                        }
                                        if ui.button("Remove").clicked() {
                                            remove = Some(index);
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        let mut start = section.start_bar + 1;
                                        ui.add(
                                            egui::DragValue::new(&mut start)
                                                .range(1..=self.project.length_bars)
                                                .prefix("Start bar "),
                                        );
                                        section.start_bar = start - 1;
                                        let remaining =
                                            self.project.length_bars - section.start_bar;
                                        section.length_bars = section.length_bars.min(remaining);
                                        ui.add(
                                            egui::DragValue::new(&mut section.length_bars)
                                                .range(1..=remaining)
                                                .prefix("Length ")
                                                .suffix(" bars"),
                                        );
                                    });
                                });
                            ui.add_space(8.0);
                        }
                    });
                ui.horizontal(|ui| {
                    let end = sections
                        .iter()
                        .map(|s| s.start_bar + s.length_bars)
                        .max()
                        .unwrap_or(0);
                    if ui
                        .add_enabled(
                            sections.len() < 128 && end < self.project.length_bars,
                            egui::Button::new("+ Section"),
                        )
                        .clicked()
                    {
                        sections.push(Section {
                            name: format!("Section {}", sections.len() + 1),
                            start_bar: end,
                            length_bars: 4.min(self.project.length_bars - end),
                        });
                    }
                    if ui.button("Cycle whole song").clicked() {
                        self.edit(|p| p.cycle = None);
                        self.looped = true;
                        if let Some(audio) = &self.audio {
                            audio.set_looped(true);
                        }
                    }
                });
            });
        self.show_sections = open;
        let old_cycle = self.project.cycle;
        let mut new_cycle = old_cycle;
        for (old, new) in original.iter().zip(&sections) {
            if old_cycle == Some(old.cycle_range()) {
                new_cycle = Some(new.cycle_range());
            }
        }
        if let Some(index) = remove {
            if old_cycle == Some(original[index].cycle_range()) {
                new_cycle = None;
            }
            sections.remove(index);
            self.selected_section = None;
            cycle = None;
        }
        if sections != original {
            self.edit(|p| {
                p.sections = sections;
                p.cycle = new_cycle;
            });
        }
        if let Some(index) = cycle {
            self.cycle_section(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn double_clicking_the_rendered_section_selects_its_cycle_range() {
        let dir = tempfile::tempdir().unwrap();
        let mut studio = Studio::new(dir.path().join("song.json"), true).unwrap();
        let ctx = egui::Context::default();
        crate::views::theme(&ctx);
        let render = |studio: &mut Studio, time: f64, events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440.0, 900.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| studio.arrangement(ctx),
            )
        };
        let output = render(&mut studio, 0.0, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::epaint::Shape::Text(text) = &shape.shape
                    && text.galley.text() == "POCKET"
                {
                    Some(text.pos + text.galley.size() * 0.5)
                } else {
                    None
                }
            })
            .expect("section label should be rendered");
        for (time, pressed) in [(0.1, true), (0.12, false), (0.2, true), (0.22, false)] {
            let _ = render(
                &mut studio,
                time,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(
            studio.project.cycle,
            Some(studio.project.sections[1].cycle_range())
        );
        assert_eq!(studio.selected_section, Some(1));
        assert_eq!(studio.position(), 16.0);
    }
    #[test]
    fn section_cycle_seeks_persists_and_can_be_undone() {
        let dir = tempfile::tempdir().unwrap();
        let mut studio = Studio::new(dir.path().join("song.json"), true).unwrap();
        let before = studio.project.clone();
        let range = before.sections[1].cycle_range();
        studio.looped = false;
        studio.cycle_section(1);
        assert_eq!(studio.project.cycle, Some(range));
        assert_eq!(studio.position(), range.start);
        assert!(studio.looped);
        assert!(!studio.playing());
        assert!(studio.commit());
        assert_eq!(studio.store.load().unwrap().cycle, Some(range));
        assert_eq!(studio.project.tracks, before.tracks);
        studio.history(false);
        assert_eq!(studio.project.cycle, None);
        studio.history(true);
        assert_eq!(studio.project.cycle, Some(range));
    }
}
