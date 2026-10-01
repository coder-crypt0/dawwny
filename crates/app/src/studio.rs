use dawwny_audio::AudioEngine;
use dawwny_core::{Clip, Instrument, Note, Project, SessionStore, SynthPatch, Track};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq)]
pub enum EditorTab {
    Piano,
    Sound,
    Mixer,
}

pub struct Studio {
    pub project: Project,
    pub store: SessionStore,
    pub audio: Option<AudioEngine>,
    pub selected_track: usize,
    pub selected_clip: usize,
    pub tab: EditorTab,
    pub editor_expanded: bool,
    pub looped: bool,
    pub zoom: f32,
    pub fit_timeline: bool,
    pub piano_zoom: f32,
    pub grid: f64,
    pub note_length: f64,
    pub note_velocity: f32,
    pub status: String,
    pub error: bool,
    pub show_agents: bool,
    pub show_library: bool,
    pub library: crate::library::LibraryState,
    pub preview: Option<AudioEngine>,
    pub show_help: bool,
    pub pending: Option<Project>,
    pub conflict: bool,
    pub undo: Vec<Project>,
    pub redo: Vec<Project>,
    last_edit: Instant,
    last_poll: Instant,
    last_modified: Option<std::time::SystemTime>,
    pub export_job: Option<mpsc::Receiver<Result<String, String>>>,
    pub clip_drag: Option<(usize, usize, f64, egui::Pos2)>,
    pub note_drag: Option<(usize, Note, egui::Pos2)>,
    pub last_external_revision: Option<u64>,
    #[cfg(feature = "capture")]
    capture_frame: u32,
}

impl Studio {
    pub fn new(path: PathBuf, no_audio: bool) -> anyhow::Result<Self> {
        let store = SessionStore::new(path);
        let project = store.initialize(&dawwny_core::demo_project())?;
        let (audio, status, error) = if no_audio {
            (None, "Audio disabled for this session".into(), false)
        } else {
            match AudioEngine::new() {
                Ok(engine) => (Some(engine), "Ready. Press Space to play.".into(), false),
                Err(e) => (
                    None,
                    format!("Audio device unavailable: {e}. Editing and export still work."),
                    true,
                ),
            }
        };
        let last_modified = std::fs::metadata(store.path())
            .and_then(|m| m.modified())
            .ok();
        let library =
            crate::library::LibraryState::new(store.path().with_file_name(".dawwny-library.json"));
        Ok(Self {
            project,
            store,
            audio,
            selected_track: 0,
            selected_clip: 0,
            tab: EditorTab::Piano,
            editor_expanded: false,
            looped: true,
            zoom: 1.0,
            fit_timeline: true,
            piano_zoom: 1.0,
            grid: 0.25,
            note_length: 0.5,
            note_velocity: 0.75,
            status,
            error,
            show_agents: false,
            show_library: true,
            library,
            preview: None,
            show_help: false,
            pending: None,
            conflict: false,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: Instant::now(),
            last_poll: Instant::now(),
            last_modified,
            export_job: None,
            clip_drag: None,
            note_drag: None,
            last_external_revision: None,
            #[cfg(feature = "capture")]
            capture_frame: 0,
        })
    }

    pub fn edit(&mut self, change: impl FnOnce(&mut Project)) {
        let mut draft = self.project.clone();
        change(&mut draft);
        if draft == self.project {
            return;
        }
        if let Err(e) = dawwny_core::validate(&draft) {
            self.fail(e.to_string());
            return;
        }
        if self.pending.is_none() {
            self.pending = Some(self.project.clone());
        }
        self.project = draft;
        self.last_edit = Instant::now();
        self.status = "Editing • autosave pending".into();
        self.error = false;
    }

    pub fn commit(&mut self) -> bool {
        if self.pending.is_none() {
            return !self.conflict;
        }
        if self.conflict {
            return false;
        }
        match self.store.replace(self.project.revision, &self.project) {
            Ok(saved) => {
                if let Some(previous) = self.pending.take() {
                    self.undo.push(previous);
                    if self.undo.len() > 32 {
                        self.undo.remove(0);
                    }
                    self.redo.clear();
                }
                self.project = saved;
                self.last_modified = std::fs::metadata(self.store.path())
                    .and_then(|m| m.modified())
                    .ok();
                self.status = format!("Saved locally • revision {}", self.project.revision);
                self.error = false;
                self.restart_playback();
                true
            }
            Err(e) => {
                self.conflict = true;
                self.fail(format!(
                    "Save failed: {e}. Save a copy or reload the session."
                ));
                false
            }
        }
    }

