//! Shared musical document. Beats are quarter notes; MIDI pitches are 0..=127.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_roundtrip() {
        let project = Project::default();
        let json = serde_json::to_string(&project).unwrap();
        assert_eq!(project, serde_json::from_str::<Project>(&json).unwrap());
    }
}
