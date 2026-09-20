//! Native sound-design parameters. All values are validated before compiling DSP.
use crate::SynthPatch;
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Waveform {
    Sine,
    Triangle,
    Saw,
    Pulse,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilterMode {
    LowPass,
    HighPass,
    BandPass,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Oscillator {
    pub waveform: Waveform,
    /// Linear amplitude, 0–1.
    pub level: f32,
    /// Pitch offset, -24–24 semitones.
    pub semitones: i32,
    /// Fine tuning, -100–100 cents.
    pub detune_cents: f32,
    /// Duty cycle for the pulse waveform, 0.05–0.95.
    pub pulse_width: f32,
}
impl Default for Oscillator {
    fn default() -> Self {
        Self {
            waveform: Waveform::Saw,
            level: 0.6,
            semitones: 0,
            detune_cents: 0.0,
            pulse_width: 0.5,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CustomSynth {
    pub osc1: Oscillator,
    pub osc2: Oscillator,
    pub sub_level: f32,
    pub noise_level: f32,
    pub filter_mode: FilterMode,
    /// Resonance amount, 0–0.9. Cutoff lives in the parent patch.
    pub resonance: f32,
    /// Amplitude envelope routed to cutoff, -4–4 octaves.
    pub filter_env: f32,
    /// Per-voice sine LFO frequency, 0.05–20 Hz.
    pub lfo_rate: f32,
    /// Pitch modulation depth, 0–2 semitones.
    pub lfo_pitch: f32,
    /// Cutoff modulation depth, 0–3 octaves.
    pub lfo_filter: f32,
}
impl Default for CustomSynth {
    fn default() -> Self {
        Self {
            osc1: Oscillator::default(),
            osc2: Oscillator {
                level: 0.35,
                detune_cents: 7.0,
                ..Default::default()
            },
            sub_level: 0.1,
            noise_level: 0.0,
            filter_mode: FilterMode::LowPass,
            resonance: 0.15,
            filter_env: 1.0,
            lfo_rate: 0.4,
            lfo_pitch: 0.0,
            lfo_filter: 0.0,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    /// Diffuse stereo algorithmic reverb. Decay is the approximate RT60 in seconds.
    Reverb {
        size: f32,
        decay: f32,
        damping: f32,
        mix: f32,
    },
    /// Tempo-synchronized echo; beats are quarter notes. Feedback is at most 0.85.
    Echo {
        beats: f32,
        feedback: f32,
        damping: f32,
        ping_pong: bool,
        mix: f32,
    },
    /// Stereo modulated delay, rate in Hz.
    Chorus { rate: f32, depth: f32, mix: f32 },
    /// Soft saturation with a low-pass tone control in Hz.
    Drive { drive: f32, tone: f32, mix: f32 },
    /// Low shelf at 180 Hz, mid bell at 1 kHz, high shelf at 4 kHz; gains in dB.
    Equalizer {
        low_db: f32,
        mid_db: f32,
        high_db: f32,
    },
    /// Stereo-linked peak compressor. Times in ms, level controls in dB.
    Compressor {
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_db: f32,
        mix: f32,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EffectSlot {
    pub enabled: bool,
    pub effect: Effect,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SoundPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub patch: SynthPatch,
}

pub(crate) fn validate_sound(p: &SynthPatch) -> Result<()> {
    fn check(v: f32, lo: f32, hi: f32, name: &str) -> Result<()> {
        ensure!(
            v.is_finite() && (lo..=hi).contains(&v),
            "{name} must be finite and between {lo} and {hi}"
        );
        Ok(())
    }
    for o in [&p.synth.osc1, &p.synth.osc2] {
        check(o.level, 0.0, 1.0, "Oscillator level")?;
        ensure!(
            (-24..=24).contains(&o.semitones),
            "Oscillator semitones must be -24–24"
        );
        check(o.detune_cents, -100.0, 100.0, "Detune cents")?;
        check(o.pulse_width, 0.05, 0.95, "Pulse width")?;
    }
    let s = &p.synth;
    for (v, lo, hi, name) in [
        (s.sub_level, 0.0, 1.0, "Sub level"),
        (s.noise_level, 0.0, 1.0, "Noise level"),
        (s.resonance, 0.0, 0.9, "Resonance"),
        (s.filter_env, -4.0, 4.0, "Filter envelope"),
        (s.lfo_rate, 0.05, 20.0, "LFO rate"),
        (s.lfo_pitch, 0.0, 2.0, "LFO pitch"),
        (s.lfo_filter, 0.0, 3.0, "LFO filter"),
    ] {
        check(v, lo, hi, name)?;
    }
    ensure!(p.effects.len() <= 8, "At most eight effect slots per track");
    for slot in &p.effects {
        match slot.effect {
            Effect::Reverb {
                size,
                decay,
                damping,
                mix,
            } => {
                check(size, 0.0, 1.0, "Reverb size")?;
                check(decay, 0.2, 8.0, "Reverb decay")?;
                check(damping, 0.0, 1.0, "Reverb damping")?;
                check(mix, 0.0, 1.0, "Reverb mix")?;
            }
            Effect::Echo {
                beats,
                feedback,
                damping,
                mix,
                ..
            } => {
                check(beats, 0.125, 4.0, "Echo beats")?;
                check(feedback, 0.0, 0.85, "Echo feedback")?;
                check(damping, 0.0, 1.0, "Echo damping")?;
                check(mix, 0.0, 1.0, "Echo mix")?;
            }
            Effect::Chorus { rate, depth, mix } => {
                check(rate, 0.05, 5.0, "Chorus rate")?;
                check(depth, 0.0, 1.0, "Chorus depth")?;
                check(mix, 0.0, 1.0, "Chorus mix")?;
            }
            Effect::Drive { drive, tone, mix } => {
                check(drive, 1.0, 20.0, "Drive")?;
                check(tone, 200.0, 20000.0, "Drive tone")?;
                check(mix, 0.0, 1.0, "Drive mix")?;
            }
            Effect::Equalizer {
                low_db,
                mid_db,
                high_db,
            } => {
                for v in [low_db, mid_db, high_db] {
                    check(v, -18.0, 18.0, "EQ gain")?;
                }
            }
            Effect::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                makeup_db,
                mix,
            } => {
                check(threshold_db, -60.0, 0.0, "Compressor threshold")?;
                check(ratio, 1.0, 20.0, "Compressor ratio")?;
                check(attack_ms, 1.0, 100.0, "Compressor attack")?;
                check(release_ms, 10.0, 2000.0, "Compressor release")?;
                check(makeup_db, 0.0, 24.0, "Compressor makeup")?;
                check(mix, 0.0, 1.0, "Compressor mix")?;
            }
        }
    }
    Ok(())
}

pub fn sound_presets() -> Vec<SoundPreset> {
    let base = SynthPatch {
        reverb: 0.0,
        delay: 0.0,
        ..Default::default()
    };
    let room = |size, decay, mix| EffectSlot {
        enabled: true,
        effect: Effect::Reverb {
            size,
            decay,
            damping: 0.45,
            mix,
        },
    };
    let mut keys = base.clone();
    keys.synth.osc1.waveform = Waveform::Triangle;
    keys.synth.osc2.waveform = Waveform::Sine;
    keys.synth.osc2.semitones = 12;
    keys.synth.osc2.level = 0.12;
    keys.synth.sub_level = 0.0;
    keys.cutoff = 3500.0;
    keys.decay = 0.7;
    keys.sustain = 0.22;
    keys.effects = vec![room(0.3, 1.4, 0.18)];
    let mut pad = base.clone();
    pad.attack = 0.8;
    pad.release = 2.8;
    pad.cutoff = 1700.0;
    pad.synth.osc2.detune_cents = -11.0;
    pad.synth.lfo_filter = 0.6;
    pad.effects = vec![
        EffectSlot {
            enabled: true,
            effect: Effect::Chorus {
                rate: 0.25,
                depth: 0.65,
                mix: 0.25,
            },
        },
        room(0.8, 4.2, 0.3),
    ];
    let mut bass = base.clone();
    bass.synth.osc1.waveform = Waveform::Pulse;
    bass.synth.osc1.pulse_width = 0.35;
    bass.synth.osc2.level = 0.0;
    bass.synth.sub_level = 0.5;
    bass.synth.resonance = 0.3;
    bass.synth.filter_env = 2.5;
    bass.cutoff = 180.0;
    bass.decay = 0.2;
    bass.sustain = 0.35;
    bass.release = 0.12;
    bass.effects = vec![EffectSlot {
        enabled: true,
        effect: Effect::Drive {
            drive: 2.0,
            tone: 2800.0,
            mix: 0.2,
        },
    }];
    let mut glass = keys.clone();
    glass.synth.osc1.waveform = Waveform::Sine;
    glass.synth.osc2.semitones = 19;
    glass.synth.osc2.level = 0.3;
    glass.cutoff = 9000.0;
    glass.decay = 0.45;
    glass.sustain = 0.0;
    glass.release = 0.9;
    glass.effects = vec![
        EffectSlot {
            enabled: true,
            effect: Effect::Echo {
                beats: 0.75,
                feedback: 0.4,
                damping: 0.3,
                ping_pong: true,
                mix: 0.25,
            },
        },
        room(0.6, 2.5, 0.18),
    ];
    let mut pluck = base.clone();
    pluck.cutoff = 450.0;
    pluck.decay = 0.22;
    pluck.sustain = 0.0;
    pluck.release = 0.15;
    pluck.synth.filter_env = 4.0;
    pluck.synth.resonance = 0.4;
    pluck.effects = vec![room(0.2, 0.7, 0.1)];
    let mut air = pad.clone();
    air.synth.osc1.waveform = Waveform::Triangle;
    air.synth.osc2.waveform = Waveform::Pulse;
    air.synth.osc2.pulse_width = 0.2;
    air.synth.noise_level = 0.06;
    air.synth.lfo_pitch = 0.08;
    air.synth.filter_mode = FilterMode::BandPass;
    air.cutoff = 2400.0;
    [
        (
            "amber_keys",
            "Amber keys",
            "Rounded triangle keys with a small room.",
            keys,
        ),
        (
            "slow_horizon",
            "Slow horizon",
            "Detuned saw pad, gentle motion, wide hall.",
            pad,
        ),
        (
            "copper_bass",
            "Copper bass",
            "Pulse and sub bass with a snapping filter.",
            bass,
        ),
        (
            "glass_orbit",
            "Glass orbit",
            "Sine harmonics with a dotted ping-pong echo.",
            glass,
        ),
        (
            "velvet_pluck",
            "Velvet pluck",
            "Short subtractive pluck with a bright attack.",
            pluck,
        ),
        (
            "air_current",
            "Air current",
            "Breathing band-pass texture and stereo chorus.",
            air,
        ),
    ]
    .into_iter()
    .map(|(id, name, description, patch)| SoundPreset {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        patch,
    })
    .collect()
}
