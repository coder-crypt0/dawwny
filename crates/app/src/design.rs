use eframe::egui::{self, Color32};

pub const BG: Color32 = Color32::from_rgb(21, 23, 27);
pub const PANEL: Color32 = Color32::from_rgb(28, 30, 35);
pub const SURFACE: Color32 = Color32::from_rgb(30, 33, 40);
pub const CONTROL: Color32 = Color32::from_rgb(39, 42, 48);
pub const HOVER: Color32 = Color32::from_rgb(58, 64, 70);
pub const SELECTED: Color32 = Color32::from_rgb(63, 75, 47);
pub const LINE: Color32 = Color32::from_rgb(47, 50, 57);
pub const TEXT: Color32 = Color32::from_rgb(226, 230, 232);
pub const MUTED: Color32 = Color32::from_rgb(140, 148, 159);
pub const ACCENT: Color32 = Color32::from_rgb(203, 231, 143);
pub const CYCLE: Color32 = Color32::from_rgb(230, 184, 99);
pub const ERROR: Color32 = Color32::from_rgb(255, 169, 144);

// Roles stay consistent across the studio; compact musical marks need less rounding.
pub const CONTROL_RADIUS: u8 = 8;
pub const SURFACE_RADIUS: u8 = 12;
pub const WINDOW_RADIUS: u8 = 14;
pub const MEDIA_RADIUS: u8 = 6;

pub fn mix_toggle(ui: &mut egui::Ui, value: &mut bool, label: &str, mute: bool) {
    let active = if mute { CYCLE } else { ACCENT };
    let response = ui
        .add(
            egui::Button::new(egui::RichText::new(label).color(if *value { BG } else { MUTED }))
                .fill(if *value { active } else { CONTROL })
                .corner_radius(CONTROL_RADIUS),
        )
        .on_hover_text(if mute { "Mute track" } else { "Solo track" });
    if response.clicked() {
        *value = !*value;
    }
}
