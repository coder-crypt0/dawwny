use dawwny_core::*;

#[test]
fn old_sessions_keep_their_sound_and_new_presets_are_distinct() {
    let old: Project =
        serde_json::from_str(include_str!("../../../examples/demo.dawwny.json")).unwrap();
    validate(&old).unwrap();
    assert!(old.tracks.iter().all(|t| t.patch.effects.is_empty()));
    let presets = sound_presets();
    for (i, preset) in presets.iter().enumerate() {
        let mut p = old.clone();
        p.tracks[0].instrument = Instrument::Synth;
        p.tracks[0].patch = preset.patch.clone();
        validate(&p).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<Project>(&json).unwrap(), p);
        for other in &presets[i + 1..] {
            assert_ne!(preset.patch, other.patch);
            assert_ne!(preset.id, other.id);
        }
    }
}
#[test]
fn invalid_sound_values_and_oversized_racks_are_rejected() {
    let mut p = demo_project();
    for bad in [f32::NAN, f32::INFINITY, -0.1, 0.91] {
        p.tracks[0].patch.synth.resonance = bad;
        assert!(validate(&p).is_err());
    }
    p.tracks[0].patch.synth.resonance = 0.2;
    p.tracks[0].patch.effects = vec![EffectSlot {
        enabled: true,
        effect: Effect::Echo {
            beats: 0.5,
            feedback: 0.86,
            damping: 0.3,
            ping_pong: true,
            mix: 0.3,
        },
    }];
    assert!(validate(&p).is_err());
    p.tracks[0].patch.effects = vec![
        EffectSlot {
            enabled: false,
            effect: Effect::Drive {
                drive: 2.0,
                tone: 1000.0,
                mix: 0.5
            }
        };
        9
    ];
    assert!(validate(&p).is_err());
}
#[test]
fn preset_transactions_preserve_notes_mix_and_atomicity() {
    let p = demo_project();
    let id = p.tracks[0].id.clone();
    let changed = apply_commands(
        &p,
        &[Command::ApplySoundPreset {
            track_id: id.clone(),
            preset_id: "slow_horizon".into(),
        }],
    )
    .unwrap();
    assert_eq!(changed.tracks[0].instrument, Instrument::Synth);
    assert_eq!(changed.tracks[0].clips, p.tracks[0].clips);
    assert_eq!(changed.tracks[0].gain, p.tracks[0].gain);
    assert_eq!(changed.revision, p.revision + 1);
    assert!(
        apply_commands(
            &p,
            &[
                Command::SetTempo { tempo: 150.0 },
                Command::ApplySoundPreset {
                    track_id: id,
                    preset_id: "unknown".into()
                }
            ]
        )
        .is_err()
    );
    assert_eq!(p.tempo, 92.0);
}
