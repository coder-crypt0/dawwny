//! A compact factory bank: 72 authored families × 32 tuned variations, plus six signature sounds.
//! Patches are constructed on selection, never stored as thousands of JSON files or audio samples.
mod families;
use crate::{
    CustomSynth, Effect, EffectSlot, FilterMode, Oscillator, SoundPreset, SynthPatch, Waveform,
};
use families::FAMILIES;
use serde::Serialize;
use std::sync::OnceLock;

struct Family {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    waves: [Waveform; 2],
    tune: i32,
    cutoff: f32,
    adsr: [f32; 4],
    sub: f32,
    noise: f32,
    tag: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct SoundInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub family: String,
    pub tags: Vec<String>,
    pub description: String,
    pub audition_pitch: u8,
}
const COLORS: [&str; 8] = [
    "Core", "Warm", "Bright", "Soft", "Focused", "Hollow", "Vivid", "Velvet",
];
const SPACES: [&str; 4] = ["Dry", "Room", "Wide", "Echo"];
pub const SOUND_CATEGORIES: &[&str] = &[
    "Bass",
    "Lead",
    "Pad",
    "Keys",
    "Pluck",
    "Bell",
    "Organ",
    "Brass",
    "Strings",
    "Motion",
    "Texture",
    "Percussive",
];
fn pitch(category: &str) -> u8 {
    match category {
        "Bass" => 36,
        "Percussive" => 48,
        "Bell" => 72,
        _ => 60,
    }
}
pub fn sound_catalog() -> &'static [SoundInfo] {
    static CATALOG: OnceLock<Vec<SoundInfo>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut sounds = Vec::with_capacity(2310);
        for (preset,category) in crate::sound_presets().into_iter().zip(["Keys","Pad","Bass","Bell","Pluck","Texture"]) {
            sounds.push(SoundInfo{id:preset.id,name:preset.name,family:"Signature".into(),category:category.into(),tags:vec!["signature".into()],description:preset.description,audition_pitch:pitch(category)});
        }
        for family in FAMILIES {
            for (color,cname) in COLORS.iter().enumerate() {
                for (space,sname) in SPACES.iter().enumerate() {
                    sounds.push(SoundInfo{
                        id:format!("factory.{}.{:02}",family.id,color*4+space),
                        name:format!("{} · {cname} {sname}",family.name),
                        family:family.name.into(),category:family.category.into(),
                        tags:vec![family.tag.into(),cname.to_lowercase(),sname.to_lowercase(),"synthesized".into()],
                        description:format!("{} {} voice, {} articulation with {} space. Fully editable Dawn synthesis.",family.tag,family.category.to_lowercase(),cname.to_lowercase(),sname.to_lowercase()),
                        audition_pitch:pitch(family.category),
                    });
                }
            }
        }
        sounds
    })
}
fn slot(effect: Effect) -> EffectSlot {
    EffectSlot {
        enabled: true,
        effect,
    }
}
pub fn sound_preset(id: &str) -> Option<SoundPreset> {
    if !id.starts_with("factory.") {
        return crate::sound_presets().into_iter().find(|p| p.id == id);
    }
    let mut parts = id.strip_prefix("factory.")?.split('.');
    let slug = parts.next()?;
    let number = parts.next()?;
    if number.len() != 2 || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let index: usize = number.parse().ok()?;
    if index >= 32 || number != format!("{index:02}") || parts.next().is_some() {
        return None;
    }
    let (family_index, f) = FAMILIES.iter().enumerate().find(|(_, f)| f.id == slug)?;
    let color = index / 4;
    let space = index % 4;
    let is_bass = f.category == "Bass";
    let transient = matches!(f.category, "Pluck" | "Bell" | "Percussive");
    let slow = matches!(f.category, "Pad" | "Strings" | "Texture");
    let detune = [2.0, 7.0, -9.0, 14.0, -5.0, 11.0][family_index % 6];
    let pulse_width = [0.5, 0.3, 0.22, 0.42, 0.16, 0.65][family_index % 6];
    let mut patch = SynthPatch {
        attack: f.adsr[0],
        decay: f.adsr[1],
        sustain: f.adsr[2],
        release: f.adsr[3],
        cutoff: f.cutoff,
        reverb: 0.0,
        delay: 0.0,
        synth: CustomSynth {
            osc1: Oscillator {
                waveform: f.waves[0],
                level: 0.58,
                semitones: 0,
                detune_cents: 0.0,
                pulse_width,
            },
            osc2: Oscillator {
                waveform: f.waves[1],
                level: if is_bass { 0.18 } else { 0.28 },
                semitones: f.tune,
                detune_cents: if f.category == "Organ" { 0.0 } else { detune },
                pulse_width: 0.5,
            },
            sub_level: f.sub,
            noise_level: f.noise,
            resonance: if is_bass { 0.28 } else { 0.16 },
            filter_env: if transient {
                1.5
            } else if f.category == "Organ" {
                0.0
            } else {
                0.5
            },
            filter_mode: if f.id == "breathy-band" {
                FilterMode::BandPass
            } else {
                FilterMode::LowPass
            },
            lfo_rate: if f.category == "Motion" {
                [0.15, 3.0, 0.24, 0.6, 1.2, 0.35][family_index % 6]
            } else {
                0.25
            },
            lfo_filter: if f.category == "Motion" {
                1.2
            } else if slow {
                0.12
            } else {
                0.0
            },
            lfo_pitch: if f.id == "pitch-drift" { 0.25 } else { 0.0 },
        },
        effects: Vec::new(),
    };
    if f.id == "noise-hat" {
        patch.synth.osc1.level = 0.0;
        patch.synth.osc2.level = 0.0;
        patch.synth.filter_mode = FilterMode::HighPass;
        patch.cutoff = 5500.0;
        patch.synth.filter_env = 0.0;
    }
    if matches!(f.tag, "gritty" | "biting" | "assertive") {
        patch.effects.push(slot(Effect::Drive {
            drive: 2.2,
            tone: 5000.0,
            mix: 0.22,
        }));
    }
    // Colors change articulation and spectrum together, while preserving the family's role.
    patch.cutoff *= [1.0, 0.66, 1.5, 0.82, 1.14, 0.9, 1.3, 0.53][color];
    match color {
        0 => {}
        1 => {
            patch.synth.osc2.level *= 0.85;
            patch.decay *= 1.15;
        }
        2 => {
            patch.synth.osc2.level *= 1.2;
            patch.synth.filter_env += 0.25;
        }
        3 => {
            patch.attack = patch.attack * 1.35 + 0.004;
            patch.synth.resonance *= 0.75;
            patch.release *= 1.2;
        }
        4 => {
            patch.synth.osc2.level *= 0.4;
            patch.release *= 0.7;
            patch.synth.resonance += 0.12;
        }
        5 => {
            patch.synth.osc1.pulse_width = 0.18;
            patch.synth.osc2.semitones = (f.tune + 12).clamp(-24, 24);
            patch.synth.osc2.level *= 0.7;
        }
        6 => {
            patch.synth.osc2.detune_cents += if is_bass { 3.0 } else { 9.0 };
            patch.synth.lfo_filter += 0.22;
            patch.synth.resonance += 0.08;
        }
        _ => {
            patch.attack = patch.attack * 1.1 + 0.002;
            patch.decay *= 1.4;
            patch.synth.noise_level *= 0.6;
            patch.synth.osc2.level *= 0.65;
        }
    }
    let wet = if is_bass { 0.12 } else { 0.24 };
    match space {
        0 => {}
        1 => patch.effects.push(slot(Effect::Reverb {
            size: if slow { 0.65 } else { 0.25 },
            decay: if slow { 2.4 } else { 0.9 },
            damping: 0.55,
            mix: wet,
        })),
        2 => {
            patch.effects.push(slot(Effect::Chorus {
                rate: 0.3,
                depth: if is_bass { 0.25 } else { 0.55 },
                mix: wet,
            }));
            if slow {
                patch.effects.push(slot(Effect::Reverb {
                    size: 0.8,
                    decay: 3.2,
                    damping: 0.5,
                    mix: 0.18,
                }));
            }
        }
        _ => patch.effects.push(slot(Effect::Echo {
            beats: if transient || is_bass { 0.5 } else { 0.75 },
            feedback: if is_bass { 0.2 } else { 0.35 },
            damping: 0.4,
            ping_pong: !is_bass,
            mix: wet,
        })),
    }
    patch.cutoff = patch.cutoff.clamp(20.0, 20000.0);
    patch.attack = patch.attack.clamp(0.001, 5.0);
    patch.decay = patch.decay.clamp(0.001, 5.0);
    patch.release = patch.release.clamp(0.001, 5.0);
    Some(SoundPreset {
        id: id.into(),
        name: format!("{} · {} {}", f.name, COLORS[color], SPACES[space]),
        description: format!(
            "{} {} voice. {} articulation, {} space.",
            f.tag, f.category, COLORS[color], SPACES[space]
        ),
        patch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn every_factory_sound_is_unique_valid_and_retrievable() {
        assert_eq!(FAMILIES.len(), 72);
        assert_eq!(sound_catalog().len(), 2310);
        let mut ids = HashSet::new();
        let mut patches = HashSet::new();
        let mut project = crate::demo_project();
        for info in sound_catalog() {
            assert!(ids.insert(info.id.clone()));
            let preset = sound_preset(&info.id).unwrap();
            assert_eq!(preset.name, info.name);
            assert!(
                patches.insert(serde_json::to_string(&preset.patch).unwrap()),
                "duplicate patch: {}",
                info.id
            );
            project.tracks[0].patch = preset.patch;
            crate::validate(&project).unwrap();
        }
        assert!(sound_preset("factory.soft-sub.99").is_none());
        assert!(sound_preset("factory.soft-sub.0").is_none());
        assert!(sound_preset("factory.soft-sub.00.extra").is_none());
    }
}
