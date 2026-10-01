use crate::studio::Studio;
use dawwny_audio::{AudioEngine, LiveEvent, MidiPort};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};

const KEYS: [(egui::Key, u8, &str); 16] = [
    (egui::Key::A, 0, "A"),
    (egui::Key::W, 1, "W"),
    (egui::Key::S, 2, "S"),
    (egui::Key::E, 3, "E"),
    (egui::Key::D, 4, "D"),
    (egui::Key::F, 5, "F"),
    (egui::Key::T, 6, "T"),
    (egui::Key::G, 7, "G"),
    (egui::Key::Y, 8, "Y"),
    (egui::Key::H, 9, "H"),
    (egui::Key::U, 10, "U"),
    (egui::Key::J, 11, "J"),
    (egui::Key::K, 12, "K"),
    (egui::Key::O, 13, "O"),
    (egui::Key::L, 14, "L"),
    (egui::Key::P, 15, "P"),
];
pub struct KeyboardState {
    pub open: bool,
    pub octave: u8,
    pub velocity: f32,
    pub held: [Option<u8>; 16],
    mouse: Option<u8>,
    ports: Vec<MidiPort>,
    error: Option<String>,
}
impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            open: false,
            octave: 4,
            velocity: 0.75,
            held: [None; 16],
            mouse: None,
            ports: Vec::new(),
            error: None,
        }
    }
}
impl Studio {
    fn keyboard_note(&mut self, event: LiveEvent) {
        self.sync_audio();
        let result = self.audio.as_mut().map(|a| a.live_event(event));
        match result {
            Some(Err(e)) => self.fail(format!("Keyboard input failed: {e}")),
            None => self.fail("Keyboard playback needs an available audio device".into()),
            _ => {}
        }
    }
    fn release_typing(&mut self) {
        for i in 0..16 {
            if let Some(pitch) = self.keyboard.held[i].take() {
                self.keyboard_note(LiveEvent::NoteOff { channel: 16, pitch });
            }
        }
        if let Some(pitch) = self.keyboard.mouse.take() {
            self.keyboard_note(LiveEvent::NoteOff { channel: 16, pitch });
        }
    }
    pub fn keyboard_input(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::K)) {
            self.keyboard.open = !self.keyboard.open;
        }
        if !self.keyboard.open || ctx.wants_keyboard_input() || !ctx.input(|i| i.focused) {
            self.release_typing();
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Z) && i.modifiers.is_none()) {
            self.keyboard.octave = self.keyboard.octave.saturating_sub(1).max(1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::X) && i.modifiers.is_none()) {
            self.keyboard.octave = (self.keyboard.octave + 1).min(7);
        }
        for (index, (key, offset, _)) in KEYS.iter().enumerate() {
            if ctx.input(|i| i.key_pressed(*key) && i.modifiers.is_none())
                && self.keyboard.held[index].is_none()
            {
                let pitch = (self.keyboard.octave + 1) * 12 + offset;
                self.keyboard.held[index] = Some(pitch);
                self.keyboard_note(LiveEvent::NoteOn {
                    channel: 16,
                    pitch,
                    velocity: self.keyboard.velocity,
                });
            }
            if !ctx.input(|i| i.key_down(*key))
                && let Some(pitch) = self.keyboard.held[index].take()
            {
                self.keyboard_note(LiveEvent::NoteOff { channel: 16, pitch });
            }
        }
    }
    pub fn keyboard_window(&mut self, ctx: &egui::Context) {
        if !self.keyboard.open {
            return;
        }
        let mut open = self.keyboard.open;
        egui::Window::new("Musical typing").open(&mut open).resizable(false).default_width(580.0)
            .default_pos(Pos2::new(440.0,150.0)).show(ctx,|ui| {
                ui.horizontal(|ui| {
                    let name=self.project.tracks.get(self.selected_track).map_or("Select a track",|t|t.name.as_str());
                    ui.label(egui::RichText::new(name).strong());
                    if ui.button("Panic").on_hover_text("Release all keyboard and MIDI notes").clicked() {
                        self.release_typing();if let Some(a)=&mut self.audio {a.all_notes_off();}
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.keyboard.octave).range(1..=7).prefix("Octave "));
                    ui.add(egui::Slider::new(&mut self.keyboard.velocity,0.05..=1.0).text("Velocity"));
                    ui.label("Z / X shift octaves");
                });
                ui.horizontal(|ui| {
                    let current=self.audio.as_ref().and_then(|a|a.midi_name()).unwrap_or("No MIDI input").to_owned();
                    ui.menu_button(current,|ui| {
                        if ui.button("Disconnect").clicked() {if let Some(a)=&mut self.audio {a.disconnect_midi();}ui.close();}
                        let mut selected=None;
                        for port in &self.keyboard.ports {if ui.button(&port.name).clicked(){selected=Some(port.index);}}
                        if let Some(index)=selected {
                            self.sync_audio();
                            if let Some(a)=&mut self.audio && let Err(e)=a.connect_midi(index) {self.keyboard.error=Some(e.to_string());}
                            ui.close();
                        }
                    });
                    if ui.button("Refresh MIDI devices").clicked() {
                        match AudioEngine::midi_ports() {Ok(ports)=>{self.keyboard.ports=ports;self.keyboard.error=None;},Err(e)=>self.keyboard.error=Some(e.to_string())}
                    }
                });
                if let Some(error)=&self.keyboard.error {ui.colored_label(Color32::LIGHT_RED,error);}
                let base=(self.keyboard.octave+1)*12;
                let (rect,_)=ui.allocate_exact_size(Vec2::new(560.0,100.0),Sense::hover());
                let white_width=rect.width()/10.0;
                let mut mouse_pitch=None;
                let white_offsets=[0,2,4,5,7,9,11,12,14,16];
                let pointer=ui.input(|i|i.pointer.hover_pos());
                for (i,offset) in white_offsets.iter().enumerate() {
                    let r=Rect::from_min_size(rect.min+Vec2::new(i as f32*white_width,0.0),Vec2::new(white_width-2.0,100.0));
                    let pitch=base+offset;
                    let down=self.keyboard.held.contains(&Some(pitch)) || self.keyboard.mouse==Some(pitch);
                    ui.painter().rect_filled(r,7,if down {Color32::from_rgb(203,231,143)}else{Color32::from_rgb(204,209,212)});
                    let label=KEYS.iter().find(|(_,n,_)|n==offset).map_or("",|(_,_,label)|*label);
                    ui.painter().text(r.center_bottom()-Vec2::new(0.0,15.0),egui::Align2::CENTER_CENTER,label,egui::FontId::proportional(14.0),Color32::from_gray(35));
                    if pointer.is_some_and(|p|r.contains(p)){mouse_pitch=Some(pitch);}
                }
                for (offset,position) in [(1,1),(3,2),(6,4),(8,5),(10,6),(13,8),(15,9)] {
                    let r=Rect::from_min_size(rect.min+Vec2::new(position as f32*white_width-white_width*0.31,0.0),Vec2::new(white_width*0.6,63.0));
                    let pitch=base+offset;
                    let down=self.keyboard.held.contains(&Some(pitch)) || self.keyboard.mouse==Some(pitch);
                    ui.painter().rect_filled(r,5,if down {Color32::from_rgb(100,121,65)}else{Color32::from_rgb(30,33,40)});
                    let label=KEYS.iter().find(|(_,n,_)|*n==offset).map_or("",|(_,_,label)|*label);
                    ui.painter().text(r.center_bottom()-Vec2::new(0.0,14.0),egui::Align2::CENTER_CENTER,label,egui::FontId::proportional(12.0),Color32::WHITE);
                    if pointer.is_some_and(|p|r.contains(p)){mouse_pitch=Some(pitch);}
                }
                if !ui.input(|i|i.pointer.primary_down()) || !ui.rect_contains_pointer(rect) {mouse_pitch=None;}
                if mouse_pitch!=self.keyboard.mouse {
                    if let Some(pitch)=self.keyboard.mouse.take(){self.keyboard_note(LiveEvent::NoteOff {channel:16,pitch});}
                    if let Some(pitch)=mouse_pitch {self.keyboard_note(LiveEvent::NoteOn {channel:16,pitch,velocity:self.keyboard.velocity});}
                    self.keyboard.mouse=mouse_pitch;
                }
                ui.label(egui::RichText::new("Play A–L with the upper keys for sharps. Input plays the selected track without changing its MIDI clips.").small().weak());
            });
        self.keyboard.open = open;
        if !open {
            self.release_typing();
        }
    }
}
