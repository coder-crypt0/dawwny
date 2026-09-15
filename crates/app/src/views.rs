use crate::studio::{EditorTab, Studio};
use dawwny_core::{Instrument, Note, Project};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

const BG: Color32 = Color32::from_rgb(21, 23, 27);
const PANEL: Color32 = Color32::from_rgb(28, 30, 35);
const LINE: Color32 = Color32::from_rgb(47, 50, 57);
const TEXT: Color32 = Color32::from_rgb(226, 230, 232);
const MUTED: Color32 = Color32::from_rgb(140, 148, 159);
const ACCENT: Color32 = Color32::from_rgb(203, 231, 143);

pub fn theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = Color32::from_rgb(17, 19, 22);
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = Color32::from_rgb(79, 96, 56);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(39, 42, 48);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(35, 38, 44);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(58, 64, 70);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(74, 85, 65);
    style.spacing.item_spacing = Vec2::new(10.0, 9.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
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
    ctx.set_style(style);
}
pub fn instrument_name(i: Instrument) -> &'static str {
    match i {
        Instrument::Keys => "Keys",
        Instrument::Pad => "Pad",
        Instrument::Bass => "Bass",
        Instrument::Lead => "Lead",
        Instrument::Drums => "Drums",
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
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(18, 12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("dawwny")
                            .size(26.0)
                            .strong()
                            .color(ACCENT),
                    );
                    ui.add_space(10.0);
                    ui.menu_button("Project", |ui| {
                        if ui.button("New session").clicked() {
                            self.new_document(Project::default());
                            ui.close();
                        }
                        if ui.button("Open project / MIDI…").clicked() {
                            self.open();
                            ui.close();
                        }
                        if ui.button("Save  Ctrl+S").clicked() {
                            self.commit();
                            ui.close();
                        }
                        if ui.button("Save a copy…").clicked() {
                            self.save_as();
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Open Velvet Dawn demo").clicked() {
                            self.new_document(dawwny_core::demo_project());
                            ui.close();
                        }
                    });
                    if ui
                        .add_enabled(
                            !self.undo.is_empty() || self.pending.is_some(),
                            egui::Button::new("Undo"),
                        )
                        .clicked()
                    {
                        self.history(false);
                    }
                    if ui
                        .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                        .clicked()
                    {
                        self.history(true);
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(&self.project.name).strong());
                    caption(ui, &format!(" /  {} tracks", self.project.tracks.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button("Export", |ui| {
                            if self.export_job.is_some() {
                                ui.disable();
                            }
                            if ui.button("WAV audio · 24-bit / 48 kHz").clicked() {
                                self.export(true);
                                ui.close();
                            }
                            if ui.button("MIDI sequence").clicked() {
                                self.export(false);
                                ui.close();
                            }
                        });
                        if ui
                            .selectable_label(self.show_agents, "Agent connection")
                            .clicked()
                        {
                            self.show_agents = !self.show_agents;
                        }
                        if ui.button("?").on_hover_text("Studio guide").clicked() {
                            self.show_help = true;
                        }
                    });
                });
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("■")
                        .on_hover_text("Stop and return to start")
                        .clicked()
                    {
                        self.stop();
                    }
                    let play = egui::Button::new(
                        egui::RichText::new(if self.playing() {
                            "Ⅱ  Stop"
                        } else {
                            "▶  Play"
                        })
                        .color(BG)
                        .strong(),
                    )
                    .fill(ACCENT)
                    .min_size(Vec2::new(100.0, 34.0));
                    if ui.add(play).clicked() {
                        self.toggle_play();
                    }
                    ui.toggle_value(&mut self.looped, "Loop")
                        .on_hover_text("Applies on the next playback start");
                    ui.add_space(10.0);
                    let pos = self.position();
                    ui.label(
                        egui::RichText::new(format!(
                            "{:03}  :  {:02}  :  {:02}",
                            pos as u32 / 4 + 1,
                            pos as u32 % 4 + 1,
                            (pos.fract() * 100.0) as u32
                        ))
                        .monospace()
                        .size(23.0),
                    );
                    ui.separator();
                    let mut tempo = self.project.tempo;
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
                    caption(ui, "BPM");
                    ui.separator();
                    ui.label("4 / 4");
                    ui.separator();
                    let mut bars = self.project.length_bars;
                    if ui
                        .add(egui::DragValue::new(&mut bars).range(1..=256))
                        .changed()
                    {
                        self.edit(|p| p.length_bars = bars);
                    }
                    caption(ui, "BARS");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(a) = &self.audio {
                            caption(ui, &format!("{} Hz  /  NATIVE", a.sample_rate()));
                        } else {
                            caption(ui, "NO AUDIO DEVICE");
                        }
                        let peak = self.audio.as_ref().map_or(0.0, |a| a.peak());
                        let (r, _) = ui.allocate_exact_size(Vec2::new(88.0, 15.0), Sense::hover());
                        ui.painter()
                            .rect_filled(r, 2.0, Color32::from_rgb(16, 19, 21));
                        for i in 0..18 {
                            let on = (i as f32 / 18.0) < peak;
                            let c = if on {
                                if i > 15 {
                                    Color32::from_rgb(233, 162, 98)
                                } else {
                                    ACCENT
                                }
                            } else {
                                LINE
                            };
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    r.min + Vec2::new(i as f32 * 5.0, 1.0),
                                    Vec2::new(3.0, 13.0),
                                ),
                                1.0,
                                c,
                            );
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
                            .color(if self.error {
                                Color32::from_rgb(255, 169, 144)
                            } else {
                                MUTED
                            }),
                    );
                    if self.conflict && ui.small_button("Save copy").clicked() {
                        self.save_as();
                    }
                    if self.conflict && ui.small_button("Reload disk version").clicked() {
                        self.reload();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        caption(ui, "NATIVE FOUNDATION  0.1");
                    });
                });
            });
    }

    pub fn agent_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("agents").default_width(238.0).width_range(218.0..=350.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::same(18))).show(ctx,|ui|{
            egui::ScrollArea::vertical().show(ui,|ui|{
                caption(ui,"LOCAL MCP"); ui.add_space(6.0); ui.heading("Agent connection");
                ui.add_space(8.0); ui.label("Connect your AI through the local MCP server. Every edit appears in this session.");
                ui.add_space(16.0); ui.separator(); ui.add_space(10.0);
                caption(ui,"CURRENT SESSION"); ui.label(egui::RichText::new(&self.project.name).strong());
                caption(ui,&format!("Revision {} • {} notes",self.project.revision,self.project.tracks.iter().flat_map(|t|&t.clips).map(|c|c.notes.len()).sum::<usize>()));
                if let Some(rev)=self.last_external_revision{ui.label(egui::RichText::new(format!("External revision {rev} received")).color(ACCENT).size(12.0));}
                ui.add_space(12.0);
                if ui.button("Copy MCP configuration").clicked(){
                    let executable=std::env::current_exe().unwrap_or_default().with_file_name(if cfg!(windows){"dawwny-mcp.exe"}else{"dawwny-mcp"});
                    let project=std::fs::canonicalize(self.store.path()).unwrap_or_else(|_|self.store.path().to_path_buf());
                    let exports=project.parent().unwrap_or(std::path::Path::new(".")).join("exports");
                    let config=serde_json::json!({"mcpServers":{"dawwny":{"command":executable,"args":["--project",project,"--export-dir",exports]}}});
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
                    .fill(BG)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    caption(ui, "ARRANGEMENT");
                    ui.add_space(12.0);
                    ui.menu_button("+ Track", |ui| {
                        for inst in [
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.zoom, 0.6..=3.0)
                                .show_value(false)
                                .text("Zoom"),
                        );
                    });
                });
                ui.add_space(8.0);
                let header = 190.0;
                let bar_width = 84.0 * self.zoom;
                let beat_width = bar_width / 4.0;
                let row_h = 66.0;
                let total_w = header + bar_width * self.project.length_bars as f32;
                let total_h = 54.0 + row_h * self.project.tracks.len() as f32;
                let mut select = None;
                let mut moved = None;
                let mut new_clip = None;
                let mut mix = None;
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
                        painter.rect_filled(rect, 0.0, BG);
                        painter.rect_filled(
                            Rect::from_min_size(origin, Vec2::new(rect.width(), 54.0)),
                            0.0,
                            PANEL,
                        );
                        painter.text(
                            origin + Vec2::new(12.0, 27.0),
                            Align2::LEFT_CENTER,
                            "TRACKS",
                            FontId::proportional(12.0),
                            MUTED,
                        );
                        for section in &self.project.sections {
                            let x = origin.x + header + section.start_bar as f32 * bar_width;
                            let r = Rect::from_min_size(
                                Pos2::new(x, origin.y),
                                Vec2::new(section.length_bars as f32 * bar_width - 2.0, 23.0),
                            );
                            painter.rect_filled(r, 2.0, Color32::from_rgb(48, 54, 44));
                            painter.text(
                                r.left_center() + Vec2::new(9.0, 0.0),
                                Align2::LEFT_CENTER,
                                &section.name,
                                FontId::proportional(12.0),
                                ACCENT,
                            );
                        }
                        for bar in 0..=self.project.length_bars {
                            let x = origin.x + header + bar as f32 * bar_width;
                            painter.line_segment(
                                [Pos2::new(x, origin.y + 27.0), Pos2::new(x, rect.bottom())],
                                Stroke::new(1.0_f32, LINE),
                            );
                            if bar < self.project.length_bars {
                                painter.text(
                                    Pos2::new(x + 7.0, origin.y + 39.0),
                                    Align2::LEFT_CENTER,
                                    format!("{:02}", bar + 1),
                                    FontId::monospace(12.0),
                                    MUTED,
                                );
                            }
                        }
                        let mut clip_hit = false;
                        for (ti, track) in self.project.tracks.iter().enumerate() {
                            let y = origin.y + 54.0 + ti as f32 * row_h;
                            let row = Rect::from_min_size(
                                Pos2::new(origin.x, y),
                                Vec2::new(rect.width(), row_h),
                            );
                            if ti == self.selected_track {
                                painter.rect_filled(row, 0.0, Color32::from_rgb(30, 33, 39));
                            }
                            painter.line_segment(
                                [row.left_bottom(), row.right_bottom()],
                                Stroke::new(1.0_f32, LINE),
                            );
                            let c = color(track.color);
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
                                painter.rect_filled(r, 4.0, if active { ACCENT } else { LINE });
                                painter.text(
                                    r.center(),
                                    Align2::CENTER_CENTER,
                                    label,
                                    FontId::proportional(12.0),
                                    if active { BG } else { MUTED },
                                );
                                if ui
                                    .interact(r, ui.id().with(("mix", ti, j)), Sense::click())
                                    .clicked()
                                {
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
                                    4.0,
                                    c.gamma_multiply(if track.mute { 0.18 } else { 0.32 }),
                                );
                                painter.rect_filled(
                                    Rect::from_min_size(r.min, Vec2::new(r.width(), 19.0)),
                                    3.0,
                                    c.gamma_multiply(if track.mute { 0.4 } else { 0.78 }),
                                );
                                if sel {
                                    painter.rect_stroke(
                                        r,
                                        4.0,
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
                        if self.playing() {
                            let x = origin.x + header + self.position() as f32 * beat_width;
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
        egui::TopBottomPanel::bottom("editor")
            .default_height(300.0)
            .min_height(230.0)
            .max_height(520.0)
            .resizable(true)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::same(12)),
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
            if ui.button("Duplicate").clicked() {
                self.duplicate_clip();
            }
            if ui.button("Delete clip").clicked() {
                self.edit(|p| {
                    p.tracks[ti].clips.remove(ci);
                });
                self.clamp_selection();
            }
        });
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
        egui::ScrollArea::both().id_salt(("piano",&clip.id)).auto_shrink([false,false]).show(ui,|ui|{
            let (rect,response)=ui.allocate_exact_size(Vec2::new(width.max(ui.available_width()),height),Sense::click());let painter=ui.painter_at(rect);let origin=rect.min;
            for pitch in lo..=hi{
                let y=origin.y+(hi-pitch)as f32*row;let black=matches!(pitch%12,1|3|6|8|10);
                let rr=Rect::from_min_size(Pos2::new(origin.x,y),Vec2::new(rect.width(),row));
                painter.rect_filled(rr,0.0,if black{Color32::from_rgb(22,24,29)}else{Color32::from_rgb(30,33,38)});
                let kr=Rect::from_min_size(rr.min,Vec2::new(key-2.0,row-1.0));
                painter.rect_filled(kr,0.0,if black{Color32::from_rgb(42,45,51)}else{Color32::from_rgb(175,181,185)});
                if pitch%12==0||!black{painter.text(kr.center(),Align2::CENTER_CENTER,note_name(pitch),FontId::monospace(11.0),if black{TEXT}else{BG});}
            }
            for step in 0..=(clip.length/self.grid)as usize{
                let x=origin.x+key+step as f32*self.grid as f32*beat;
                let major=(step as f64*self.grid)%4.0<0.01;
                painter.line_segment([Pos2::new(x,rect.top()),Pos2::new(x,rect.bottom())],Stroke::new(1.0_f32,if major{Color32::from_rgb(76,81,88)}else{LINE}));
            }
            let mut hit=false;
            for (i,n)in clip.notes.iter().enumerate(){
                let nr=Rect::from_min_size(Pos2::new(origin.x+key+n.start as f32*beat,origin.y+(hi-n.pitch)as f32*row+1.0),Vec2::new((n.duration as f32*beat-1.0).max(3.0),row-2.0));
                painter.rect_filled(nr,3.0,tint.gamma_multiply(0.55+n.velocity*0.45));
                if nr.width()>28.0{ui.painter_at(nr).text(nr.left_center()+Vec2::new(5.0,0.0),Align2::LEFT_CENTER,note_name(n.pitch),FontId::monospace(11.0),BG);}
                let res=ui.interact(nr,ui.id().with(&n.id),Sense::click_and_drag()).on_hover_text(format!("{} · {:.2} beats · velocity {:.0}%\nDrag to move · right-click to delete",note_name(n.pitch),n.duration,n.velocity*100.0));
                if res.hovered(){hit=true;}
                if res.secondary_clicked(){remove=Some(i);}
                if res.drag_started(){self.note_drag=ui.input(|i|i.pointer.press_origin()).map(|origin|(i,n.clone(),origin));}
                if res.drag_stopped()&& let Some((index,mut original,drag_origin))=self.note_drag.take(){let delta=ui.input(|i|i.pointer.latest_pos()).unwrap_or(drag_origin)-drag_origin;let dx=delta.x as f64/beat as f64;original.start=((original.start+dx)/self.grid).round()*self.grid;original.start=original.start.clamp(0.0,(clip.length-original.duration).max(0.0));original.pitch=(original.pitch as i16-(delta.y/row).round()as i16).clamp(0,127)as u8;move_note=Some((index,original));}
            }
            if response.clicked()&&!hit&& let Some(p)=response.interact_pointer_pos()&& p.x>=origin.x+key&&p.x<origin.x+width{let start=(((p.x-origin.x-key)/beat)as f64/self.grid).floor()*self.grid;let pitch=hi.saturating_sub(((p.y-origin.y)/row)as u8);add=Some(Note{id:dawwny_core::new_id(),pitch,start,duration:self.note_length.min(clip.length-start),velocity:self.note_velocity});}
            if self.playing(){let pos=self.position()-clip.start;if pos>=0.0&&pos<clip.length{let x=origin.x+key+pos as f32*beat;painter.line_segment([Pos2::new(x,rect.top()),Pos2::new(x,rect.bottom())],Stroke::new(1.5_f32,ACCENT));}}
        });
        if let Some(n) = add {
            self.edit(|p| p.tracks[ti].clips[ci].notes.push(n));
        }
        if let Some(i) = remove {
            self.edit(|p| {
                p.tracks[ti].clips[ci].notes.remove(i);
            });
        }
        if let Some((i, n)) = move_note {
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
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("DAWN / SYNTH")
                        .size(22.0)
                        .color(color(track.color))
                        .strong(),
                );
                ui.separator();
                egui::ComboBox::from_id_salt("instrument")
                    .selected_text(instrument_name(track.instrument))
                    .show_ui(ui, |ui| {
                        for inst in [
                            Instrument::Keys,
                            Instrument::Pad,
                            Instrument::Bass,
                            Instrument::Lead,
                            Instrument::Drums,
                        ] {
                            ui.selectable_value(&mut track.instrument, inst, instrument_name(inst));
                        }
                    });
                ui.add(egui::TextEdit::singleline(&mut track.name).desired_width(160.0));
            });
            ui.add_space(12.0);
            ui.columns(3, |cols| {
                caption(&mut cols[0], "AMPLITUDE ENVELOPE");
                cols[0].add(
                    egui::Slider::new(&mut track.patch.attack, 0.001..=2.0)
                        .logarithmic(true)
                        .text("Attack / s"),
                );
                cols[0].add(
                    egui::Slider::new(&mut track.patch.decay, 0.001..=2.0)
                        .logarithmic(true)
                        .text("Decay / s"),
                );
                cols[0].add(egui::Slider::new(&mut track.patch.sustain, 0.0..=1.0).text("Sustain"));
                cols[0].add(
                    egui::Slider::new(&mut track.patch.release, 0.01..=5.0)
                        .logarithmic(true)
                        .text("Release / s"),
                );
                caption(&mut cols[1], "TONE & SPACE");
                cols[1].add(
                    egui::Slider::new(&mut track.patch.cutoff, 80.0..=18000.0)
                        .logarithmic(true)
                        .text("Cutoff / Hz"),
                );
                cols[1].add(egui::Slider::new(&mut track.patch.reverb, 0.0..=1.0).text("Reverb"));
                cols[1].add(egui::Slider::new(&mut track.patch.delay, 0.0..=1.0).text("Delay"));
                caption(&mut cols[2], "CHANNEL");
                cols[2].add(egui::Slider::new(&mut track.gain, 0.0..=1.0).text("Gain"));
                cols[2].add(egui::Slider::new(&mut track.pan, -1.0..=1.0).text("Pan"));
                cols[2].horizontal(|ui| {
                    ui.toggle_value(&mut track.mute, "Mute");
                    ui.toggle_value(&mut track.solo, "Solo");
                });
                caption(&mut cols[2], "Sound edits restart playback when saved.");
            });
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
                        .corner_radius(6.0)
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
                                    ui.toggle_value(&mut mute, "M");
                                    ui.toggle_value(&mut solo, "S");
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
                    .corner_radius(6.0)
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