    pub fn fail(&mut self, message: String) {
        self.status = message;
        self.error = true;
    }
    pub fn stop(&mut self) {
        if let Some(preview) = &mut self.preview {
            preview.stop();
        }
        if let Some(a) = &mut self.audio {
            a.stop();
        }
    }
    pub fn playing(&self) -> bool {
        self.audio.as_ref().is_some_and(|a| a.is_playing())
    }
    pub fn position(&self) -> f64 {
        self.audio.as_ref().map_or(0.0, |a| a.position_beats())
    }
    pub fn toggle_play(&mut self) {
        if let Some(preview) = &mut self.preview {
            preview.stop();
        }
        if self.playing() {
            self.stop();
            return;
        }
        if !self.commit() {
            return;
        }
        if let Some(a) = &mut self.audio {
            if let Err(e) = a.play(&self.project, self.looped) {
                self.fail(format!("Playback failed: {e}"));
            }
        } else {
            self.fail(
                "No audio output. Restart with an available audio device; WAV export is available."
                    .into(),
            );
        }
    }
    fn restart_playback(&mut self) {
        if self.playing() {
            self.stop();
            self.toggle_play();
        }
    }

    pub fn history(&mut self, redo: bool) {
        if !self.commit() {
            return;
        }
        let snapshot = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(snapshot) = snapshot {
            match self.store.replace(self.project.revision, &snapshot) {
                Ok(saved) => {
                    if redo {
                        self.undo.push(self.project.clone());
                    } else {
                        self.redo.push(self.project.clone());
                    }
                    self.project = saved;
                    self.clamp_selection();
                    self.status = if redo { "Edit restored" } else { "Edit undone" }.into();
                    self.error = false;
                    self.restart_playback();
                }
                Err(e) => {
                    if redo {
                        self.redo.push(snapshot);
                    } else {
                        self.undo.push(snapshot);
                    }
                    self.fail(e.to_string());
                }
            }
        }
    }

