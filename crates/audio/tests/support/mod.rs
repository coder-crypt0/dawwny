//! Small generated SF2 fixture. These waveforms are test signals, not acoustic presets.
use std::{fs, path::Path};
fn chunk(id: &[u8; 4], data: Vec<u8>) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}
fn list(kind: &[u8; 4], chunks: Vec<Vec<u8>>) -> Vec<u8> {
    let mut data = kind.to_vec();
    for c in chunks {
        data.extend(c);
    }
    chunk(b"LIST", data)
}
fn name(value: &str) -> Vec<u8> {
    let mut data = vec![0; 20];
    data[..value.len()].copy_from_slice(value.as_bytes());
    data
}
fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
pub fn bank(path: &Path) {
    let mut phdr = Vec::new();
    for (label, program, bank, zone) in [
        ("Test mellow", 0, 0, 0),
        ("Test bright", 24, 0, 1),
        ("Test kit", 0, 128, 2),
        ("EOP", 0, 0, 3),
    ] {
        phdr.extend(name(label));
        phdr.extend(words(&[program, bank, zone]));
        phdr.extend([0; 12]);
    }
    let mut inst = name("Test signals");
    inst.extend(words(&[0]));
    inst.extend(name("EOI"));
    inst.extend(words(&[2]));
    let mut samples = Vec::new();
    for harmonic in [1.0_f32, 2.0] {
        for i in 0..256 {
            let s = (std::f32::consts::TAU * i as f32 / 256.0 * harmonic).sin() * 18000.0;
            samples.extend_from_slice(&(s as i16).to_le_bytes());
        }
        samples.extend([0; 92]);
    }
    let mut shdr = Vec::new();
    for (label, start, end) in [
        ("Signal A", 0u32, 256u32),
        ("Signal B", 302, 558),
        ("EOS", 604, 604),
    ] {
        shdr.extend(name(label));
        for value in [start, end, start, end, 16000] {
            shdr.extend_from_slice(&value.to_le_bytes());
        }
        shdr.extend([60, 0]);
        shdr.extend(words(&[0, 1]));
    }
    let mut igen = Vec::new();
    // Velocity layers exercise actual SF2 zone routing. Loop and 0.0625 second release.
    for (lo, hi, sample) in [(0u16, 79u16, 0u16), (80, 127, 1)] {
        igen.extend(words(&[
            44,
            lo | (hi << 8),
            54,
            1,
            38,
            (-4800i16) as u16,
            53,
            sample,
        ]));
    }
    igen.extend(words(&[0, 0]));
    let data = [
        list(
            b"INFO",
            vec![
                chunk(b"ifil", words(&[2, 1])),
                chunk(b"INAM", b"Generated test bank\0".to_vec()),
            ],
        ),
        list(b"sdta", vec![chunk(b"smpl", samples)]),
        list(
            b"pdta",
            vec![
                chunk(b"phdr", phdr),
                chunk(b"pbag", words(&[0, 0, 1, 0, 2, 0, 3, 0])),
                chunk(b"pmod", vec![0; 10]),
                chunk(b"pgen", words(&[41, 0, 41, 0, 41, 0, 0, 0])),
                chunk(b"inst", inst),
                chunk(b"ibag", words(&[0, 0, 4, 0, 8, 0])),
                chunk(b"imod", vec![0; 10]),
                chunk(b"igen", igen),
                chunk(b"shdr", shdr),
            ],
        ),
    ];
    let mut riff = b"sfbk".to_vec();
    for c in data {
        riff.extend(c);
    }
    fs::write(path, chunk(b"RIFF", riff)).unwrap();
}
pub fn project(path: &Path) -> dawwny_core::Project {
    let mut p = dawwny_core::demo_project();
    p.tracks.truncate(1);
    p.sections.clear();
    p.length_bars = 1;
    p.tempo = 120.0;
    let t = &mut p.tracks[0];
    t.instrument = dawwny_core::Instrument::Sampler;
    t.patch = dawwny_core::SynthPatch {
        reverb: 0.0,
        delay: 0.0,
        sample: Some(dawwny_core::SampleInstrument {
            file: path.to_str().unwrap().into(),
            bank: 0,
            program: 0,
        }),
        ..Default::default()
    };
    t.clips = vec![dawwny_core::Clip {
        id: "clip".into(),
        name: "Test".into(),
        start: 0.0,
        length: 4.0,
        notes: vec![dawwny_core::Note {
            id: "note".into(),
            pitch: 60,
            start: 0.0,
            duration: 1.0,
            velocity: 0.5,
        }],
    }];
    p
}
