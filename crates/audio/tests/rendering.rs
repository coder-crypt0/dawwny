use dawwny_audio::{Renderer, compile, render_wav};
use dawwny_core::*;

fn project() -> Project {
    Project {
        length_bars: 1,
        tempo: 120.0,
        tracks: vec![Track {
            id: "track".into(),
            name: "Keys".into(),
            color: [100, 120, 140],
            instrument: Instrument::Keys,
            gain: 0.7,
            pan: 0.0,
            mute: false,
            solo: false,
            patch: SynthPatch {
                reverb: 0.0,
                ..Default::default()
            },
            clips: vec![Clip {
                id: "clip".into(),
                name: "Phrase".into(),
                start: 0.0,
                length: 4.0,
                notes: vec![Note {
                    id: "note".into(),
                    pitch: 60,
                    start: 0.0,
                    duration: 0.5,
                    velocity: 0.8,
                }],
            }],
        }],
        ..Default::default()
    }
}
fn samples(p: &Project) -> Vec<[f32; 2]> {
    let mut r = Renderer::new(compile(p, 16000).unwrap());
    (0..16000).map(|_| r.next_frame()).collect()
}
#[test]
fn renders_real_finite_audio_and_mute_silence() {
    let mut p = project();
    let a = samples(&p);
    assert!(a.iter().any(|v| v[0].abs() > 0.01));
    assert!(a.iter().flatten().all(|v| v.is_finite() && v.abs() <= 0.95));
    p.tracks[0].mute = true;
    assert!(samples(&p).iter().flatten().all(|v| *v == 0.0));
}
#[test]
fn every_sound_parameter_affects_the_render() {
    let p = project();
    let original = samples(&p);
    for param in 0..7 {
        let mut changed = p.clone();
        let s = &mut changed.tracks[0].patch;
        match param {
            0 => s.attack = 0.3,
            1 => s.decay = 0.01,
            2 => s.sustain = 0.1,
            3 => s.release = 0.9,
            4 => s.cutoff = 120.0,
            5 => s.reverb = 0.8,
            _ => s.delay = 0.8,
        }
        let difference: f32 = original
            .iter()
            .zip(samples(&changed))
            .map(|(a, b)| (a[0] - b[0]).abs())
            .sum();
        assert!(difference > 0.01, "parameter {param} is disconnected");
    }
}
#[test]
fn solo_and_pan_are_applied() {
    let mut p = project();
    let original = samples(&p);
    let mut other = p.tracks[0].clone();
    other.id = "other".into();
    other.clips[0].id = "other-clip".into();
    other.clips[0].notes[0].id = "other-note".into();
    other.clips[0].notes[0].pitch = 72;
    p.tracks[0].solo = true;
    p.tracks.push(other);
    assert_eq!(original, samples(&p));
    p.tracks[0].pan = -1.0;
    assert!(samples(&p).iter().all(|v| v[1].abs() < 1e-7));
}
#[test]
fn metadata_memory_does_not_scale_with_song_duration() {
    let mut p = project();
    let small = compile(&p, 48000).unwrap();
    p.length_bars = 256;
    let long = compile(&p, 48000).unwrap();
    assert_eq!(small.event_count(), 1);
    assert_eq!(long.event_count(), 1);
    assert_eq!(small.event_storage_bytes(), long.event_storage_bytes());
    assert!(small.event_storage_bytes() < 1024);
}
#[test]
fn wav_has_correct_size_tail_and_peak() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("audio.wav");
    let p = project();
    let stats = render_wav(&p, &file, 16000).unwrap();
    let mut reader = hound::WavReader::open(file).unwrap();
    assert_eq!(reader.spec().bits_per_sample, 24);
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.duration() as u64, stats.frames);
    assert!(stats.frames > 32000);
    assert!(stats.peak > 0.01 && stats.peak < 1.0);
    assert_eq!(stats.stolen_voices, 0);
    assert!(reader.samples::<i32>().any(|s| s.unwrap() != 0));
}
#[test]
fn voice_overload_remains_bounded() {
    let mut p = project();
    let original = p.tracks[0].clips[0].notes[0].clone();
    for i in 1..140 {
        let mut n = original.clone();
        n.id = format!("n-{i}");
        p.tracks[0].clips[0].notes.push(n);
    }
    let mut r = Renderer::new(compile(&p, 16000).unwrap());
    for _ in 0..1024 {
        assert!(r.next_frame().into_iter().all(f32::is_finite));
    }
    assert_eq!(r.stolen_voices(), 12);
}

#[test]
fn dawn_controls_change_real_rendered_audio() {
    let mut p = project();
    p.tracks[0].instrument = Instrument::Synth;
    p.tracks[0].clips[0].notes[0].duration = 2.0;
    p.tracks[0].patch.synth.osc1.waveform = Waveform::Pulse;
    let original = samples(&p);
    for parameter in 0..16 {
        let mut changed = p.clone();
        let s = &mut changed.tracks[0].patch.synth;
        match parameter {
            0 => s.osc1.waveform = Waveform::Triangle,
            1 => s.osc1.level = 0.1,
            2 => s.osc1.semitones = 12,
            3 => s.osc1.detune_cents = 45.0,
            4 => s.osc1.pulse_width = 0.2,
            5 => s.osc2.waveform = Waveform::Sine,
            6 => s.osc2.level = 0.0,
            7 => s.osc2.semitones = -12,
            8 => s.osc2.detune_cents = -50.0,
            9 => s.sub_level = 0.5,
            10 => s.noise_level = 0.2,
            11 => s.filter_mode = FilterMode::HighPass,
            12 => s.resonance = 0.8,
            13 => s.filter_env = -3.0,
            14 => {
                s.lfo_pitch = 1.0;
                s.lfo_rate = 3.0;
            }
            _ => {
                s.lfo_filter = 2.0;
                s.lfo_rate = 2.0;
            }
        }
        let changed = samples(&changed);
        let difference: f32 = original
            .iter()
            .zip(&changed)
            .map(|(a, b)| (a[0] - b[0]).abs())
            .sum();
        assert!(
            difference > 0.01,
            "disconnected synth parameter {parameter}"
        );
        assert!(
            changed
                .iter()
                .flatten()
                .all(|s| s.is_finite() && s.abs() <= 0.95)
        );
    }
}
#[test]
fn extreme_echo_racks_are_rejected_before_large_allocations() {
    let mut p = project();
    p.tracks[0].patch.effects = vec![
        EffectSlot {
            enabled: true,
            effect: Effect::Echo {
                beats: 4.0,
                feedback: 0.85,
                damping: 0.5,
                ping_pong: true,
                mix: 1.0
            }
        };
        8
    ];
    p.tempo = 30.0;
    let err = compile(&p, 192000).unwrap_err().to_string();
    assert!(err.contains("64 MiB"));
    for slot in &mut p.tracks[0].patch.effects {
        slot.enabled = false;
    }
    assert!(compile(&p, 192000).is_ok());
}
