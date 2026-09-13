use dawwny_core::*;

#[test]
fn demo_is_valid_and_serializable() {
    let p = demo_project();
    validate(&p).unwrap();
    assert_eq!(
        p,
        serde_json::from_str::<Project>(&serde_json::to_string(&p).unwrap()).unwrap()
    );
    assert!(
        p.tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| c.notes.len())
            .sum::<usize>()
            > 300
    );
}
#[test]
fn a_failed_batch_cannot_partially_change_disk() {
    let d = tempfile::tempdir().unwrap();
    let store = SessionStore::new(d.path().join("project.json"));
    let original = store.initialize(&demo_project()).unwrap();
    assert!(
        store
            .transact(
                0,
                &[
                    Command::RenameProject {
                        name: "Changed".into()
                    },
                    Command::SetTempo { tempo: f64::NAN }
                ]
            )
            .is_err()
    );
    assert_eq!(store.load().unwrap(), original);
    let next = store
        .transact(0, &[Command::SetTempo { tempo: 102.0 }])
        .unwrap();
    assert_eq!(next.revision, 1);
    assert!(
        store
            .transact(0, &[Command::SetTempo { tempo: 80.0 }])
            .unwrap_err()
            .to_string()
            .contains("Revision conflict")
    );
    let restored = store.replace(1, &original).unwrap();
    assert_eq!(restored.revision, 2);
    assert_eq!(restored.id, original.id);
    assert_eq!(store.initialize(&Project::default()).unwrap(), restored);
}
#[test]
fn parameters_sections_and_duplicate_ids_are_bounded() {
    let original = demo_project();
    let mut p = original.clone();
    p.schema_version = 2;
    assert!(validate(&p).is_err());
    p = original.clone();
    p.tracks[0].patch.release = f32::INFINITY;
    assert!(validate(&p).is_err());
    p = original.clone();
    p.tracks[0].patch.reverb = 1.5;
    assert!(validate(&p).is_err());
    p = original.clone();
    p.sections[0].start_bar = u32::MAX;
    assert!(validate(&p).is_err());
    p = original.clone();
    p.tracks[0].clips[0].notes[0].id = p.tracks[1].id.clone();
    assert!(validate(&p).is_err());
    p = original.clone();
    p.tracks[0].gain = -0.1;
    assert!(validate(&p).is_err());
    p = original.clone();
    p.tracks[0].clips[0].notes[0].start = 20.0;
    assert!(validate(&p).is_err());
    assert!(apply_commands(&original, &[]).is_err());
}
#[test]
fn midi_roundtrip_preserves_each_note_tempo_and_drum_channel() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("demo.mid");
    let p = demo_project();
    export_midi(&p, &path).unwrap();
    let q = import_midi(&path).unwrap();
    assert_eq!(p.tracks.len(), q.tracks.len());
    assert_eq!(p.length_bars, q.length_bars);
    assert!((p.tempo - q.tempo).abs() < 0.001);
    for (a, b) in p.tracks.iter().zip(&q.tracks) {
        assert_eq!(a.instrument, b.instrument);
        let flatten = |t: &Track| {
            let mut notes = t
                .clips
                .iter()
                .flat_map(|c| {
                    c.notes.iter().map(|n| {
                        (
                            n.pitch,
                            ((n.start + c.start) * 960.0).round() as i64,
                            (n.duration * 960.0).round() as i64,
                            (n.velocity * 127.0).round() as i64,
                        )
                    })
                })
                .collect::<Vec<_>>();
            notes.sort();
            notes
        };
        let an = flatten(a);
        let bn = flatten(b);
        assert_eq!(an.len(), bn.len());
        for (a, b) in an.iter().zip(bn) {
            assert_eq!(a.0, b.0);
            assert!((a.1 - b.1).abs() <= 1);
            assert!((a.2 - b.2).abs() <= 1);
            assert_eq!(a.3, b.3);
        }
    }
}
#[test]
fn midi_rejects_tempo_maps() {
    use midly::{Format, Header, MetaMessage, Smf, Timing, TrackEvent, TrackEventKind};
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("tempo.mid");
    let smf = Smf {
        header: Header::new(Format::SingleTrack, Timing::Metrical(480.into())),
        tracks: vec![vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(500_000.into())),
            },
            TrackEvent {
                delta: 480.into(),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(600_000.into())),
            },
        ]],
    };
    smf.save(&path).unwrap();
    assert!(
        import_midi(&path)
            .unwrap_err()
            .to_string()
            .contains("Tempo maps")
    );
}
#[test]
fn simultaneous_session_writers_do_not_overwrite() {
    let d = tempfile::tempdir().unwrap();
    let s = SessionStore::new(d.path().join("shared.json"));
    s.initialize(&demo_project()).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let threads = (0..2)
        .map(|i| {
            let store = s.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.transact(
                    0,
                    &[Command::RenameProject {
                        name: format!("Writer {i}"),
                    }],
                )
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let successes = threads
        .into_iter()
        .filter_map(|t| t.join().unwrap().ok())
        .count();
    assert_eq!(successes, 1);
    assert_eq!(s.load().unwrap().revision, 1);
}
