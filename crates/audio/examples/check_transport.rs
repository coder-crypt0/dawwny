//! Opt-in hardware smoke check; plays a quiet demo and exercises the real CPAL stream.
use anyhow::{Result, ensure};
use dawwny_audio::{AudioEngine, LiveEvent};
use std::{thread::sleep, time::Duration};
fn wait() {
    sleep(Duration::from_millis(150));
}
fn main() -> Result<()> {
    let mut project = dawwny_core::demo_project();
    project.master_gain = 0.02;
    if let Some(file) = std::env::args().nth(1) {
        let file = std::fs::canonicalize(file)?.to_string_lossy().into_owned();
        for track in &mut project.tracks {
            use dawwny_core::Instrument;
            let (bank, program) = match track.instrument {
                Instrument::Keys => (0, 0),
                Instrument::Pad => (0, 48),
                Instrument::Bass => (0, 32),
                Instrument::Drums => (128, 0),
                _ => (0, 24),
            };
            track.instrument = Instrument::Sampler;
            track.patch.sample = Some(dawwny_core::SampleInstrument {
                file: file.clone(),
                bank,
                program,
            });
        }
    }
    let mut engine = AudioEngine::new()?;
    engine.play(&project, true)?;
    wait();
    ensure!(
        engine.position_beats() > 0.0,
        "Device clock did not advance"
    );
    engine.pause();
    wait();
    let paused = engine.position_beats();
    wait();
    ensure!(
        (engine.position_beats() - paused).abs() < 1e-6,
        "Paused transport advanced"
    );
    engine.seek_beats(8.0)?;
    wait();
    ensure!(
        (engine.position_beats() - 8.0).abs()
            <= project.tempo / (60.0 * engine.sample_rate() as f64),
        "Seek did not reach beat eight: {:.12} at {} Hz",
        engine.position_beats(),
        engine.sample_rate()
    );
    engine.play(&project, true)?;
    wait();
    ensure!(engine.position_beats() > 8.0, "Resume rewound the project");
    let before = engine.position_beats();
    project.tracks[0].mute = true;
    engine.update_project(&project, 0)?;
    wait();
    ensure!(
        engine.position_beats() > before,
        "Mute reset or stopped transport"
    );
    let before = engine.position_beats();
    project.tracks[1].solo = true;
    project.tracks[1].pan = 0.5;
    engine.update_project(&project, 0)?;
    wait();
    ensure!(engine.position_beats() > before, "Solo/pan reset transport");
    let before = engine.position_beats();
    project.tempo = 120.0;
    engine.update_project(&project, 0)?;
    wait();
    ensure!(
        engine.position_beats() >= before,
        "Tempo edit lost the musical position"
    );
    engine.pause();
    wait();
    let before = engine.position_beats();
    engine.live_event(LiveEvent::NoteOn {
        channel: 16,
        pitch: 60,
        velocity: 0.7,
    })?;
    wait();
    ensure!(
        engine.is_monitoring() && engine.peak() > 0.0,
        "Live instrument produced no output"
    );
    ensure!(
        (engine.position_beats() - before).abs() < 1e-6,
        "Live input advanced the arrangement"
    );
    engine.live_event(LiveEvent::NoteOff {
        channel: 16,
        pitch: 60,
    })?;
    engine.stop()?;
    wait();
    ensure!(
        engine.position_beats() == 0.0,
        "Stop did not return to start"
    );
    engine.collect_retired();
    println!(
        "PASS: real device clock, pause/resume, seeking, mute/solo/pan, tempo replacement, live input and stop; {} Hz; MIDI inputs: {}",
        engine.sample_rate(),
        AudioEngine::midi_ports()?.len()
    );
    Ok(())
}