    pub fn reload(&mut self) {
        match self.store.load() {
            Ok(p) => {
                self.stop();
                self.project = p;
                self.pending = None;
                self.conflict = false;
                self.undo.clear();
                self.redo.clear();
                self.clamp_selection();
                self.status = "Loaded current session from disk".into();
                self.error = false;
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    pub fn open(&mut self) {
        if !self.commit() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Project or MIDI", &["json", "mid", "midi"])
            .pick_file()
        {
            let imported = path.extension().is_some_and(|x| x == "mid" || x == "midi");
            let result = if imported {
                dawwny_core::import_midi(&path)
            } else {
                dawwny_core::load_project(&path)
            };
            match result {
                Ok(p) => {
                    if imported {
                        self.new_document(p);
                    } else {
                        self.stop();
                        self.project = p;
                        self.store = SessionStore::new(path);
                        self.pending = None;
                        self.conflict = false;
                        self.undo.clear();
                        self.redo.clear();
                        self.selected_track = 0;
                        self.selected_clip = 0;
                        self.status = "Session opened".into();
                        self.error = false;
                    }
                }
                Err(e) => self.fail(format!("Could not open file: {e}")),
            }
        }
    }

    pub fn new_document(&mut self, mut project: Project) {
        if !self.commit() {
            return;
        }
        project.id = dawwny_core::new_id();
        project.revision = 0;
        let parent = self
            .store
            .path()
            .parent()
            .unwrap_or(std::path::Path::new("sessions"));
        let store = SessionStore::new(parent.join(format!("{}.dawwny.json", project.id)));
        match store.initialize(&project) {
            Ok(p) => {
                self.stop();
                self.project = p;
                self.store = store;
                self.pending = None;
                self.conflict = false;
                self.undo.clear();
                self.redo.clear();
                self.selected_track = 0;
                self.selected_clip = 0;
                self.status = "New local session created".into();
                self.error = false;
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    pub fn save_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("dawwny project", &["json"])
            .set_file_name(format!("{}.dawwny.json", self.project.name))
            .save_file()
        {
            // The active session must always go through revision-checked writes.
            if path == self.store.path() {
                self.commit();
                return;
            }
            if path.exists() {
                self.fail("Choose a new filename for the copy. Open an existing session to edit it with revision protection.".into());
                return;
            }
            match dawwny_core::save_project(&path, &self.project) {
                Ok(()) => {
                    self.stop();
                    self.store = SessionStore::new(path);
                    self.pending = None;
                    self.conflict = false;
                    self.undo.clear();
                    self.redo.clear();
                    self.status = "Saved project copy".into();
                    self.error = false;
                }
                Err(e) => self.fail(e.to_string()),
            }
        }
    }

    pub fn export(&mut self, wav: bool) {
        if self.export_job.is_some() || !self.commit() {
            return;
        }
        let ext = if wav { "wav" } else { "mid" };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Export", &[ext])
            .set_file_name(format!("{}.{}", self.project.name, ext))
            .save_file()
        {
            let project = self.project.clone();
            let (sender, receiver) = mpsc::channel();
            self.export_job = Some(receiver);
            self.status = format!("Rendering {}…", ext.to_uppercase());
            self.error = false;
            std::thread::spawn(move || {
                let result = if wav {
                    dawwny_audio::render_wav(&project, &path, 48000).map(|_| ())
                } else {
                    dawwny_core::export_midi(&project, &path)
                };
                let _ = sender.send(
                    result
                        .map(|()| format!("Exported {}", path.display()))
                        .map_err(|e| e.to_string()),
                );
            });
        }
    }

    pub fn add_track(&mut self, instrument: Instrument) {
        let colors = [
            [165, 144, 230],
            [83, 180, 181],
            [223, 161, 91],
            [130, 176, 235],
            [207, 119, 151],
        ];
        let i = self.project.tracks.len();
        let track = Track {
            id: dawwny_core::new_id(),
            name: format!("{} {}", crate::views::instrument_name(instrument), i + 1),
            color: colors[i % colors.len()],
            instrument,
            gain: 0.65,
            pan: 0.0,
            mute: false,
            solo: false,
            patch: SynthPatch::default(),
            clips: vec![],
        };
        self.edit(|p| p.tracks.push(track));
        self.selected_track = i.min(self.project.tracks.len().saturating_sub(1));
        self.selected_clip = 0;
    }

    pub fn add_clip(&mut self, track: usize, start: f64) {
        let length = 4.0f64.min(self.project.length_bars as f64 * 4.0 - start);
        if length <= 0.0 {
            return;
        }
        let clip = Clip {
            id: dawwny_core::new_id(),
            name: "New phrase".into(),
            start,
            length,
            notes: vec![],
        };
        self.edit(|p| p.tracks[track].clips.push(clip));
        self.selected_track = track;
        self.selected_clip = self.project.tracks[track].clips.len().saturating_sub(1);
        self.tab = EditorTab::Piano;
    }

    pub fn duplicate_clip(&mut self) {
        let ti = self.selected_track;
        let ci = self.selected_clip;
        if let Some(mut c) = self
            .project
            .tracks
            .get(ti)
            .and_then(|t| t.clips.get(ci))
            .cloned()
        {
            c.start += c.length;
            c.id = dawwny_core::new_id();
            for n in &mut c.notes {
                n.id = dawwny_core::new_id();
            }
            self.edit(|p| p.tracks[ti].clips.push(c));
        }
    }

    pub fn clamp_selection(&mut self) {
        self.selected_track = self
            .selected_track
            .min(self.project.tracks.len().saturating_sub(1));
        self.selected_clip = self.selected_clip.min(
            self.project
                .tracks
                .get(self.selected_track)
                .map_or(0, |t| t.clips.len().saturating_sub(1)),
        );
    }

    fn housekeeping(&mut self, ctx: &egui::Context) {
        if self.audio.as_ref().is_some_and(|a| a.device_failed()) && !self.error {
            self.fail(
                "The audio output device stopped. Check the device and press Play to reconnect."
                    .into(),
            );
        }
        if self.pending.is_some()
            && !self.conflict
            && self.last_edit.elapsed() > Duration::from_millis(300)
            && !ctx.input(|i| i.pointer.any_down())
        {
            self.commit();
        }
        if self.last_poll.elapsed() > Duration::from_millis(750)
            && self.pending.is_none()
            && !ctx.input(|i| i.pointer.any_down())
        {
            self.last_poll = Instant::now();
            let modified = std::fs::metadata(self.store.path())
                .and_then(|m| m.modified())
                .ok();
            if modified != self.last_modified {
                self.last_modified = modified;
                match self.store.load() {
                    Ok(p) if p != self.project => {
                        self.last_external_revision = Some(p.revision);
                        self.project = p;
                        self.undo.clear();
                        self.redo.clear();
                        self.clip_drag = None;
                        self.note_drag = None;
                        self.clamp_selection();
                        self.status = format!(
                            "External edit received • revision {}",
                            self.project.revision
                        );
                        self.error = false;
                        self.restart_playback();
                    }
                    Err(e) => self.fail(format!("Session update rejected: {e}")),
                    _ => {}
                }
            }
        }
        if let Some(receiver) = &self.export_job {
            match receiver.try_recv() {
                Ok(Ok(message)) => {
                    self.status = message;
                    self.error = false;
                    self.export_job = None;
                }
                Ok(Err(error)) => {
                    self.fail(error);
                    self.export_job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.fail("Export worker stopped unexpectedly".into());
                    self.export_job = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        ctx.request_repaint_after(if self.playing() {
            Duration::from_millis(33)
        } else if self.pending.is_some() || self.export_job.is_some() {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(750)
        });
    }
}

impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.housekeeping(ctx);
        #[cfg(feature = "capture")]
        if let Ok(path) = std::env::var("DAWWNY_SCREENSHOT_PATH") {
            self.capture_frame += 1;
            ctx.request_repaint_after(Duration::from_millis(50));
            if self.capture_frame == 20 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let capture = ctx.input(|i| {
                i.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(capture) = capture {
                let bytes: Vec<u8> = capture
                    .pixels
                    .iter()
                    .flat_map(|color| color.to_array())
                    .collect();
                match image::save_buffer(
                    path,
                    &bytes,
                    capture.size[0] as u32,
                    capture.size[1] as u32,
                    image::ColorType::Rgba8,
                ) {
                    Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    Err(e) => self.fail(format!("Screenshot failed: {e}")),
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.export_job.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.fail("An export is still running. Close the studio after it finishes.".into());
            } else if !self.commit() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::L)) {
            self.show_library = !self.show_library;
        }
        if self.preview.as_ref().is_some_and(|p| p.is_playing()) {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.toggle_play();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S)) {
            self.commit();
        }
        if !ctx.wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z))
        {
            self.history(false);
        }
        if !ctx.wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Y))
        {
            self.history(true);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            self.open();
        }
        self.header(ctx);
        self.footer(ctx);
        if self.show_agents {
            self.agent_panel(ctx);
        }
        if self.show_library {
            self.library_panel(ctx);
        }
        self.editor(ctx);
        if !self.editor_expanded {
            self.arrangement(ctx);
        }
        if self.show_help {
            egui::Window::new("Studio guide").open(&mut self.show_help).resizable(false).show(ctx,|ui|{
                ui.label("Space — play / stop     Ctrl+S — save     Ctrl+Z / Ctrl+Y — undo / redo");
                ui.label("Double-click an empty track lane to create a clip. Drag a clip to move it.");
                ui.label("Select a clip, then click the piano grid to add a note. Drag notes to move them.");
                ui.label("Right-click a note to edit its length, velocity, pitch, or delete it. Sound controls edit the selected track.");
                ui.label("Edits autosave locally. Playback restarts when a committed edit changes the sound.");
                ui.separator();
                ui.label("Foundation: fixed 4/4, built-in instruments and MIDI clips. Audio recording, VST3 hosting,");
                ui.label("automation, hardware MIDI, and remote access are planned. No AI model is bundled.");
            });
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.commit();
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_edits_undo_redo_and_agent_conflicts_preserve_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Studio::new(dir.path().join("session.json"), true).unwrap();
        app.edit(|p| p.tempo = 100.0);
        assert!(app.commit());
        app.history(false);
        assert_eq!(app.project.tempo, 92.0);
        app.history(true);
        assert_eq!(app.project.tempo, 100.0);
        app.edit(|p| p.name = "Unsaved local idea".into());
        app.store
            .transact(
                app.project.revision,
                &[dawwny_core::Command::SetTempo { tempo: 108.0 }],
            )
            .unwrap();
        assert!(!app.commit());
        assert!(app.conflict);
        assert_eq!(app.project.name, "Unsaved local idea");
        assert_eq!(app.store.load().unwrap().tempo, 108.0);
        app.reload();
        assert_eq!(app.project.tempo, 108.0);
        assert!(!app.conflict);
    }
    #[test]
    fn editor_renders_each_tab_for_empty_and_populated_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Studio::new(dir.path().join("session.json"), true).unwrap();
        let ctx = egui::Context::default();
        crate::views::theme(&ctx);
        app.load_sound("factory.dusk-wash.10");
        for width in [940.0, 1440.0, 1920.0] {
            for tab in [EditorTab::Piano, EditorTab::Sound, EditorTab::Mixer] {
                app.tab = tab;
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 850.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        app.header(ctx);
                        app.footer(ctx);
                        app.library_panel(ctx);
                        app.editor(ctx);
                        app.arrangement(ctx);
                    },
                );
                assert!(!output.shapes.is_empty());
            }
        }
        app.new_document(Project::default());
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.header(ctx);
            app.editor(ctx);
            app.arrangement(ctx);
        });
        app.add_track(Instrument::Keys);
        app.add_clip(0, 0.0);
        assert!(app.commit());
        assert_eq!(app.store.load().unwrap().tracks[0].clips.len(), 1);
    }
}
