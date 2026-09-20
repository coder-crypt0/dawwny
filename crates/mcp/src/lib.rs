//! Local MCP access to one explicitly configured session; no arbitrary file tools.
use dawwny_core::{Command, SessionStore};
use rmcp::{handler::server::wrapper::Parameters, schemars::JsonSchema, tool, tool_router};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyCommandsArgs {
    /// Revision returned by read_project. Stale transactions are rejected atomically.
    pub expected_revision: u64,
    /// One to 256 musical commands, validated as a single transaction.
    pub commands: Vec<Command>,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportArgs {
    pub expected_revision: u64,
}

#[derive(Clone)]
pub struct DawwnyMcp {
    store: SessionStore,
    export_dir: PathBuf,
    export_slot: Arc<tokio::sync::Semaphore>,
}
impl DawwnyMcp {
    pub fn new(project: PathBuf, export_dir: PathBuf) -> Self {
        Self {
            store: SessionStore::new(project),
            export_dir,
            export_slot: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
    async fn export(&self, expected_revision: u64, wav: bool) -> Result<String, String> {
        let permit = self.export_slot.clone().try_acquire_owned().map_err(|_| {
            "An export is already running; try again after it completes".to_string()
        })?;
        let store = self.store.clone();
        let dir = self.export_dir.clone();
        tokio::task::spawn_blocking(move||{
            let _permit=permit;
            let project=store.load().map_err(|e|e.to_string())?;
            if project.revision!=expected_revision{return Err(format!("Revision conflict: expected {expected_revision}, current {}",project.revision));}
            std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
            let path=dir.join(format!("revision-{}-{}.{}",project.revision,dawwny_core::new_id(),if wav{"wav"}else{"mid"}));
            let value=if wav {
                let stats=dawwny_audio::render_wav(&project,&path,48_000).map_err(|e|e.to_string())?;
                serde_json::json!({"revision":project.revision,"path":path,"frames":stats.frames,"peak":stats.peak,"sample_rate":48000,"stolen_voices":stats.stolen_voices})
            }else{
                dawwny_core::export_midi(&project,&path).map_err(|e|e.to_string())?;
                serde_json::json!({"revision":project.revision,"path":path})
            };
            Ok(value.to_string())
        }).await.map_err(|e|format!("Export worker failed: {e}"))?
    }
}

#[tool_router(server_handler)]
impl DawwnyMcp {
    #[tool(
        description = "Discover the Dawn custom synthesizer and six stock sound presets, including complete editable patch settings. Apply a preset with apply_sound_preset, or edit its patch with update_track. Set instrument to synth to enable oscillators, sub/noise, resonant filter and LFO. Every instrument supports the ordered eight-slot effects rack: reverb, tempo echo, chorus, drive, three-band EQ and compressor. No third-party plugin binaries are required."
    )]
    async fn list_sounds(&self) -> Result<String, String> {
        serde_json::to_string(&serde_json::json!({
            "instrument": "synth",
            "presets": dawwny_core::sound_presets(),
            "effects": ["reverb", "echo", "chorus", "drive", "equalizer", "compressor"],
            "signal_flow": "voice -> envelope/filter -> track gain/pan and legacy ambience -> ordered effect slots -> master limiter",
            "limits": {"effect_slots_per_track": 8, "effect_buffers_mib_per_session": 64, "rack_tail_seconds": 30}
        })).map_err(|e| e.to_string())
    }
    #[tool(
        description = "Read the complete dawwny musical project and current revision. Read this before editing. Beat positions use quarter notes; clip note starts are relative to their clip. No AI model is bundled."
    )]
    async fn read_project(&self) -> Result<String, String> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            store
                .load()
                .map_err(|e| e.to_string())
                .and_then(|p| serde_json::to_string(&p).map_err(|e| e.to_string()))
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[tool(
        description = "Apply 1–256 validated musical commands atomically to the configured local project. Requires the current expected_revision. IDs must be unique. Fixed 4/4, 30–300 BPM, 32 tracks, 128 clips, 32768 notes. Use list_sounds to discover presets. Custom oscillator/filter/LFO settings apply to instrument synth; ordered effects apply to every instrument. update_track replaces the entire patch, so preserve settings you want to keep. After a revision conflict read_project again; never blindly retry old changes."
    )]
    async fn apply_commands(
        &self,
        Parameters(args): Parameters<ApplyCommandsArgs>,
    ) -> Result<String, String> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            store
                .transact(args.expected_revision, &args.commands)
                .map_err(|e| e.to_string())
                .and_then(|p| serde_json::to_string(&p).map_err(|e| e.to_string()))
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[tool(
        description = "Export all MIDI notes, tempo, and instrument families to a unique .mid file in the configured export directory. MIDI preserves every track, including muted tracks; mixer/effects are not MIDI audio. Requires the current revision."
    )]
    async fn export_midi(
        &self,
        Parameters(args): Parameters<ExportArgs>,
    ) -> Result<String, String> {
        self.export(args.expected_revision, false).await
    }
    #[tool(
        description = "Render the audible mix through the native synth engine to a unique 24-bit stereo WAV at 48 kHz. Applies custom synthesis, mute/solo, gain/pan and the ordered stock effect rack. Rack tails are capped at 30 seconds. Streams to disk; one export at a time. Requires current revision. Native VST binaries are not yet hosted."
    )]
    async fn render_wav(&self, Parameters(args): Parameters<ExportArgs>) -> Result<String, String> {
        self.export(args.expected_revision, true).await
    }
}
pub fn build_server(project: PathBuf, export_dir: PathBuf) -> DawwnyMcp {
    DawwnyMcp::new(project, export_dir)
}
