use dawwny_core::{
    Effect, EffectSlot, FilterMode, Instrument, Oscillator, SynthPatch, Track, Waveform,
};
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

const ACCENT: Color32 = Color32::from_rgb(203, 231, 143);
fn slider(ui: &mut egui::Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>, name: &str) {
    ui.add(egui::Slider::new(value, range).text(name));
}
fn logarithmic(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    name: &str,
) {
    ui.add(egui::Slider::new(value, range).logarithmic(true).text(name));
}
fn caption(ui: &mut egui::Ui, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .small()
            .color(Color32::from_gray(155)),
    );
}
fn wave_name(w: Waveform) -> &'static str {
    match w {
        Waveform::Sine => "Sine",
        Waveform::Triangle => "Triangle",
        Waveform::Saw => "Saw",
        Waveform::Pulse => "Pulse",
    }
}

pub fn show(ui: &mut egui::Ui, track: &mut Track) {
    let track_id = track.id.clone();
    ui.push_id(track_id, |ui| {
        ui.spacing_mut().slider_width = 105.0;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(if track.instrument == Instrument::Synth {
                    "DAWN"
                } else {
                    "INSTRUMENT"
                })
                .size(23.0)
                .color(ACCENT),
            );
            egui::ComboBox::from_id_salt("instrument")
                .selected_text(crate::views::instrument_name(track.instrument))
                .show_ui(ui, |ui| {
                    for i in [
                        Instrument::Synth,
                        Instrument::Keys,
                        Instrument::Pad,
                        Instrument::Bass,
                        Instrument::Lead,
                        Instrument::Drums,
                    ] {
                        ui.selectable_value(
                            &mut track.instrument,
                            i,
                            crate::views::instrument_name(i),
                        );
                    }
                });
            ui.add(egui::TextEdit::singleline(&mut track.name).desired_width(135.0));
            egui::ComboBox::from_id_salt("presets")
                .selected_text("Signatures…")
                .show_ui(ui, |ui| {
                    for p in dawwny_core::sound_presets() {
                        if ui.button(&p.name).on_hover_text(&p.description).clicked() {
                            track.instrument = Instrument::Synth;
                            track.patch = p.patch;
                            ui.close();
                        }
                    }
                });
            if ui
                .button("Copy patch")
                .on_hover_text(
                    "Copy these exact settings as JSON for your agent or another session.",
                )
                .clicked()
                && let Ok(json) = serde_json::to_string_pretty(&track.patch)
            {
                ui.ctx().copy_text(json);
            }
        });
        let id = ui.id().with("sound_page");
        let mut effects = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut effects, false, "Synthesis");
            ui.selectable_value(
                &mut effects,
                true,
                format!("Effects / {}", track.patch.effects.len()),
            );
            ui.separator();
            caption(ui, "Edits autosave. Active playback restarts when saved.");
        });
        ui.data_mut(|d| d.insert_temp(id, effects));
        ui.separator();
        if effects {
            rack(ui, &mut track.patch);
        } else {
            // At small widths each block receives its own row, so labels never overlap.
            if ui.available_width() >= 840.0 {
                ui.columns(3, |cols| {
                    oscillators(&mut cols[0], track);
                    envelope_filter(&mut cols[1], track);
                    modulation_mix(&mut cols[2], track);
                });
            } else {
                oscillators(ui, track);
                ui.separator();
                envelope_filter(ui, track);
                ui.separator();
                modulation_mix(ui, track);
            }
        }
    });
}
fn oscillators(ui: &mut egui::Ui, track: &mut Track) {
    if track.instrument != Instrument::Synth {
        caption(ui, "BUILT-IN VOICE");
        ui.label(crate::views::instrument_name(track.instrument));
        ui.label("Use Dawn for editable oscillators, tuning, noise and modulation. Effects work on every instrument.");
        if ui.button("Switch to Dawn").clicked() {
            track.instrument = Instrument::Synth;
        }
        return;
    }
    ui.push_id("osc1", |ui| {
        oscillator(ui, "OSCILLATOR 01", &mut track.patch.synth.osc1)
    });
    ui.add_space(8.0);
    ui.push_id("osc2", |ui| {
        oscillator(ui, "OSCILLATOR 02", &mut track.patch.synth.osc2)
    });
    slider(
        ui,
        &mut track.patch.synth.sub_level,
        0.0..=1.0,
        "Sub / −1 octave",
    );
    slider(ui, &mut track.patch.synth.noise_level, 0.0..=1.0, "Noise");
}
fn oscillator(ui: &mut egui::Ui, name: &str, oscillator: &mut Oscillator) {
    caption(ui, name);
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("waveform")
            .width(90.0)
            .selected_text(wave_name(oscillator.waveform))
            .show_ui(ui, |ui| {
                for w in [
                    Waveform::Sine,
                    Waveform::Triangle,
                    Waveform::Saw,
                    Waveform::Pulse,
                ] {
                    ui.selectable_value(&mut oscillator.waveform, w, wave_name(w));
                }
            });
        let (r, _) = ui.allocate_exact_size(Vec2::new(90.0, 27.0), egui::Sense::hover());
        let points = (0..=64)
            .map(|i| {
                let p = i as f32 / 64.0;
                let y = match oscillator.waveform {
                    Waveform::Sine => (std::f32::consts::TAU * p).sin(),
                    Waveform::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
                    Waveform::Saw => 2.0 * p - 1.0,
                    Waveform::Pulse => {
                        if p < oscillator.pulse_width {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                };
                Pos2::new(r.left() + p * r.width(), r.center().y - y * 10.0)
            })
            .collect();
        ui.painter()
            .add(egui::Shape::line(points, Stroke::new(1.3_f32, ACCENT)));
    });
    slider(ui, &mut oscillator.level, 0.0..=1.0, "Level");
    ui.horizontal(|ui| {
        ui.add(
            egui::DragValue::new(&mut oscillator.semitones)
                .range(-24..=24)
                .suffix(" st"),
        );
        ui.add(
            egui::DragValue::new(&mut oscillator.detune_cents)
                .range(-100.0..=100.0)
                .speed(0.25)
                .suffix(" ct"),
        );
    });
    if oscillator.waveform == Waveform::Pulse {
        slider(ui, &mut oscillator.pulse_width, 0.05..=0.95, "Pulse width");
    }
}
fn envelope_filter(ui: &mut egui::Ui, track: &mut Track) {
    let p = &mut track.patch;
    caption(ui, "AMPLITUDE ENVELOPE");
    let (r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(300.0), 48.0),
        egui::Sense::hover(),
    );
    let duration = p.attack + p.decay + 0.5 + p.release;
    let point = |time: f32, level: f32| {
        Pos2::new(
            r.left() + r.width() * time / duration,
            r.bottom() - 4.0 - level * (r.height() - 8.0),
        )
    };
    ui.painter().add(egui::Shape::line(
        vec![
            point(0.0, 0.0),
            point(p.attack, 1.0),
            point(p.attack + p.decay, p.sustain),
            point(p.attack + p.decay + 0.5, p.sustain),
            point(duration, 0.0),
        ],
        Stroke::new(1.5_f32, ACCENT),
    ));
    logarithmic(ui, &mut p.attack, 0.001..=5.0, "Attack / s");
    logarithmic(ui, &mut p.decay, 0.001..=5.0, "Decay / s");
    slider(ui, &mut p.sustain, 0.0..=1.0, "Sustain");
    logarithmic(ui, &mut p.release, 0.001..=5.0, "Release / s");
    ui.add_space(8.0);
    caption(ui, "FILTER");
    logarithmic(ui, &mut p.cutoff, 20.0..=20000.0, "Cutoff / Hz");
    if track.instrument == Instrument::Synth {
        egui::ComboBox::from_id_salt("filter_mode")
            .selected_text(match p.synth.filter_mode {
                FilterMode::LowPass => "Low pass",
                FilterMode::HighPass => "High pass",
                FilterMode::BandPass => "Band pass",
            })
            .show_ui(ui, |ui| {
                for (mode, label) in [
                    (FilterMode::LowPass, "Low pass"),
                    (FilterMode::HighPass, "High pass"),
                    (FilterMode::BandPass, "Band pass"),
                ] {
                    ui.selectable_value(&mut p.synth.filter_mode, mode, label);
                }
            });
        slider(ui, &mut p.synth.resonance, 0.0..=0.9, "Resonance");
        slider(ui, &mut p.synth.filter_env, -4.0..=4.0, "Envelope / oct");
    }
}
fn modulation_mix(ui: &mut egui::Ui, track: &mut Track) {
    if track.instrument == Instrument::Synth {
        caption(ui, "MODULATION / SINE LFO");
        logarithmic(
            ui,
            &mut track.patch.synth.lfo_rate,
            0.05..=20.0,
            "Rate / Hz",
        );
        slider(
            ui,
            &mut track.patch.synth.lfo_pitch,
            0.0..=2.0,
            "Pitch / st",
        );
        slider(
            ui,
            &mut track.patch.synth.lfo_filter,
            0.0..=3.0,
            "Filter / oct",
        );
        ui.add_space(12.0);
    }
    caption(ui, "CHANNEL");
    slider(ui, &mut track.gain, 0.0..=1.0, "Gain");
    slider(ui, &mut track.pan, -1.0..=1.0, "Pan");
    ui.horizontal(|ui| {
        ui.toggle_value(&mut track.mute, "Mute");
        ui.toggle_value(&mut track.solo, "Solo");
    });
    ui.add_space(12.0);
    ui.collapsing("Legacy ambience",|ui| {
        ui.small("Original session ambience, before the effects rack. Set to zero to use only rack effects.");
        slider(ui,&mut track.patch.reverb,0.0..=1.0,"Reverb");
        slider(ui,&mut track.patch.delay,0.0..=1.0,"Delay");
    });
}
fn effect_name(effect: &Effect) -> &'static str {
    match effect {
        Effect::Reverb { .. } => "Space / reverb",
        Effect::Echo { .. } => "Echo / tempo delay",
        Effect::Chorus { .. } => "Drift / chorus",
        Effect::Drive { .. } => "Heat / drive",
        Effect::Equalizer { .. } => "Tone / 3-band EQ",
        Effect::Compressor { .. } => "Glue / compressor",
    }
}
fn defaults() -> [Effect; 6] {
    [
        Effect::Reverb {
            size: 0.5,
            decay: 2.0,
            damping: 0.45,
            mix: 0.25,
        },
        Effect::Echo {
            beats: 0.75,
            feedback: 0.35,
            damping: 0.3,
            ping_pong: true,
            mix: 0.25,
        },
        Effect::Chorus {
            rate: 0.4,
            depth: 0.5,
            mix: 0.3,
        },
        Effect::Drive {
            drive: 2.0,
            tone: 6000.0,
            mix: 0.3,
        },
        Effect::Equalizer {
            low_db: 0.0,
            mid_db: 0.0,
            high_db: 0.0,
        },
        Effect::Compressor {
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 15.0,
            release_ms: 150.0,
            makeup_db: 0.0,
            mix: 1.0,
        },
    ]
}
fn rack(ui: &mut egui::Ui, p: &mut SynthPatch) {
    ui.horizontal(|ui| {
        caption(ui, "SIGNAL FLOW / TOP TO BOTTOM");
        ui.add_enabled_ui(p.effects.len() < 8, |ui| {
            ui.menu_button("+ Add effect", |ui| {
                for effect in defaults() {
                    if ui.button(effect_name(&effect)).clicked() {
                        p.effects.push(EffectSlot {
                            enabled: true,
                            effect,
                        });
                        ui.close();
                    }
                }
            });
        });
        caption(ui, &format!("{} / 8 slots", p.effects.len()));
    });
    if p.effects.is_empty() {
        ui.add_space(15.0);
        ui.label("An empty rack passes the sound through unchanged.");
        ui.label("Add Space for depth, Echo for repeats, or shape the tone with EQ and Drive.");
    }
    let count = p.effects.len();
    let mut remove = None;
    let mut swap = None;
    for (index, slot) in p.effects.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut slot.enabled, "")
                    .on_hover_text("Bypass this effect");
                ui.label(
                    egui::RichText::new(format!("{:02}  {}", index + 1, effect_name(&slot.effect)))
                        .color(ACCENT)
                        .strong(),
                );
                if ui
                    .add_enabled(index > 0, egui::Button::new("↑"))
                    .on_hover_text("Move earlier")
                    .clicked()
                {
                    swap = Some((index, index - 1));
                }
                if ui
                    .add_enabled(index + 1 < count, egui::Button::new("↓"))
                    .on_hover_text("Move later")
                    .clicked()
                {
                    swap = Some((index, index + 1));
                }
                if ui.button("Remove").clicked() {
                    remove = Some(index);
                }
            });
            ui.add_enabled_ui(slot.enabled, |ui| {
                egui::Grid::new("parameters")
                    .num_columns(if ui.available_width() > 650.0 { 2 } else { 1 })
                    .spacing([28.0, 8.0])
                    .show(ui, |ui| effect_controls(ui, &mut slot.effect));
            });
        });
    }
    if let Some(index) = remove {
        p.effects.remove(index);
    } else if let Some((a, b)) = swap {
        p.effects.swap(a, b);
    }
}
fn effect_controls(ui: &mut egui::Ui, e: &mut Effect) {
    // Parameter rows pair related controls; wrapping is handled by the scroll viewport.
    match e {
        Effect::Reverb {
            size,
            decay,
            damping,
            mix,
        } => {
            slider(ui, size, 0.0..=1.0, "Room size");
            logarithmic(ui, decay, 0.2..=8.0, "Decay / s");
            ui.end_row();
            slider(ui, damping, 0.0..=1.0, "Damping");
            slider(ui, mix, 0.0..=1.0, "Wet / dry");
        }
        Effect::Echo {
            beats,
            feedback,
            damping,
            ping_pong,
            mix,
        } => {
            slider(ui, beats, 0.125..=4.0, "Time / beats");
            slider(ui, feedback, 0.0..=0.85, "Feedback");
            ui.end_row();
            slider(ui, damping, 0.0..=1.0, "Damping");
            slider(ui, mix, 0.0..=1.0, "Wet / dry");
            ui.end_row();
            ui.checkbox(ping_pong, "Ping-pong stereo");
        }
        Effect::Chorus { rate, depth, mix } => {
            logarithmic(ui, rate, 0.05..=5.0, "Rate / Hz");
            slider(ui, depth, 0.0..=1.0, "Depth");
            ui.end_row();
            slider(ui, mix, 0.0..=1.0, "Wet / dry");
        }
        Effect::Drive { drive, tone, mix } => {
            logarithmic(ui, drive, 1.0..=20.0, "Drive");
            logarithmic(ui, tone, 200.0..=20000.0, "Tone / Hz");
            ui.end_row();
            slider(ui, mix, 0.0..=1.0, "Wet / dry");
        }
        Effect::Equalizer {
            low_db,
            mid_db,
            high_db,
        } => {
            slider(ui, low_db, -18.0..=18.0, "180 Hz / dB");
            slider(ui, mid_db, -18.0..=18.0, "1 kHz / dB");
            ui.end_row();
            slider(ui, high_db, -18.0..=18.0, "4 kHz / dB");
        }
        Effect::Compressor {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            makeup_db,
            mix,
        } => {
            slider(ui, threshold_db, -60.0..=0.0, "Threshold / dB");
            slider(ui, ratio, 1.0..=20.0, "Ratio");
            ui.end_row();
            logarithmic(ui, attack_ms, 1.0..=100.0, "Attack / ms");
            logarithmic(ui, release_ms, 10.0..=2000.0, "Release / ms");
            ui.end_row();
            slider(ui, makeup_db, 0.0..=24.0, "Makeup / dB");
            slider(ui, mix, 0.0..=1.0, "Wet / dry");
        }
    }
}
