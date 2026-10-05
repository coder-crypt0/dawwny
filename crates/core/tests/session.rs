use dawwny_core::*;

#[test]
fn clip_transforms_preserve_note_identity_gate_velocity_and_atomicity() {
    let mut project = demo_project();
    project.tracks.truncate(1);
    project.tracks[0].clips.truncate(1);
    let clip = &mut project.tracks[0].clips[0];
    clip.length = 4.0;
    clip.notes.truncate(2);
    clip.notes[0].start = 0.26;
    clip.notes[0].duration = 0.5;
    clip.notes[1].start = 3.9;
    clip.notes[1].duration = 0.1;
    let before = clip.clone();
    let command = Command::QuantizeClip {
        track_id: project.tracks[0].id.clone(),
        clip_id: before.id.clone(),
        grid: 0.25,
    };
    let quantized = apply_commands(&project, &[command]).unwrap();
    let notes = &quantized.tracks[0].clips[0].notes;
    assert_eq!(notes[0].start, 0.25);
    assert_eq!(notes[1].start, 3.75);
    for (note, old) in notes.iter().zip(&before.notes) {
        assert_eq!(
            (note.id.as_str(), note.duration, note.velocity, note.pitch),
            (old.id.as_str(), old.duration, old.velocity, old.pitch)
        );
    }
    let transpose = Command::TransposeClip {
        track_id: project.tracks[0].id.clone(),
        clip_id: before.id.clone(),
        semitones: 12,
    };
    let moved = apply_commands(&quantized, std::slice::from_ref(&transpose)).unwrap();
    assert_eq!(
        moved.tracks[0].clips[0].notes[0].pitch,
        before.notes[0].pitch + 12
    );
    project.tracks[0].clips[0].notes[1].pitch = 127;
    assert!(apply_commands(&project, &[transpose]).is_err());
    assert_eq!(
        project.tracks[0].clips[0].notes[0].pitch,
        before.notes[0].pitch
    );
    let invalid = Command::QuantizeClip {
        track_id: project.tracks[0].id.clone(),
        clip_id: before.id,
        grid: f64::NAN,
    };
    assert!(apply_commands(&project, &[invalid]).is_err());
}

#[test]
fn section_and_cycle_commands_are_persistent_atomic_and_backward_compatible() {
    let dir = tempfile::tempdir().unwrap();
    let store = SessionStore::new(dir.path().join("song.json"));
    let original = store.initialize(&demo_project()).unwrap();
    let mut legacy = serde_json::to_value(&original).unwrap();
    legacy.as_object_mut().unwrap().remove("cycle");
    assert_eq!(
        serde_json::from_value::<Project>(legacy).unwrap().cycle,
        None
    );
    let section = Section {
        name: "Chorus".into(),
        start_bar: 4,
        length_bars: 4,
    };
    let range = section.cycle_range();
    let cycled = store
        .transact(
            0,
            &[
                Command::SetSections {
                    sections: vec![section],
                },
                Command::SetCycleRange { range: Some(range) },
            ],
        )
        .unwrap();
    assert_eq!(
        store.load().unwrap().cycle,
        Some(CycleRange {
            start: 16.0,
            end: 32.0
        })
    );
    assert_eq!(cycled.tracks, original.tracks);
    for invalid in [
        CycleRange {
            start: f64::NAN,
            end: 4.0,
        },
        CycleRange {
            start: -1.0,
            end: 4.0,
        },
        CycleRange {
            start: 4.0,
            end: 4.0,
        },
        CycleRange {
            start: 4.0,
            end: 4.1,
        },
        CycleRange {
            start: 16.0,
            end: 65.0,
        },
    ] {
        assert!(
            store
                .transact(
                    1,
                    &[
                        Command::RenameProject {
                            name: "Should not save".into()
                        },
                        Command::SetCycleRange {
                            range: Some(invalid)
                        },
                    ]
                )
                .is_err()
        );
        assert_eq!(store.load().unwrap(), cycled);
    }
    assert!(
        store
            .transact(1, &[Command::SetLength { length_bars: 2 }])
            .is_err()
    );
    let cleared = store
        .transact(1, &[Command::SetCycleRange { range: None }])
        .unwrap();
    assert_eq!(cleared.cycle, None);
}

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

#[test]
fn sample_selection_preserves_notes_effects_and_exports_bank_and_program() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = demo_project();
    p.tracks.truncate(1);
    let notes = p.tracks[0].clips.clone();
    let patch = p.tracks[0].patch.clone();
    let sample = SampleInstrument {
        file: dir.path().join("bank.sf2").to_string_lossy().into_owned(),
        bank: 8,
        program: 24,
    };
    let draft = apply_commands(
        &p,
        &[Command::SetSampleInstrument {
            track_id: p.tracks[0].id.clone(),
            sample: sample.clone(),
        }],
    )
    .unwrap();
    assert_eq!(draft.tracks[0].clips, notes);
    assert_eq!(draft.tracks[0].patch.effects, patch.effects);
    assert_eq!(draft.tracks[0].instrument, Instrument::Sampler);
    let midi = dir.path().join("sample.mid");
    export_midi(&draft, &midi).unwrap();
    let bytes = std::fs::read(midi).unwrap();
    let smf = midly::Smf::parse(&bytes).unwrap();
    assert!(smf.tracks[1].iter().any(|e|matches!(e.kind,midly::TrackEventKind::Midi {message:midly::MidiMessage::ProgramChange {program},..} if program.as_int()==24)));
    assert!(smf.tracks[1].iter().any(|e|matches!(e.kind,midly::TrackEventKind::Midi {message:midly::MidiMessage::Controller {controller,value},..} if controller.as_int()==0 && value.as_int()==8)));
    let mut bad = sample;
    bad.file = "relative.sf2".into();
    assert!(
        apply_commands(
            &p,
            &[Command::SetSampleInstrument {
                track_id: p.tracks[0].id.clone(),
                sample: bad
            }]
        )
        .is_err()
    );
}
