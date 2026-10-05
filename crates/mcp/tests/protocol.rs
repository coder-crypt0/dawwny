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
    assert_eq!(tools.len(), 7);
    let catalog = client
        .call_tool(CallToolRequestParams::new("list_sounds"))
        .await?;
    assert_eq!(output(&catalog)["total"], 2310);
    assert_eq!(output(&catalog)["sounds"].as_array().unwrap().len(), 24);
    let sound = client
        .call_tool(
            CallToolRequestParams::new("get_sound").with_arguments(
                json!({"preset_id":"factory.soft-sub.00"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await?;
    assert!(output(&sound)["patch"]["synth"].is_object());
    let read = client
        .call_tool(CallToolRequestParams::new("read_project"))
        .await?;
    assert_eq!(output(&read)["revision"], 0);
    let apply = CallToolRequestParams::new("apply_commands").with_arguments(
        json!({"expected_revision":0,"commands":[{"type":"quantize_clip","track_id":project.tracks[0].id,"clip_id":project.tracks[0].clips[0].id,"grid":0.25},{"type":"transpose_clip","track_id":project.tracks[0].id,"clip_id":project.tracks[0].clips[0].id,"semitones":1},{"type":"set_sections","sections":[{"name":"Verse","start_bar":0,"length_bars":1}]},{"type":"set_cycle_range","range":{"start":0.0,"end":2.0}},{"type":"set_tempo","tempo":110.0},{"type":"apply_sound_preset","track_id":project.tracks[0].id,"preset_id":"factory.soft-sub.00"}]})
            .as_object()
            .unwrap()
            .clone(),
    );
    let result = client.call_tool(apply.clone()).await?;
    assert_ne!(result.is_error, Some(true));
    assert_eq!(output(&result)["revision"], 1);
    assert_eq!(store.load()?.tempo, 110.0);
    assert_eq!(
        store.load()?.tracks[0].clips[0].notes[0].pitch,
        project.tracks[0].clips[0].notes[0].pitch + 1
    );
    assert_eq!(
        store.load()?.cycle,
        Some(dawwny_core::CycleRange {
            start: 0.0,
            end: 2.0
        })
    );
    assert_eq!(store.load()?.sections[0].name, "Verse");
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

#[path = "../../audio/tests/support/mod.rs"]
mod support;
#[tokio::test]
async fn agents_discover_and_select_samples_atomically() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let sf2 = dir.path().join("test.sf2");
    support::bank(&sf2);
    let project = support::project(&sf2);
    let path = dir.path().join("session.json");
    let store = dawwny_core::SessionStore::new(path.clone());
    store.initialize(&project)?;
    let (server_io, client_io) = tokio::io::duplex(65536);
    let server = tokio::spawn(async move {
        let s = dawwny_mcp::build_server(path, dir.path().join("exports"))
            .serve(server_io)
            .await?;
        s.waiting().await?;
        anyhow::Ok(())
    });
    let mut client = ().serve(client_io).await?;
    let catalog = client
        .call_tool(
            CallToolRequestParams::new("list_sample_presets").with_arguments(
                json!({"file":sf2,"query":"bright"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await?;
    assert_eq!(output(&catalog)["total"], 1);
    assert_eq!(output(&catalog)["presets"][0]["program"], 24);
    for (program, valid) in [(127, false), (24, true)] {
        let result=client.call_tool(CallToolRequestParams::new("apply_commands").with_arguments(json!({"expected_revision":0,"commands":[{"type":"set_sample_instrument","track_id":project.tracks[0].id,"sample":{"file":sf2,"bank":0,"program":program}}]}).as_object().unwrap().clone())).await?;
        assert_eq!(result.is_error == Some(true), !valid);
        assert_eq!(store.load()?.revision, if valid { 1 } else { 0 });
    }
    let wav = client
        .call_tool(
            CallToolRequestParams::new("render_wav")
                .with_arguments(json!({"expected_revision":1}).as_object().unwrap().clone()),
        )
        .await?;
    assert!(output(&wav)["peak"].as_f64().unwrap() > 0.01);
    client.close().await?;
    server.await??;
    Ok(())
}
