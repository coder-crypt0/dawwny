mod support;
use dawwny_audio::{LiveEvent, Renderer, compile, load_sample_bank};
#[test]
fn names_routing_sharing_and_export_use_the_real_bank() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sf2");
    support::bank(&path);
    let a = load_sample_bank(&path).unwrap();
    let b = load_sample_bank(&path).unwrap();
    assert!(std::sync::Arc::ptr_eq(&a, &b));
    assert_eq!(a.presets.len(), 3);
    assert_eq!(a.presets[1].name, "Test bright");
    assert_eq!(a.presets[2].bank, 128);
    let p = support::project(&path);
    let output = dir.path().join("mix.wav");
    let stats = dawwny_audio::render_wav(&p, &output, 16000).unwrap();
    assert!(stats.peak > 0.01);
    let mut r = Renderer::new(compile(&p, 16000).unwrap());
    r.seek_beats(0.5);
    assert!((0..1000).map(|_| r.next_frame()[0].abs()).sum::<f32>() > 1.0);
    let mut wrong = p.clone();
    wrong.tracks[0].patch.sample.as_mut().unwrap().program = 127;
    assert!(compile(&wrong, 16000).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(
        compile(&p, 16000)
            .err()
            .unwrap()
            .to_string()
            .contains("Missing sample bank")
    );
}
#[test]
fn sample_gate_overlap_velocity_sustain_and_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sf2");
    support::bank(&path);
    let mut p = support::project(&path);
    p.tracks[0].clips[0].notes.push(dawwny_core::Note {
        id: "overlap".into(),
        pitch: 60,
        start: 0.5,
        duration: 1.0,
        velocity: 0.5,
    });
    let mut r = Renderer::new(compile(&p, 16000).unwrap());
    for _ in 0..10400 {
        r.next_frame();
    }
    // First note has released, second same-pitch note remains held.
    assert!((0..1000).map(|_| r.next_frame()[0].abs()).sum::<f32>() > 1.0);
    for _ in 0..5000 {
        r.next_frame();
    }
    assert!((0..1000).map(|_| r.next_frame()[0].abs()).sum::<f32>() < 0.01);
    r.live_event(LiveEvent::NoteOn {
        channel: 16,
        pitch: 60,
        velocity: 0.9,
    });
    assert!((0..1000).map(|_| r.next_live_frame()[0].abs()).sum::<f32>() > 1.0);
    let position = r.frame();
    r.live_event(LiveEvent::Sustain {
        channel: 16,
        down: true,
    });
    r.live_event(LiveEvent::NoteOff {
        channel: 16,
        pitch: 60,
    });
    assert!((0..2000).map(|_| r.next_live_frame()[0].abs()).sum::<f32>() > 1.0);
    r.live_event(LiveEvent::Sustain {
        channel: 16,
        down: false,
    });
    for _ in 0..4000 {
        r.next_live_frame();
    }
    assert!((0..100).map(|_| r.next_live_frame()[0].abs()).sum::<f32>() < 0.01);
    assert_eq!(r.frame(), position);
    r.live_event(LiveEvent::AllOff);
    assert!(!r.has_live_voices());
}
#[test]
fn truncated_and_bad_zone_spans_fail_before_parsing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sf2");
    support::bank(&path);
    let original = std::fs::read(&path).unwrap();
    for n in [0, 7, 12, 48, original.len() - 1] {
        std::fs::write(&path, &original[..n]).unwrap();
        assert!(load_sample_bank(&path).is_err());
    }
    let mut broken = original;
    let at = broken.windows(4).position(|b| b == b"inst").unwrap() + 8 + 20;
    broken[at..at + 2].copy_from_slice(&65535u16.to_le_bytes());
    std::fs::write(&path, broken).unwrap();
    assert!(
        load_sample_bank(&path)
            .unwrap_err()
            .to_string()
            .contains("zone span")
    );
}
