//! Local MCP access to one explicitly configured session; no arbitrary file tools.
use dawwny_core::{Command, SessionStore};
use rmcp::{handler::server::wrapper::Parameters, schemars::JsonSchema, tool, tool_router};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListSoundsArgs {
    /// Search name, family, category and tags. Maximum 128 bytes.
    pub query: Option<String>,
    pub category: Option<String>,
    /// Zero-based pagination offset.
    pub offset: Option<usize>,
    /// Page size, 1–100. Defaults to 24.
    pub limit: Option<usize>,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetSoundArgs {
    pub preset_id: String,
}

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
        description = "Search and page through 2310 Dawn presets across 12 categories. Returns lightweight metadata, not every patch. Use get_sound for complete settings, then apply_sound_preset to load one on a track. Search terms match name, category, family and tags. Returns total matches and next_offset; maximum page size 100."
    )]
    async fn list_sounds(
        &self,
        Parameters(args): Parameters<ListSoundsArgs>,
    ) -> Result<String, String> {
        let query = args.query.unwrap_or_default().to_lowercase();
        let category = args.category.unwrap_or_default();
        if query.len() > 128 || category.len() > 64 {
            return Err("Sound search is too long".into());
        }
        let limit = args.limit.unwrap_or(24);
        if !(1..=100).contains(&limit) {
            return Err("limit must be 1–100".into());
        }
        let offset = args.offset.unwrap_or(0);
        let terms: Vec<_> = query.split_whitespace().collect();
        let matches: Vec<_> = dawwny_core::sound_catalog()
            .iter()
            .filter(|s| {
                if !category.is_empty() && !s.category.eq_ignore_ascii_case(&category) {
                    return false;
                }
                let text = format!(
                    "{} {} {} {}",
                    s.name,
                    s.category,
                    s.family,
                    s.tags.join(" ")
                )
                .to_lowercase();
                terms.iter().all(|term| text.contains(term))
            })
            .collect();
        let total = matches.len();
        let sounds: Vec<_> = matches.into_iter().skip(offset).take(limit).collect();
        let next = offset.saturating_add(sounds.len());
        serde_json::to_string(&serde_json::json!({"sounds":sounds,"total":total,"offset":offset,"next_offset":if next<total{Some(next)}else{None},"categories":dawwny_core::SOUND_CATEGORIES})).map_err(|e|e.to_string())
    }
    #[tool(
        description = "Get one Dawn preset's complete editable SynthPatch by its stable preset_id from list_sounds. Copy and modify these settings through update_track with instrument synth. Effects work on all instruments. Loading a preset preserves track notes and mix. Presets are synthesized; no sample libraries are downloaded."
    )]
    async fn get_sound(
        &self,
        Parameters(args): Parameters<GetSoundArgs>,
    ) -> Result<String, String> {
        let preset = dawwny_core::sound_preset(&args.preset_id)
            .ok_or("Unknown preset_id; search list_sounds first")?;
        serde_json::to_string(&preset).map_err(|e| e.to_string())
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
