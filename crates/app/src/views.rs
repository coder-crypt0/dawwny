use crate::studio::{EditorTab, Studio};
use dawwny_core::{Instrument, Note, Project};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::design::*;

pub fn theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = Color32::from_rgb(17, 19, 22);
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = SELECTED;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.inactive.bg_fill = CONTROL;
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(35, 38, 44);
    style.visuals.widgets.hovered.bg_fill = HOVER;
    style.visuals.widgets.active.bg_fill = SELECTED;
    style.spacing.interact_size = Vec2::new(38.0, 30.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(21.0));
    style.visuals.window_corner_radius = egui::CornerRadius::same(WINDOW_RADIUS);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(SURFACE_RADIUS);
    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(CONTROL_RADIUS);
        widget.bg_stroke = Stroke::NONE;
    }
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(11.0, 6.0);
    style.visuals.slider_trailing_fill = true;
    ctx.set_style(style);
    // Use the user's installed interface font; no proprietary font is redistributed.
    if cfg!(windows)
        && let Ok(windows) = std::env::var("WINDIR")
        && let Ok(bytes) = std::fs::read(std::path::Path::new(&windows).join("Fonts/segoeui.ttf"))
    {
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("system-ui".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "system-ui".into());
        ctx.set_fonts(fonts);
    }
}
pub fn instrument_name(i: Instrument) -> &'static str {
    match i {
        Instrument::Synth => "Dawn synth",
        Instrument::Keys => "Keys",
        Instrument::Pad => "Pad",
        Instrument::Bass => "Bass",
        Instrument::Lead => "Lead",
        Instrument::Drums => "Drums",
        Instrument::Sampler => "Sample instrument",
    }
}
fn color(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
fn caption(ui: &mut egui::Ui, s: &str) {
    ui.label(egui::RichText::new(s).size(12.0).color(MUTED));
}
fn note_name(n: u8) -> String {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", names[n as usize % 12], n as i16 / 12 - 1)
}

impl Studio {
    pub fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(210.0, 54.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.label(
                                egui::RichText::new("dawwny")
                                    .size(19.0)
                                    .color(ACCENT)
                                    .strong(),
                            );
                            let title: String = self.project.name.chars().take(24).collect();
                            ui.menu_button(format!("{title}  ▼"), |ui| {
                                if ui.button("New session").clicked() {
                                    self.new_document(Project::default());
                                    ui.close();
                                }
                                if ui.button("Open project / MIDI…    Ctrl+O").clicked() {
                                    self.open();
                                    ui.close();
                                }
                                if ui.button("Save    Ctrl+S").clicked() {
                                    self.commit();
                                    ui.close();
                                }
                                if ui.button("Save a copy…").clicked() {
                                    self.save_as();
                                    ui.close();
                                }
                                ui.separator();
                                if ui
                                    .add_enabled(
                                        !self.undo.is_empty() || self.pending.is_some(),
                                        egui::Button::new("Undo    Ctrl+Z"),
                                    )
                                    .clicked()
                                {
                                    self.history(false);
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(
                                        !self.redo.is_empty(),
                                        egui::Button::new("Redo    Ctrl+Y"),
                                    )
                                    .clicked()
                                {
                                    self.history(true);
                                    ui.close();
                                }
                                ui.separator();
                                let mut bars = self.project.length_bars;
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut bars)
                                            .range(1..=256)
                                            .prefix("Length: ")
                                            .suffix(" bars"),
                                    )
                                    .changed()
                                {
                                    self.edit(|p| p.length_bars = bars);
                                }
                                ui.label("Time signature: 4 / 4");
                                if ui.button("Open Velvet Dawn demo").clicked() {
                                    self.new_document(dawwny_core::demo_project());
                                    ui.close();
                                }
                                if ui.button("Studio guide").clicked() {
                                    self.show_help = true;
                                    ui.close();
                                }
                                if ui.button("About dawwny & licenses").clicked() {
                                    self.show_about = true;
                                    ui.close();
                                }
                            });
                        },
                    );
                    egui::Frame::new()
                        .fill(PANEL)
                        .corner_radius(SURFACE_RADIUS)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .add(egui::Button::new("■").min_size(Vec2::new(40.0, 34.0)))
                                    .on_hover_text("Stop and return to start")
                                    .clicked()
                                {
                                    self.stop();
                                }
                                let play = egui::Button::new(
                                    egui::RichText::new(if self.playing() { "Ⅱ" } else { "▶" })
                                        .color(BG)
                                        .size(18.0),
                                )
                                .fill(ACCENT)
                                .corner_radius(CONTROL_RADIUS)
                                .min_size(Vec2::new(40.0, 34.0));
                                if ui.add(play).on_hover_text("Play / pause · Space").clicked() {
                                    self.toggle_play();
                                }
                                let recording = self.recording();
                                if ui.add(egui::Button::new(
                                    egui::RichText::new("●").size(18.0).color(if recording { BG } else { RECORD }),
                                ).fill(if recording { RECORD } else { CONTROL }).min_size(Vec2::new(40.0, 34.0)))
                                    .on_hover_text("Record selected instrument · R · starts a linear take; cycle is disabled")
                                    .clicked() { self.toggle_recording(); }
                                let cycle_tip = self.project.cycle.map_or_else(
                                    || "Cycle whole song · C".to_owned(),
                                    |range| {
                                        format!(
                                            "Cycle bars {:.2}–{:.2} · C",
                                            range.start / 4.0 + 1.0,
                                            range.end / 4.0
                                        )
                                    },
                                );
                                let cycle_button = egui::Button::new(
                                    egui::RichText::new("↻").size(18.0).color(if self.looped {
                                        BG
                                    } else {
                                        MUTED
                                    }),
                                )
                                .fill(if self.looped { CYCLE } else { CONTROL })
                                .min_size(Vec2::new(40.0, 34.0));
                                if ui.add(cycle_button).on_hover_text(cycle_tip).clicked() {
                                    self.toggle_cycle();
                                }
                                let mut click = self.project.metronome;
                                let response = ui.add(egui::Button::new(
                                    egui::RichText::new("Click").size(12.0).color(if click.enabled { BG } else { MUTED })
                                ).fill(if click.enabled { ACCENT } else { CONTROL })
                                    .corner_radius(CONTROL_RADIUS).min_size(Vec2::new(48.0, 34.0)))
                                    .on_hover_text("Metronome · Ctrl+Shift+M · Right-click for level");
                                if response.clicked() { click.enabled = !click.enabled; }
                                response.context_menu(|ui| {
                                    ui.label(egui::RichText::new("Metronome").strong());
                                    ui.checkbox(&mut click.enabled, "Enable click");
                                    ui.add(egui::Slider::new(&mut click.gain, 0.0..=1.0).text("Level"));
                                    ui.label(egui::RichText::new("Accents beat one in 4/4. Follows master level; excluded from exports.").size(12.0).color(MUTED));
                                });
                                if click != self.project.metronome {
                                    self.edit(|project| project.metronome = click);
                                }
                                ui.add_space(6.0);
                                let position = self.position();
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:03}  {:02}  {:02}",
                                            position as u32 / 4 + 1,
                                            position as u32 % 4 + 1,
                                            (position.fract() * 100.0) as u32
                                        ))
                                        .monospace()
                                        .size(18.0),
                                    );
                                    caption(ui, "BAR     BEAT    TICK");
                                });
                                ui.separator();
                                let mut tempo = self.project.tempo;
                                ui.vertical(|ui| {
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut tempo)
                                                .range(30.0..=300.0)
                                                .speed(0.1)
                                                .fixed_decimals(1),
                                        )
                                        .changed()
                                    {
                                        self.edit(|p| p.tempo = tempo);
                                    }
                                    caption(ui, "BPM   ·   4/4");
                                });
                            });
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .selectable_label(self.keyboard.open, "Keys")
                            .on_hover_text("Musical typing and MIDI input · Ctrl+K")
                            .clicked()
                        {
                            self.keyboard.open = !self.keyboard.open;
                        }
                        ui.menu_button("Export", |ui| {
                            if self.export_job.is_some() {
                                ui.disable();
                            }
                            if ui.button("WAV · 24-bit / 48 kHz").clicked() {
                                self.export(true);
                                ui.close();
                            }
                            if ui.button("MIDI sequence").clicked() {
                                self.export(false);
                                ui.close();
                            }
                        });
                        if ui
                            .selectable_label(self.show_agents, "MCP")
                            .on_hover_text("Agent connection")
                            .clicked()
                        {
                            self.show_agents = !self.show_agents;
                            if self.show_agents && ctx.content_rect().width() < 1300.0 {
                                self.show_library = false;
                            }
                        }
                        if ui
                            .selectable_label(self.show_library, "Sounds")
                            .on_hover_text("Sound library · L")
                            .clicked()
                        {
                            self.show_library = !self.show_library;
                            if self.show_library && ctx.content_rect().width() < 1300.0 {
                                self.show_agents = false;
                            }
                        }
                    });
                });
            });
    }

    pub fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(16, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.status)
                            .size(12.0)
                            .color(if self.error { ERROR } else { MUTED }),
                    );
                    if self.conflict && ui.small_button("Save copy").clicked() {
                        self.save_as();
                    }
                    if self.conflict && ui.small_button("Reload disk version").clicked() {
                        self.reload();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let peak = self.audio.as_ref().map_or(0.0, |a| a.peak());
                        let (r, _) = ui.allocate_exact_size(Vec2::new(70.0, 6.0), Sense::hover());
                        ui.painter().rect_filled(r, 3.0, LINE);
                        ui.painter().rect_filled(
                            Rect::from_min_size(r.min, Vec2::new(r.width() * peak, 6.0)),
                            3.0,
                            ACCENT,
                        );
                        caption(
                            ui,
                            if self.playing() {
                                "NATIVE AUDIO"
                            } else {
                                "LOCAL STUDIO"
                            },
                        );
                    });
                });
            });
    }

    pub fn agent_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("agents").default_width(238.0).width_range(218.0..=350.0)
            .frame(egui::Frame::new().fill(PANEL).corner_radius(SURFACE_RADIUS).outer_margin(8).inner_margin(16)).show(ctx,|ui|{
            egui::ScrollArea::vertical().show(ui,|ui|{
                caption(ui,"LOCAL MCP"); ui.add_space(6.0); ui.heading("Agent connection");
                ui.add_space(8.0); ui.label("Connect your AI through the local MCP server. Every edit appears in this session.");
                ui.add_space(16.0); ui.separator(); ui.add_space(10.0);
                caption(ui,"CURRENT SESSION"); ui.label(egui::RichText::new(&self.project.name).strong());
                caption(ui,&format!("Revision {} • {} notes",self.project.revision,self.project.tracks.iter().flat_map(|t|&t.clips).map(|c|c.notes.len()).sum::<usize>()));
                if let Some(rev)=self.last_external_revision{ui.label(egui::RichText::new(format!("External revision {rev} received")).color(ACCENT).size(12.0));}
                ui.add_space(12.0);
                if ui.button("Copy MCP configuration").clicked(){
                    let executable=std::env::current_exe().unwrap_or_default();
                    let project=std::fs::canonicalize(self.store.path()).unwrap_or_else(|_|self.store.path().to_path_buf());
                    let exports=project.parent().unwrap_or(std::path::Path::new(".")).join("exports");
                    let config=serde_json::json!({"mcpServers":{"dawwny":{"command":executable,"args":["--mcp","--project",project,"--export-dir",exports]}}});
                    ctx.copy_text(serde_json::to_string_pretty(&config).unwrap_or_default());
                    self.status="MCP configuration copied. Add it to your agent client's MCP servers.".into();
                }
                if ui.button("Copy session path").clicked(){ctx.copy_text(std::fs::canonicalize(self.store.path()).unwrap_or_else(|_|self.store.path().to_path_buf()).display().to_string());}
                ui.add_space(12.0);caption(ui,"TRY WITH YOUR AGENT");
                ui.label(egui::RichText::new("“Read this session. Add a soft bass part that follows the chords, with a little variation in the last four bars.”").italics().color(TEXT));
                ui.add_space(18.0);ui.separator();ui.add_space(12.0);
                caption(ui,"MUSICAL TOOLS");
                for (a,b) in [("Read & arrange","Tracks, clips, notes, tempo"),("Shape sounds","Envelope, filter, reverb, delay"),("Mix & export","Gain, pan, mute, MIDI and WAV")] {
                    ui.label(egui::RichText::new(a).strong());caption(ui,b);ui.add_space(5.0);
                }
                ui.add_space(12.0);ui.label(egui::RichText::new("No model runs inside dawwny. Your MCP client controls the connection.").size(12.0).color(MUTED));
                ui.add_space(20.0);caption(ui,"NATIVE PLUGINS");
                ui.label("VST3 hosting is planned. This build uses its own small synthesizer engine.");
                if let Some(audio)=&self.audio{ui.add_space(20.0);caption(ui,"AUDIO OUTPUT");ui.label(egui::RichText::new(audio.device_name()).size(12.0).color(MUTED));}
            });
        });
    }

    pub fn arrangement(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .corner_radius(SURFACE_RADIUS)
                    .outer_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 4,
                        bottom: 4,
                    })
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                let timeline_width = ui.available_width();
                if self.fit_timeline {
                    self.zoom = ((timeline_width - 190.0)
                        / (84.0 * self.project.length_bars as f32))
                        .clamp(0.02, 3.0);
                }
                ui.horizontal(|ui| {
                    caption(ui, "ARRANGEMENT");
                    ui.add_space(12.0);
                    ui.menu_button("+ Track", |ui| {
                        for inst in [
                            Instrument::Synth,
                            Instrument::Keys,
                            Instrument::Pad,
                            Instrument::Bass,
                            Instrument::Lead,
                            Instrument::Drums,
                        ] {
                            if ui.button(instrument_name(inst)).clicked() {
                                self.add_track(inst);
                                ui.close();
                            }
                        }
                    });
                    if ui
                        .button("Sections")
                        .on_hover_text("Edit song sections and cycle ranges")
                        .clicked()
                    {
                        self.show_sections = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Slider::new(&mut self.zoom, 0.02..=3.0)
                                    .logarithmic(true)
                                    .show_value(false)
                                    .text("Zoom"),
                            )
                            .changed()
                        {
                            self.fit_timeline = false;
                        }
                        if ui.button("Fit").clicked() {
                            self.fit_timeline = true;
                        }
                    });
                });
                ui.add_space(8.0);
                let header = 190.0;
                let bar_width = 84.0 * self.zoom;
                let beat_width = bar_width / 4.0;
                let row_h = 58.0;
                let total_w = header + bar_width * self.project.length_bars as f32;
                let total_h = 54.0 + row_h * self.project.tracks.len() as f32;
                let mut select = None;
                let mut moved = None;
                let mut new_clip = None;
                let mut mix = None;
                let mut seek = None;
                let mut cycle_section = None;
                egui::ScrollArea::both()
                    .id_salt("arrange_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(
                                total_w.max(ui.available_width()),
                                total_h.max(ui.available_height()),
                            ),
                            Sense::click(),
                        );
                        let painter = ui.painter_at(rect);
                        let origin = rect.min;
                        let ruler = Rect::from_min_max(
                            Pos2::new(origin.x + header, origin.y + 25.0),
                            Pos2::new(rect.right(), origin.y + 54.0),
                        );
                        let ruler_response = ui
                            .interact(
                                ruler,
                                ui.id().with("playhead_ruler"),
                                Sense::click_and_drag(),
                            )
                            .on_hover_text("Click or drag to move the playhead")
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if (ruler_response.clicked() || ruler_response.dragged())
                            && let Some(p) = ruler_response.interact_pointer_pos()
                        {
                            seek = Some(
                                ((p.x - origin.x - header) as f64 / beat_width as f64)
                                    .clamp(0.0, self.project.length_bars as f64 * 4.0),
                            );
                        }
                        painter.rect_filled(rect, SURFACE_RADIUS, BG);
                        painter.rect_filled(
                            Rect::from_min_size(origin, Vec2::new(rect.width(), 54.0)),
                            egui::CornerRadius {
                                nw: SURFACE_RADIUS,
                                ne: SURFACE_RADIUS,
                                sw: 0,
                                se: 0,
                            },
                            PANEL,
                        );
                        painter.text(
                            origin + Vec2::new(12.0, 27.0),
                            Align2::LEFT_CENTER,
                            "TRACKS",
                            FontId::proportional(12.0),
                            MUTED,
                        );
                        if self.looped
                            && let Some(range) = self.project.cycle
                        {
                            let left = origin.x + header + range.start as f32 * beat_width;
                            let right = origin.x + header + range.end as f32 * beat_width;
                            let band = Rect::from_min_max(
                                Pos2::new(left, origin.y + 25.0),
                                Pos2::new(right, rect.bottom()),
                            );
                            painter.rect_filled(band, MEDIA_RADIUS, CYCLE.gamma_multiply(0.07));
                            painter.rect_filled(
                                Rect::from_min_max(band.min, Pos2::new(right, origin.y + 54.0)),
                                MEDIA_RADIUS,
                                CYCLE.gamma_multiply(0.16),
                            );
                            for x in [left, right] {
                                painter.line_segment(
                                    [Pos2::new(x, origin.y + 25.0), Pos2::new(x, rect.bottom())],
                                    Stroke::new(1.0_f32, CYCLE.gamma_multiply(0.6)),
                                );
                            }
                        }
                        for (index, section) in self.project.sections.iter().enumerate() {
                            let x = origin.x + header + section.start_bar as f32 * bar_width;
                            let r = Rect::from_min_size(
                                Pos2::new(x, origin.y),
                                Vec2::new(
                                    (section.length_bars as f32 * bar_width - 2.0).max(4.0),
                                    23.0,
                                ),
                            );
                            let cycling =
                                self.looped && self.project.cycle == Some(section.cycle_range());
                            let selected = self.selected_section == Some(index);
                            let response = ui
                                .put(
                                    r,
                                    egui::Button::new(
                                        egui::RichText::new(&section.name)
                                            .size(12.0)
                                            .color(if cycling { BG } else { TEXT }),
                                    )
                                    .fill(if cycling {
                                        CYCLE
                                    } else if selected {
                                        SELECTED
                                    } else {
                                        CONTROL
                                    })
                                    .corner_radius(MEDIA_RADIUS)
                                    .truncate(),
                                )
                                .on_hover_text(format!(
                                    "{} · bars {}–{}\nDouble-click to cycle",
                                    section.name,
                                    section.start_bar + 1,
                                    section.start_bar + section.length_bars
                                ));
                            if response.clicked() {
                                self.selected_section = Some(index);
                            }
                            if response.double_clicked() {
                                cycle_section = Some(index);
                            }
                            response.context_menu(|ui| {
                                if ui.button("Cycle section").clicked() {
                                    cycle_section = Some(index);
                                    ui.close();
                                }
                                if ui.button("Edit sections…").clicked() {
                                    self.show_sections = true;
                                    ui.close();
                                }
                            });
                        }
                        for bar in 0..=self.project.length_bars {
                            let x = origin.x + header + bar as f32 * bar_width;
                            painter.line_segment(
                                [Pos2::new(x, origin.y + 27.0), Pos2::new(x, rect.bottom())],
                                Stroke::new(1.0_f32, LINE),
                            );
                            if bar < self.project.length_bars
                                && bar % (40.0 / bar_width).ceil().max(1.0) as u32 == 0
                            {
                                painter.text(
                                    Pos2::new(x + 7.0, origin.y + 39.0),
                                    Align2::LEFT_CENTER,
                                    format!("{:02}", bar + 1),
                                    FontId::monospace(12.0),
                                    MUTED,
                                );
                            }
                        }
                        let any_solo = self.project.tracks.iter().any(|track| track.solo);
                        let mut clip_hit = false;
                        for (ti, track) in self.project.tracks.iter().enumerate() {
                            let y = origin.y + 54.0 + ti as f32 * row_h;
                            let row = Rect::from_min_size(
                                Pos2::new(origin.x, y),
                                Vec2::new(rect.width(), row_h),
                            );
                            if ti == self.selected_track {
                                painter.rect_filled(row.shrink(2.0), CONTROL_RADIUS, SURFACE);
                            }
                            painter.line_segment(
                                [row.left_bottom(), row.right_bottom()],
                                Stroke::new(1.0_f32, LINE),
                            );
                            let c = color(track.color);
                            let inaudible = track.mute || (any_solo && !track.solo);
                            painter.rect_filled(
                                Rect::from_min_size(
                                    row.min + Vec2::new(0.0, 11.0),
                                    Vec2::new(3.0, 44.0),
                                ),
                                2.0,
                                c,
                            );
                            painter.text(
                                row.min + Vec2::new(13.0, 19.0),
                                Align2::LEFT_CENTER,
                                &track.name,
                                FontId::proportional(14.0),
                                TEXT,
                            );
                            painter.text(
                                row.min + Vec2::new(13.0, 43.0),
                                Align2::LEFT_CENTER,
                                instrument_name(track.instrument),
                                FontId::proportional(12.0),
                                MUTED,
                            );
                            let sr = Rect::from_min_size(
                                row.min + Vec2::new(2.0, 0.0),
                                Vec2::new(header - 65.0, row_h),
                            );
                            if ui
                                .interact(sr, ui.id().with(("track", ti)), Sense::click())
                                .clicked()
                            {
                                select = Some((ti, 0));
                            }
                            for (j, label, active) in [(0, "M", track.mute), (1, "S", track.solo)] {
                                let r = Rect::from_min_size(
                                    row.min + Vec2::new(header - 61.0 + j as f32 * 27.0, 31.0),
                                    Vec2::new(22.0, 23.0),
                                );
                                let active_color = if j == 0 { CYCLE } else { ACCENT };
                                let response = ui
                                    .scope(|ui| {
                                        ui.spacing_mut().button_padding = Vec2::new(5.0, 3.0);
                                        ui.spacing_mut().interact_size = Vec2::new(22.0, 23.0);
                                        ui.put(
                                            r,
                                            egui::Button::new(
                                                egui::RichText::new(label)
                                                    .size(12.0)
                                                    .color(if active { BG } else { MUTED }),
                                            )
                                            .fill(if active { active_color } else { CONTROL })
                                            .corner_radius(CONTROL_RADIUS),
                                        )
                                        .on_hover_text(
                                            if j == 0 { "Mute track" } else { "Solo track" },
                                        )
                                    })
                                    .inner;
                                if response.clicked() {
                                    mix = Some((ti, j, !active));
                                }
                            }
                            for (ci, clip) in track.clips.iter().enumerate() {
                                let r = Rect::from_min_size(
                                    Pos2::new(
                                        origin.x + header + clip.start as f32 * beat_width + 2.0,
                                        y + 6.0,
                                    ),
                                    Vec2::new(
                                        (clip.length as f32 * beat_width - 4.0).max(4.0),
                                        row_h - 12.0,
                                    ),
                                );
                                let sel = ti == self.selected_track && ci == self.selected_clip;
                                painter.rect_filled(
                                    r,
                                    MEDIA_RADIUS,
                                    c.gamma_multiply(if inaudible { 0.18 } else { 0.32 }),
                                );
                                painter.rect_filled(
                                    Rect::from_min_size(r.min, Vec2::new(r.width(), 19.0)),
                                    egui::CornerRadius {
                                        nw: MEDIA_RADIUS,
                                        ne: MEDIA_RADIUS,
                                        sw: 0,
                                        se: 0,
                                    },
                                    c.gamma_multiply(if inaudible { 0.4 } else { 0.78 }),
                                );
                                if sel {
                                    painter.rect_stroke(
                                        r,
                                        MEDIA_RADIUS,
                                        Stroke::new(1.5_f32, c),
                                        StrokeKind::Inside,
                                    );
                                }
                                let cp = ui.painter_at(r.shrink(4.0));
                                cp.text(
                                    r.min + Vec2::new(7.0, 10.0),
                                    Align2::LEFT_CENTER,
                                    &clip.name,
                                    FontId::proportional(12.0),
                                    TEXT,
                                );
                                let min =
                                    clip.notes.iter().map(|n| n.pitch).min().unwrap_or(48) as f32;
                                let max =
                                    clip.notes.iter().map(|n| n.pitch).max().unwrap_or(72) as f32;
                                for n in &clip.notes {
                                    let x =
                                        r.left() + n.start as f32 / clip.length as f32 * r.width();
                                    let ny = r.bottom()
                                        - 6.0
                                        - (n.pitch as f32 - min) / (max - min).max(8.0) * 21.0;
                                    cp.rect_filled(
                                        Rect::from_min_size(
                                            Pos2::new(x, ny),
                                            Vec2::new(
                                                (n.duration as f32 / clip.length as f32
                                                    * r.width())
                                                .max(2.0),
                                                2.5,
                                            ),
                                        ),
                                        1.0,
                                        c.gamma_multiply(if track.mute { 0.3 } else { 1.0 }),
                                    );
                                }
                                let res = ui
                                    .interact(
                                        r,
                                        ui.id().with(("clip", ti, ci)),
                                        Sense::click_and_drag(),
                                    )
                                    .on_hover_text(format!(
                                        "{} · {:.1} beats · {} notes",
                                        clip.name,
                                        clip.length,
                                        clip.notes.len()
                                    ));
                                if res.hovered() {
                                    clip_hit = true;
                                }
                                if res.clicked() {
                                    select = Some((ti, ci));
                                }
                                if res.drag_started() {
                                    self.clip_drag = ui
                                        .input(|i| i.pointer.press_origin())
                                        .map(|origin| (ti, ci, clip.start, origin));
                                    select = Some((ti, ci));
                                }
                                if res.drag_stopped()
                                    && let Some((t, c, start, drag_origin)) = self.clip_drag.take()
                                {
                                    let delta =
                                        ui.input(|i| i.pointer.latest_pos()).unwrap_or(drag_origin)
                                            - drag_origin;
                                    let target = ((start + delta.x as f64 / beat_width as f64)
                                        / self.grid)
                                        .round()
                                        * self.grid;
                                    moved = Some((
                                        t,
                                        c,
                                        target.max(0.0).min(
                                            self.project.length_bars as f64 * 4.0 - clip.length,
                                        ),
                                    ));
                                }
                            }
                        }
                        if response.double_clicked()
                            && !clip_hit
                            && let Some(p) = response.interact_pointer_pos()
                        {
                            let ti = ((p.y - origin.y - 54.0) / row_h).floor() as usize;
                            if p.x > origin.x + header
                                && p.y > origin.y + 54.0
                                && ti < self.project.tracks.len()
                            {
                                let beat =
                                    ((p.x - origin.x - header) / bar_width).floor() as f64 * 4.0;
                                new_clip = Some((ti, beat));
                            }
                        }
                        {
                            let x = origin.x + header + self.position() as f32 * beat_width;
                            painter.rect_filled(
                                Rect::from_center_size(
                                    Pos2::new(x, origin.y + 28.5),
                                    Vec2::new(10.0, 7.0),
                                ),
                                3.0,
                                ACCENT,
                            );
                            painter.line_segment(
                                [Pos2::new(x, origin.y + 25.0), Pos2::new(x, rect.bottom())],
                                Stroke::new(1.5_f32, ACCENT),
                            );
                        }
                        if self.project.tracks.is_empty() {
                            painter.text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                "Add an instrument track to start composing.",
                                FontId::proportional(17.0),
                                MUTED,
                            );
                        }
                    });
                if let Some((t, c)) = select {
                    self.selected_track = t;
                    self.selected_clip = c;
                    self.sync_audio();
                }
                if let Some(index) = cycle_section {
                    self.cycle_section(index);
                }
                if let Some(beat) = seek {
                    self.seek(beat);
                }
                if let Some((t, c, start)) = moved {
                    self.edit(|p| p.tracks[t].clips[c].start = start);
                }
                if let Some((t, j, v)) = mix {
                    self.edit(|p| {
                        if j == 0 {
                            p.tracks[t].mute = v
                        } else {
                            p.tracks[t].solo = v
                        }
                    });
                }
                if let Some((t, b)) = new_clip {
                    self.add_clip(t, b);
                }
            });
    }

    pub fn editor(&mut self, ctx: &egui::Context) {
        let panel = if self.editor_expanded {
            egui::TopBottomPanel::bottom("editor_expanded")
                .exact_height((ctx.available_rect().height() - 8.0).max(230.0))
                .resizable(false)
        } else {
            egui::TopBottomPanel::bottom("editor")
                .default_height(290.0)
                .min_height(230.0)
                .max_height(650.0)
                .resizable(true)
        };
        panel
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .corner_radius(SURFACE_RADIUS)
                    .outer_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 4,
                        bottom: 8,
                    })
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, EditorTab::Piano, "Piano roll");
                    ui.selectable_value(&mut self.tab, EditorTab::Sound, "Sound");
                    ui.selectable_value(&mut self.tab, EditorTab::Mixer, "Mixer");
                    ui.separator();
                    if let Some(t) = self.project.tracks.get(self.selected_track) {
                        ui.label(egui::RichText::new(&t.name).color(color(t.color)));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(if self.editor_expanded {
                                "Collapse"
                            } else {
                                "Expand"
                            })
                            .on_hover_text("Use the full workspace for this editor")
                            .clicked()
                        {
                            self.editor_expanded = !self.editor_expanded;
                        }
                    });
                });
                ui.separator();
                match self.tab {
                    EditorTab::Piano => self.piano(ui),
                    EditorTab::Sound => self.sound(ui),
                    EditorTab::Mixer => self.mixer(ui),
                }
            });
    }

    fn piano(&mut self, ui: &mut egui::Ui) {
        let ti = self.selected_track;
        let ci = self.selected_clip;
        let Some(track) = self.project.tracks.get(ti) else {
            ui.label("Select an instrument track.");
            return;
        };
        let Some(clip) = track.clips.get(ci).cloned() else {
            ui.add_space(24.0);
            ui.label("This track has no clips yet.");
            if ui.button("+ Create a one-bar clip").clicked() {
                self.add_clip(ti, 0.0);
            }
            return;
        };
        let tint = color(track.color);
        let mut delete_clip = false;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&clip.name).strong());
            caption(ui, &format!("{} notes", clip.notes.len()));
            ui.separator();
            egui::ComboBox::from_id_salt("grid")
                .selected_text(format!("Grid 1/{}", (4.0 / self.grid) as u32))
                .width(92.0)
                .show_ui(ui, |ui| {
                    for (val, label) in
                        [(1.0, "1/4"), (0.5, "1/8"), (0.25, "1/16"), (0.125, "1/32")]
                    {
                        ui.selectable_value(&mut self.grid, val, label);
                    }
                });
            ui.add(
                egui::DragValue::new(&mut self.note_length)
                    .range(0.125..=16.0)
                    .speed(0.125)
                    .prefix("Length "),
            );
            ui.add(
                egui::Slider::new(&mut self.note_velocity, 0.05..=1.0)
                    .text("Velocity")
                    .show_value(false),
            );
            ui.menu_button("Notes", |ui| {
                if ui.button(format!("Quantize to 1/{} grid", (4.0 / self.grid) as u32)).on_hover_text("Align note starts, preserving duration and velocity · Undo restores the performance").clicked() {
                    self.transform_clip(None); ui.close();
                }
                ui.menu_button("Transpose", |ui| {
                    for (semitones, label) in [(-12, "Down an octave"), (-1, "Down a semitone"), (1, "Up a semitone"), (12, "Up an octave")] {
                        if ui.button(label).clicked() { self.transform_clip(Some(semitones)); ui.close(); }
                    }
                });
            });
            if ui.button("Duplicate").clicked() {
                self.duplicate_clip();
            }
            if ui.button("Delete clip").clicked() {
                delete_clip = true;
            }
        });
        if delete_clip {
            self.edit(|p| {
                p.tracks[ti].clips.remove(ci);
            });
            self.clamp_selection();
            self.note_drag = None;
            return;
        }
        let lo = clip
            .notes
            .iter()
            .map(|n| n.pitch)
            .min()
            .unwrap_or(60)
            .saturating_sub(5)
            .min(60);
        let hi = clip
            .notes
            .iter()
            .map(|n| n.pitch)
            .max()
            .unwrap_or(72)
            .saturating_add(5)
            .max(lo + 18)
            .min(127);
        let row = 17.0;
        let key = 48.0;
        let beat = 55.0 * self.piano_zoom;
        let width = key + clip.length as f32 * beat;
        let height = (hi - lo + 1) as f32 * row;
        let mut add = None;
        let mut remove = None;
        let mut move_note = None;
        egui::ScrollArea::both().id_salt(("piano",&clip.id)).auto_shrink([false,false]).show(ui,|ui| {
            let (rect,response)=ui.allocate_exact_size(Vec2::new(width.max(ui.available_width()),height),Sense::click());
            let painter=ui.painter_at(rect);let origin=rect.min;
            for pitch in lo..=hi {
                let y=origin.y+(hi-pitch)as f32*row;
                let black=matches!(pitch%12,1|3|6|8|10);
                let rr=Rect::from_min_size(Pos2::new(origin.x,y),Vec2::new(rect.width(),row));
                painter.rect_filled(rr,3.0,if black{Color32::from_rgb(22,24,29)}else{Color32::from_rgb(30,33,38)});
                let kr=Rect::from_min_size(rr.min,Vec2::new(key-2.0,row-1.0));
                painter.rect_filled(kr,MEDIA_RADIUS,if black{Color32::from_rgb(42,45,51)}else{Color32::from_rgb(175,181,185)});
                if pitch%12==0||!black {
                    painter.text(kr.center(),Align2::CENTER_CENTER,note_name(pitch),FontId::monospace(11.0),if black{TEXT}else{BG});
                }
            }
            for step in 0..=(clip.length/self.grid)as usize {
                let x=origin.x+key+step as f32*self.grid as f32*beat;
                let major=(step as f64*self.grid)%4.0<0.01;
                painter.line_segment([Pos2::new(x,rect.top()),Pos2::new(x,rect.bottom())],Stroke::new(1.0_f32,if major{Color32::from_rgb(65,70,80)}else{LINE}));
            }
            let mut hit=false;
            for (i,n) in clip.notes.iter().enumerate() {
                let nr=Rect::from_min_size(Pos2::new(origin.x+key+n.start as f32*beat,origin.y+(hi-n.pitch)as f32*row+1.0),Vec2::new((n.duration as f32*beat-1.0).max(3.0),row-2.0));
                painter.rect_filled(nr,MEDIA_RADIUS,tint.gamma_multiply(0.55+n.velocity*0.45));
                if nr.width()>28.0 {
                    ui.painter_at(nr).text(nr.left_center()+Vec2::new(5.0,0.0),Align2::LEFT_CENTER,note_name(n.pitch),FontId::monospace(11.0),BG);
                }
                let res=ui.interact(nr,ui.id().with(&n.id),Sense::click_and_drag()).on_hover_text(format!("{} · {:.2} beats · velocity {:.0}%\nDrag to move · right-click to edit",note_name(n.pitch),n.duration,n.velocity*100.0));
                if res.hovered(){hit=true;}
                let mut edited=n.clone();
                res.context_menu(|ui| {
                    ui.label(egui::RichText::new("Note properties").strong());
                    ui.add(egui::DragValue::new(&mut edited.pitch).range(0..=127).prefix("Pitch "));
                    ui.add(egui::DragValue::new(&mut edited.duration).range((1.0/960.0)..=(clip.length-n.start)).speed(0.125).prefix("Length ").suffix(" beats"));
                    ui.add(egui::Slider::new(&mut edited.velocity,0.0..=1.0).text("Velocity"));
                    if ui.button("Delete note").clicked(){remove=Some(i);ui.close();}
                });
                if edited!=*n{move_note=Some((i,edited));}
                if res.drag_started(){self.note_drag=ui.input(|i|i.pointer.press_origin()).map(|origin|(i,n.clone(),origin));}
                if res.dragged() && let Some((_,original,drag_origin))=&self.note_drag {
                    let delta=ui.input(|i|i.pointer.latest_pos()).unwrap_or(*drag_origin)-*drag_origin;
                    let start=((original.start+delta.x as f64/beat as f64)/self.grid).round()*self.grid;
                    let start=start.clamp(0.0,(clip.length-original.duration).max(0.0));
                    let ghost=nr.translate(Vec2::new((start-n.start)as f32*beat,(delta.y/row).round()*row));
                    painter.rect_filled(ghost,MEDIA_RADIUS,tint.gamma_multiply(0.35));
                    painter.rect_stroke(ghost,MEDIA_RADIUS,Stroke::new(1.3_f32,TEXT),StrokeKind::Inside);
                }
                if res.drag_stopped() && let Some((index,mut original,drag_origin))=self.note_drag.take() {
                    let delta=ui.input(|i|i.pointer.latest_pos()).unwrap_or(drag_origin)-drag_origin;
                    let dx=delta.x as f64/beat as f64;
                    original.start=((original.start+dx)/self.grid).round()*self.grid;
                    original.start=original.start.clamp(0.0,(clip.length-original.duration).max(0.0));
                    original.pitch=(original.pitch as i16-(delta.y/row).round()as i16).clamp(0,127)as u8;
                    move_note=Some((index,original));
                }
            }
            if response.clicked() && !hit && let Some(p)=response.interact_pointer_pos() && p.x>=origin.x+key && p.x<origin.x+width {
                let start=(((p.x-origin.x-key)/beat)as f64/self.grid).floor()*self.grid;
                let pitch=hi.saturating_sub(((p.y-origin.y)/row)as u8);
                let duration=self.note_length.min(clip.length-start);
                if duration>=1.0/960.0 {add=Some(Note{id:dawwny_core::new_id(),pitch,start,duration,velocity:self.note_velocity});}
            }
            if self.playing() {
                let position=self.position()-clip.start;
                if position>=0.0 && position<clip.length {
                    let x=origin.x+key+position as f32*beat;
                    painter.line_segment([Pos2::new(x,rect.top()),Pos2::new(x,rect.bottom())],Stroke::new(1.5_f32,ACCENT));
                }
            }
        });
        if let Some(n) = add {
            self.edit(|p| p.tracks[ti].clips[ci].notes.push(n));
        }
        if let Some(i) = remove {
            self.edit(|p| {
                p.tracks[ti].clips[ci].notes.remove(i);
            });
        }
        if remove.is_none()
            && let Some((i, n)) = move_note
        {
            self.edit(|p| p.tracks[ti].clips[ci].notes[i] = n);
        }
    }

    fn sound(&mut self, ui: &mut egui::Ui) {
        let ti = self.selected_track;
        let Some(mut track) = self.project.tracks.get(ti).cloned() else {
            ui.label("Add a track to shape a sound.");
            return;
        };
        let before = track.clone();
        egui::ScrollArea::vertical()
            .id_salt(("sound", &track.id))
            .show(ui, |ui| {
                if crate::sound_editor::show(ui, &mut track, self.samples.bank.as_deref()) {
                    self.samples.open = true;
                }
            });
        if track != before {
            self.edit(|p| p.tracks[ti] = track);
        }
    }

    fn mixer(&mut self, ui: &mut egui::Ui) {
        let mut change = None;
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for (ti, t) in self.project.tracks.iter().enumerate() {
                    let mut gain = t.gain;
                    let mut pan = t.pan;
                    let mut mute = t.mute;
                    let mut solo = t.solo;
                    egui::Frame::new()
                        .fill(BG)
                        .inner_margin(egui::Margin::same(12))
                        .corner_radius(SURFACE_RADIUS)
                        .show(ui, |ui| {
                            ui.set_width(115.0);
                            ui.label(egui::RichText::new(&t.name).color(color(t.color)).strong());
                            caption(ui, instrument_name(t.instrument));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Slider::new(&mut gain, 0.0..=1.0)
                                        .vertical()
                                        .show_value(false),
                                );
                                ui.vertical(|ui| {
                                    ui.add_space(15.0);
                                    ui.label(format!("{:.1} dB", 20.0 * gain.max(0.00001).log10()));
                                    crate::design::mix_toggle(ui, &mut mute, "M", true);
                                    crate::design::mix_toggle(ui, &mut solo, "S", false);
                                });
                            });
                            ui.add(
                                egui::Slider::new(&mut pan, -1.0..=1.0)
                                    .show_value(false)
                                    .text("Pan"),
                            );
                        });
                    if gain != t.gain || pan != t.pan || mute != t.mute || solo != t.solo {
                        change = Some((ti, gain, pan, mute, solo));
                    }
                }
                let mut master = self.project.master_gain;
                egui::Frame::new()
                    .fill(Color32::from_rgb(39, 46, 36))
                    .inner_margin(egui::Margin::same(12))
                    .corner_radius(SURFACE_RADIUS)
                    .show(ui, |ui| {
                        ui.set_width(100.0);
                        ui.label(egui::RichText::new("MASTER").color(ACCENT).strong());
                        caption(ui, "Stereo output");
                        ui.add(
                            egui::Slider::new(&mut master, 0.0..=1.0)
                                .vertical()
                                .show_value(false),
                        );
                        ui.label(format!("{:.1} dB", 20.0 * master.max(0.00001).log10()));
                    });
                if master != self.project.master_gain {
                    self.edit(|p| p.master_gain = master);
                }
            });
        });
        if let Some((i, gain, pan, mute, solo)) = change {
            self.edit(|p| {
                let t = &mut p.tracks[i];
                t.gain = gain;
                t.pan = pan;
                t.mute = mute;
                t.solo = solo;
            });
        }
    }
}
