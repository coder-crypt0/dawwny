use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, CallToolResult},
};
use serde_json::json;

fn output(result: &CallToolResult) -> serde_json::Value {
    serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap()
}
#[tokio::test]
async fn mcp_client_edits_the_native_session_and_exports_real_music() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("session.json");
    let mut project = dawwny_core::demo_project();
    project.length_bars = 1;
    project.sections.clear();
    project.tracks.truncate(1);
    project.tracks[0].clips.truncate(1);
    project.tracks[0].clips[0].length = 4.0;
    project.tracks[0].clips[0]
        .notes
        .retain(|n| n.start + n.duration <= 4.0);
    let store = dawwny_core::SessionStore::new(path.clone());
    store.initialize(&project)?;
    let (server_io, client_io) = tokio::io::duplex(65536);
    let service = dawwny_mcp::build_server(path, dir.path().join("exports"));
    let server = tokio::spawn(async move {
        let s = service.serve(server_io).await?;
        s.waiting().await?;
        anyhow::Ok(())
    });
    let mut client = ().serve(client_io).await?;
    let tools = client.list_all_tools().await?;
    assert_eq!(tools.len(), 5);
    let catalog = client
        .call_tool(CallToolRequestParams::new("list_sounds"))
        .await?;
    assert_eq!(output(&catalog)["presets"].as_array().unwrap().len(), 6);
    let read = client
        .call_tool(CallToolRequestParams::new("read_project"))
        .await?;
    assert_eq!(output(&read)["revision"], 0);
    let apply = CallToolRequestParams::new("apply_commands").with_arguments(
        json!({"expected_revision":0,"commands":[{"type":"set_tempo","tempo":110.0},{"type":"apply_sound_preset","track_id":project.tracks[0].id,"preset_id":"glass_orbit"}]})
            .as_object()
            .unwrap()
            .clone(),
    );
    let result = client.call_tool(apply.clone()).await?;
    assert_ne!(result.is_error, Some(true));
    assert_eq!(output(&result)["revision"], 1);
    assert_eq!(store.load()?.tempo, 110.0);
    assert_eq!(
        store.load()?.tracks[0].instrument,
        dawwny_core::Instrument::Synth
    );
    let conflict = client.call_tool(apply).await?;
    assert_eq!(conflict.is_error, Some(true));
    assert_eq!(store.load()?.revision, 1);
    let midi = client
        .call_tool(
            CallToolRequestParams::new("export_midi")
                .with_arguments(json!({"expected_revision":1}).as_object().unwrap().clone()),
        )
        .await?;
    assert_ne!(midi.is_error, Some(true));
    let midi_data = output(&midi);
    let imported =
        dawwny_core::import_midi(std::path::Path::new(midi_data["path"].as_str().unwrap()))?;
    assert!(!imported.tracks[0].clips[0].notes.is_empty());
    let wav = client
        .call_tool(
            CallToolRequestParams::new("render_wav")
                .with_arguments(json!({"expected_revision":1}).as_object().unwrap().clone()),
        )
        .await?;
    assert_ne!(wav.is_error, Some(true));
    assert!(output(&wav)["peak"].as_f64().unwrap() > 0.01);
    client.close().await?;
    server.await??;
    Ok(())
}
