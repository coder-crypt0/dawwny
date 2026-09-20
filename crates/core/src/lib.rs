//! Versioned musical document shared by the native UI, audio engine, and agents.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
mod commands;
mod demo;
mod midi;
mod sound;
mod storage;
mod validation;
pub use commands::apply_commands;
pub use demo::demo_project;
pub use midi::{export_midi, import_midi};
pub use sound::{
    CustomSynth, Effect, EffectSlot, FilterMode, Oscillator, SoundPreset, Waveform, sound_presets,
};
pub use storage::{SessionStore, load_project, new_id, save_project};
pub use validation::validate;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub tempo: f64,
    pub length_bars: u32,
    pub master_gain: f32,
    pub tracks: Vec<Track>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub color: [u8; 3],
    pub instrument: Instrument,
    pub gain: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub patch: SynthPatch,
    pub clips: Vec<Clip>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Instrument {
    Keys,
    Pad,
    Bass,
    Lead,
    Drums,
    Synth,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SynthPatch {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub cutoff: f32,
    pub reverb: f32,
    pub delay: f32,
    #[serde(default)]
    pub synth: CustomSynth,
    #[serde(default)]
    pub effects: Vec<EffectSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub id: String,
    pub name: String,
    pub start: f64,
    pub length: f64,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub pitch: u8,
    pub start: f64,
    pub duration: f64,
    pub velocity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub name: String,
    pub start_bar: u32,
    pub length_bars: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    SetTempo {
        tempo: f64,
    },
    RenameProject {
        name: String,
    },
    SetLength {
        length_bars: u32,
    },
    SetMasterGain {
        gain: f32,
    },
    AddTrack {
        track: Track,
    },
    RemoveTrack {
        track_id: String,
    },
    /// Load a stock Dawn synth preset onto a track, preserving its notes and mix.
    ApplySoundPreset {
        track_id: String,
        preset_id: String,
    },
    UpdateTrack {
        track_id: String,
        name: Option<String>,
        gain: Option<f32>,
        pan: Option<f32>,
        mute: Option<bool>,
        solo: Option<bool>,
        instrument: Option<Instrument>,
        patch: Option<SynthPatch>,
    },
    AddClip {
        track_id: String,
        clip: Clip,
    },
    RemoveClip {
        track_id: String,
        clip_id: String,
    },
    SetClip {
        track_id: String,
        clip: Clip,
    },
    AddNotes {
        track_id: String,
        clip_id: String,
        notes: Vec<Note>,
    },
    RemoveNote {
        track_id: String,
        clip_id: String,
        note_id: String,
    },
}

impl Default for SynthPatch {
    fn default() -> Self {
        Self {
            attack: 0.008,
            decay: 0.25,
            sustain: 0.55,
            release: 0.35,
            cutoff: 8000.0,
            reverb: 0.18,
            delay: 0.0,
            synth: CustomSynth::default(),
            effects: Vec::new(),
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: 1,
            id: "session".into(),
            name: "Untitled session".into(),
            revision: 0,
            tempo: 92.0,
            length_bars: 16,
            master_gain: 0.7,
            tracks: Vec::new(),
            sections: Vec::new(),
        }
    }
}
