use crate::{LiveEvent, Renderer, compile, dsp::MixSnapshot};
use anyhow::{Context, Result, bail, ensure};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dawwny_core::{CycleRange, Project};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

#[derive(Default)]
struct Telemetry {
    playing: AtomicBool,
    beat: AtomicU64,
    peak: AtomicU32,
    looped: AtomicBool,
    panic: AtomicBool,
    monitoring: AtomicBool,
    device_error: AtomicBool,
    recording: AtomicBool,
    recording_overflow: AtomicBool,
    recording_end: AtomicU64,
}
// Fixed-size mix snapshots avoid allocating for fader moves on either thread.
#[allow(clippy::large_enum_variant)]
enum Command {
    Replace {
        renderer: Box<Renderer>,
        preserve: bool,
    },
    Monitor(Box<Renderer>),
    Mix(MixSnapshot),
    MonitorMix(MixSnapshot),
    Live(LiveEvent),
    Cycle(Option<CycleRange>),
    Capture(bool),
}
#[derive(Clone, Copy, Debug)]
pub struct RecordedEvent {
    pub beat: f64,
    pub event: LiveEvent,
}

struct Runtime {
    arrangement: Box<Renderer>,
    monitor: Box<Renderer>,
    commands: Consumer<Command>,
    retired: Producer<Box<Renderer>>,
    midi: Consumer<LiveEvent>,
    state: Arc<Telemetry>,
    tail: u64,
    cycle: Option<CycleRange>,
    cycle_frames: Option<(u64, u64)>,
    recording: bool,
    recorded: Producer<RecordedEvent>,
}
impl Runtime {
    fn begin_block(&mut self) {
        // Leave commands queued when retired storage is full. The control thread reclaims it.
        for _ in 0..32 {
            if self.retired.is_full() {
                break;
            }
            let Ok(command) = self.commands.pop() else {
                break;
            };
            match command {
                Command::Replace {
                    mut renderer,
                    preserve,
                } => {
                    if preserve {
                        renderer.seek_beats(self.arrangement.position_beats());
                    }
                    let old = std::mem::replace(&mut self.arrangement, renderer);
                    self.update_cycle_frames();
                    // Capacity was checked above; only this thread writes to this queue.
                    let _ = self.retired.push(old);
                }
                Command::Monitor(renderer) => {
                    let old = std::mem::replace(&mut self.monitor, renderer);
                    let _ = self.retired.push(old);
                    self.tail = 0;
                }
                Command::Mix(mix) => self.arrangement.apply_mix(mix),
                Command::MonitorMix(mix) => self.monitor.apply_mix(mix),
                Command::Live(event) => {
                    self.capture(event);
                    self.monitor.live_event(event);
                }
                Command::Capture(enabled) => {
                    if !enabled {
                        self.midi_input();
                    }
                    self.recording = enabled;
                    if !enabled {
                        self.state.recording_end.store(
                            self.arrangement.position_beats().to_bits(),
                            Ordering::Relaxed,
                        );
                        self.state.recording.store(false, Ordering::Release);
                    }
                }
                Command::Cycle(range) => {
                    self.cycle = range;
                    self.update_cycle_frames();
                }
            }
        }
        self.midi_input();
        if self.state.panic.swap(false, Ordering::AcqRel) {
            self.capture(LiveEvent::AllOff);
            self.monitor.live_event(LiveEvent::AllOff);
            self.tail = 0;
        }
    }
    fn midi_input(&mut self) {
        for _ in 0..256 {
            let Ok(event) = self.midi.pop() else {
                break;
            };
            self.capture(event);
            self.monitor.live_event(event);
        }
    }
    fn capture(&mut self, event: LiveEvent) {
        if self.recording
            && self.state.recording.load(Ordering::Acquire)
            && self
                .recorded
                .push(RecordedEvent {
                    beat: self.arrangement.position_beats(),
                    event,
                })
                .is_err()
        {
            self.state.recording_overflow.store(true, Ordering::Release);
        }
    }
    fn update_cycle_frames(&mut self) {
        self.cycle_frames = self.cycle.map(|range| {
            (
                self.arrangement.frame_at_beat(range.start),
                self.arrangement.frame_at_beat(range.end),
            )
        });
    }
    fn next_frame(&mut self) -> [f32; 2] {
        let mut stereo = [0.0; 2];
        if self.state.playing.load(Ordering::Relaxed) {
            if self.state.looped.load(Ordering::Relaxed)
                && self.arrangement.frame()
                    >= self
                        .cycle_frames
                        .map_or(self.arrangement.song_frames(), |(_, end)| end)
            {
                if let Some(range) = self.cycle {
                    self.arrangement.seek_beats(range.start);
                } else {
                    self.arrangement.rewind();
                }
            }
            if self.arrangement.frame() < self.arrangement.total_frames() {
                stereo = self.arrangement.next_frame();
            } else {
                self.state.playing.store(false, Ordering::Relaxed);
            }
        }
        if self.monitor.has_live_voices() {
            self.tail = self.monitor.tail_frames().max(128);
        }
        if self.tail > 0 {
            let live = self.monitor.next_live_frame();
            self.tail -= 1;
            for i in 0..2 {
                stereo[i] = (stereo[i] + live[i]).clamp(-0.95, 0.95);
            }
        }
        stereo
    }
    fn publish(&self, peak: f32) {
        self.state.beat.store(
            self.arrangement.position_beats().to_bits(),
            Ordering::Relaxed,
        );
        self.state
            .monitoring
            .store(self.tail > 0, Ordering::Relaxed);
        let previous = f32::from_bits(self.state.peak.load(Ordering::Relaxed));
        self.state
            .peak
            .store(peak.max(previous * 0.9).to_bits(), Ordering::Relaxed);
    }
}

#[derive(Clone, Debug)]
pub struct MidiPort {
    pub index: usize,
    pub name: String,
}

pub struct AudioEngine {
    stream: Option<cpal::Stream>,
    state: Arc<Telemetry>,
    commands: Option<Producer<Command>>,
    retired: Option<Consumer<Box<Renderer>>>,
    midi_events: Arc<Mutex<Producer<LiveEvent>>>,
    midi_consumer: Option<Consumer<LiveEvent>>,
    recorded: Consumer<RecordedEvent>,
    recording_producer: Option<Producer<RecordedEvent>>,
    midi: Option<midir::MidiInputConnection<()>>,
    midi_name: Option<String>,
    project: Option<Project>,
    selected: usize,
    rate: u32,
    name: String,
}
impl AudioEngine {
    pub fn new() -> Result<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .context("No default output device")?;
        let config = device.default_output_config()?;
        let (producer, consumer) = RingBuffer::new(256);
        let (recording_producer, recorded) = RingBuffer::new(4096);
        Ok(Self {
            stream: None,
            state: Arc::default(),
            commands: None,
            retired: None,
            midi_events: Arc::new(Mutex::new(producer)),
            midi_consumer: Some(consumer),
            recorded,
            recording_producer: Some(recording_producer),
            midi: None,
            midi_name: None,
            project: None,
            selected: usize::MAX,
            rate: config.sample_rate().0,
            name: device
                .name()
                .unwrap_or_else(|_| "Default audio output".into()),
        })
    }
    pub fn collect_retired(&mut self) {
        if let Some(retired) = &mut self.retired {
            while let Ok(renderer) = retired.pop() {
                drop(renderer);
            }
        }
    }
    fn send(&mut self, command: Command) -> Result<()> {
        self.collect_retired();
        let queue = self
            .commands
            .as_mut()
            .context("Audio stream is not prepared")?;
        if queue.push(command).is_err() {
            self.state.panic.store(true, Ordering::Release);
            if self.state.recording.load(Ordering::Acquire) {
                self.state.recording_overflow.store(true, Ordering::Release);
            }
            bail!("Audio command queue is full; try again");
        }
        Ok(())
    }
    fn monitor(project: &Project, selected: usize, rate: u32) -> Result<Box<Renderer>> {
        let mut preview = Project {
            tempo: project.tempo,
            master_gain: project.master_gain,
            length_bars: 1,
            ..Project::default()
        };
        if let Some(track) = project.tracks.get(selected) {
            let mut track = track.clone();
            track.clips.clear();
            track.mute = false;
            track.solo = false;
            preview.tracks.push(track);
        }
        Ok(Box::new(Renderer::new(compile(&preview, rate)?)))
    }
    pub fn update_project(&mut self, project: &Project, selected: usize) -> Result<()> {
        dawwny_core::validate(project)?;
        self.collect_retired();
        if self.project.as_ref() == Some(project) && self.selected == selected {
            return Ok(());
        }
        if self.stream.is_none() {
            let device = cpal::default_host()
                .default_output_device()
                .context("No default output device")?;
            let supported = device.default_output_config()?;
            let config = supported.config();
            self.rate = config.sample_rate.0;
            let (producer, commands) = RingBuffer::new(32);
            let (retired, consumer) = RingBuffer::new(32);
            let arrangement = Box::new(Renderer::new(compile(project, self.rate)?));
            let cycle_frames = project.cycle.map(|range| {
                (
                    arrangement.frame_at_beat(range.start),
                    arrangement.frame_at_beat(range.end),
                )
            });
            let runtime = Runtime {
                arrangement,
                cycle: project.cycle,
                cycle_frames,
                recording: false,
                recorded: self
                    .recording_producer
                    .take()
                    .context("Recording stream is already prepared")?,
                monitor: Self::monitor(project, selected, self.rate)?,
                commands,
                retired,
                midi: self
                    .midi_consumer
                    .take()
                    .context("Restart the studio to reconnect audio")?,
                state: self.state.clone(),
                tail: 0,
            };
            let stream = match supported.sample_format() {
                cpal::SampleFormat::F32 => build::<f32>(&device, &config, runtime),
                cpal::SampleFormat::I16 => build::<i16>(&device, &config, runtime),
                cpal::SampleFormat::U16 => build::<u16>(&device, &config, runtime),
                other => bail!("Audio format {other:?} is not supported"),
            }?;
            stream.play()?;
            self.stream = Some(stream);
            self.commands = Some(producer);
            self.retired = Some(consumer);
        } else {
            let previous = self.project.as_ref().context("Missing audio project")?;
            let same_structure = audio_structure_equal(previous, project);
            let cycle_changed = previous.cycle != project.cycle;
            let monitor_changed = self.selected != selected
                || previous.tempo != project.tempo
                || previous
                    .tracks
                    .get(self.selected)
                    .map(|t| (t.instrument, &t.patch))
                    != project
                        .tracks
                        .get(selected)
                        .map(|t| (t.instrument, &t.patch));
            let arrangement = if same_structure {
                None
            } else {
                Some(Box::new(Renderer::new(compile(project, self.rate)?)))
            };
            let monitor = if monitor_changed {
                Some(Self::monitor(project, selected, self.rate)?)
            } else {
                None
            };
            if cycle_changed {
                self.send(Command::Cycle(project.cycle))?;
            }
            if let Some(renderer) = arrangement {
                self.send(Command::Replace {
                    renderer,
                    preserve: true,
                })?;
            } else {
                self.send(Command::Mix(MixSnapshot::new(project)))?;
            }
            if let Some(renderer) = monitor {
                self.send(Command::Monitor(renderer))?;
            } else {
                self.send(Command::MonitorMix(MixSnapshot::monitor(project, selected)))?;
            }
        }
        self.project = Some(project.clone());
        self.selected = selected;
        Ok(())
    }
    pub fn play(&mut self, project: &Project, looped: bool) -> Result<()> {
        let selected = if self.selected == usize::MAX {
            0
        } else {
            self.selected
        };
        self.update_project(project, selected)?;
        self.set_looped(looped);
        if looped && let Some(range) = project.cycle {
            if self.position_beats() < range.start || self.position_beats() >= range.end {
                self.seek_beats(range.start)?;
            }
        } else if self.position_beats() >= project.length_bars as f64 * 4.0 {
            self.seek_beats(0.0)?;
        }
        self.state.playing.store(true, Ordering::Release);
        Ok(())
    }
    pub fn pause(&mut self) {
        self.state.playing.store(false, Ordering::Release);
    }
    pub fn stop(&mut self) -> Result<()> {
        self.pause();
        self.all_notes_off();
        if self.stream.is_some() {
            self.seek_beats(0.0)?;
        }
        Ok(())
    }
    pub fn seek_beats(&mut self, beat: f64) -> Result<()> {
        ensure!(beat.is_finite(), "Playhead position must be finite");
        let project = self
            .project
            .as_ref()
            .context("No audio session is prepared")?;
        let beat = beat.clamp(0.0, project.length_bars as f64 * 4.0);
        let mut renderer = Box::new(Renderer::new(compile(project, self.rate)?));
        renderer.seek_beats(beat);
        self.send(Command::Replace {
            renderer,
            preserve: false,
        })?;
        self.state.beat.store(beat.to_bits(), Ordering::Release);
        Ok(())
    }
    pub fn start_recording(&mut self) -> Result<()> {
        while self.recorded.pop().is_ok() {}
        self.state
            .recording_overflow
            .store(false, Ordering::Release);
        self.state.recording.store(true, Ordering::Release);
        if let Err(error) = self.send(Command::Capture(true)) {
            self.state.recording.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(())
    }
    /// Finish at an audio-block boundary before draining the final captured events.
    pub fn finish_recording(&mut self) -> Result<f64> {
        let result = self.send(Command::Capture(false));
        if let Err(error) = result {
            self.state.recording.store(false, Ordering::Release);
            return Err(error);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
        while self.state.recording.load(Ordering::Acquire) {
            if self.device_failed() || std::time::Instant::now() >= deadline {
                self.state.recording.store(false, Ordering::Release);
                bail!("The audio device did not acknowledge the end of the recording");
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        Ok(f64::from_bits(
            self.state.recording_end.load(Ordering::Relaxed),
        ))
    }
    pub fn recorded_event(&mut self) -> Option<RecordedEvent> {
        self.recorded.pop().ok()
    }
    pub fn recording_overflowed(&self) -> bool {
        self.state.recording_overflow.load(Ordering::Acquire)
    }
    pub fn live_event(&mut self, event: LiveEvent) -> Result<()> {
        self.send(Command::Live(event))
    }
    pub fn all_notes_off(&mut self) {
        self.state.panic.store(true, Ordering::Release);
    }
    pub fn set_looped(&self, looped: bool) {
        self.state.looped.store(looped, Ordering::Relaxed);
    }
    pub fn is_playing(&self) -> bool {
        self.state.playing.load(Ordering::Relaxed)
    }
    pub fn is_monitoring(&self) -> bool {
        self.state.monitoring.load(Ordering::Relaxed)
    }
    pub fn position_beats(&self) -> f64 {
        f64::from_bits(self.state.beat.load(Ordering::Acquire))
    }
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.state.peak.load(Ordering::Relaxed))
    }
    pub fn device_failed(&self) -> bool {
        self.state.device_error.load(Ordering::Relaxed)
    }
    pub fn device_name(&self) -> &str {
        &self.name
    }
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }
    pub fn midi_name(&self) -> Option<&str> {
        self.midi_name.as_deref()
    }
    pub fn midi_ports() -> Result<Vec<MidiPort>> {
        let input = midir::MidiInput::new("Dawwny MIDI")?;
        input
            .ports()
            .iter()
            .enumerate()
            .map(|(index, port)| {
                Ok(MidiPort {
                    index,
                    name: input.port_name(port)?,
                })
            })
            .collect()
    }
    pub fn disconnect_midi(&mut self) {
        self.midi.take();
        self.midi_name = None;
        self.all_notes_off();
    }
    pub fn connect_midi(&mut self, index: usize) -> Result<()> {
        let mut input = midir::MidiInput::new("Dawwny MIDI")?;
        input.ignore(midir::Ignore::TimeAndActiveSense);
        let ports = input.ports();
        let port = ports
            .get(index)
            .context("MIDI input is no longer available; refresh devices")?;
        let name = input.port_name(port)?;
        self.disconnect_midi();
        let events = self.midi_events.clone();
        let state = self.state.clone();
        self.midi = Some(
            input
                .connect(
                    port,
                    "Dawwny input",
                    move |_, message, _| {
                        if let Some(event) = decode_midi(message) {
                            match events.lock() {
                                Ok(mut queue) => {
                                    if queue.push(event).is_err() {
                                        if state.recording.load(Ordering::Acquire) {
                                            state.recording_overflow.store(true, Ordering::Release);
                                        }
                                        state.panic.store(true, Ordering::Release);
                                    }
                                }
                                Err(_) => state.panic.store(true, Ordering::Release),
                            }
                        }
                    },
                    (),
                )
                .map_err(|e| anyhow::anyhow!("MIDI connection failed: {e}"))?,
        );
        self.midi_name = Some(name);
        Ok(())
    }
}
fn audio_structure_equal(a: &Project, b: &Project) -> bool {
    a.tempo == b.tempo
        && a.length_bars == b.length_bars
        && a.tracks.len() == b.tracks.len()
        && a.tracks.iter().zip(&b.tracks).all(|(a, b)| {
            a.id == b.id && a.instrument == b.instrument && a.patch == b.patch && a.clips == b.clips
        })
}
fn decode_midi(message: &[u8]) -> Option<LiveEvent> {
    if message.len() < 3 || message[1] > 127 || message[2] > 127 {
        return None;
    }
    let channel = message[0] & 15;
    match message[0] & 0xf0 {
        0x90 if message[2] > 0 => Some(LiveEvent::NoteOn {
            channel,
            pitch: message[1],
            velocity: message[2] as f32 / 127.0,
        }),
        0x80 | 0x90 => Some(LiveEvent::NoteOff {
            channel,
            pitch: message[1],
        }),
        0xb0 if message[1] == 64 => Some(LiveEvent::Sustain {
            channel,
            down: message[2] >= 64,
        }),
        0xb0 if matches!(message[1], 120 | 123) => Some(LiveEvent::AllOff),
        _ => None,
    }
}
fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut runtime: Runtime,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let failure = runtime.state.clone();
    Ok(device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            runtime.begin_block();
            let mut peak = 0.0_f32;
            for frame in output.chunks_mut(channels) {
                let stereo = runtime.next_frame();
                peak = peak.max(stereo[0].abs()).max(stereo[1].abs());
                for (i, sample) in frame.iter_mut().enumerate() {
                    let value = if channels == 1 {
                        (stereo[0] + stereo[1]) * 0.5
                    } else if i < 2 {
                        stereo[i]
                    } else {
                        0.0
                    };
                    *sample = T::from_sample(value);
                }
            }
            runtime.publish(peak);
        },
        move |_| {
            failure.device_error.store(true, Ordering::Relaxed);
            failure.playing.store(false, Ordering::Relaxed);
        },
        None,
    )?)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
    };
    thread_local! {static TRACK:Cell<bool>=const{Cell::new(false)};static COUNT:Cell<usize>=const{Cell::new(0)};}
    struct Counted;
    unsafe impl GlobalAlloc for Counted {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            TRACK.with(|t| {
                if t.get() {
                    COUNT.with(|c| c.set(c.get() + 1));
                }
            });
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            TRACK.with(|t| {
                if t.get() {
                    COUNT.with(|c| c.set(c.get() + 1));
                }
            });
            unsafe { System.dealloc(ptr, layout) }
        }
    }
    #[global_allocator]
    static ALLOCATOR: Counted = Counted;
    fn runtime() -> (Runtime, Producer<Command>, Consumer<Box<Renderer>>) {
        let p = dawwny_core::demo_project();
        let (producer, commands) = RingBuffer::new(32);
        let (retired, consumer) = RingBuffer::new(32);
        let (_, midi) = RingBuffer::new(256);
        let (recorded, _) = RingBuffer::new(4096);
        (
            Runtime {
                arrangement: Box::new(Renderer::new(compile(&p, 16000).unwrap())),
                monitor: AudioEngine::monitor(&p, 0, 16000).unwrap(),
                commands,
                retired,
                midi,
                state: Arc::default(),
                tail: 0,
                cycle: None,
                cycle_frames: None,
                recording: false,
                recorded,
            },
            producer,
            consumer,
        )
    }
    #[test]
    fn recording_timestamps_input_on_the_audio_clock_without_callback_allocations() {
        let (mut runtime, mut commands, _retired) = runtime();
        let (recorded, mut events) = RingBuffer::new(4);
        runtime.recorded = recorded;
        runtime.state.recording.store(true, Ordering::Release);
        runtime.state.playing.store(true, Ordering::Release);
        commands.push(Command::Capture(true)).ok().unwrap();
        commands
            .push(Command::Live(LiveEvent::NoteOn {
                channel: 16,
                pitch: 60,
                velocity: 0.7,
            }))
            .ok()
            .unwrap();
        COUNT.with(|c| c.set(0));
        TRACK.with(|t| t.set(true));
        runtime.begin_block();
        for _ in 0..16000 {
            std::hint::black_box(runtime.next_frame());
        }
        TRACK.with(|t| t.set(false));
        assert_eq!(COUNT.with(Cell::get), 0);
        let note = events.pop().unwrap();
        assert_eq!(note.beat, 0.0);
        assert!(matches!(note.event, LiveEvent::NoteOn { pitch: 60, .. }));
        let end = runtime.arrangement.position_beats();
        let (mut midi, input) = RingBuffer::new(4);
        runtime.midi = input;
        midi.push(LiveEvent::Sustain {
            channel: 0,
            down: true,
        })
        .ok()
        .unwrap();
        commands
            .push(Command::Live(LiveEvent::NoteOff {
                channel: 16,
                pitch: 60,
            }))
            .ok()
            .unwrap();
        commands.push(Command::Capture(false)).ok().unwrap();
        commands
            .push(Command::Live(LiveEvent::NoteOn {
                channel: 16,
                pitch: 64,
                velocity: 0.5,
            }))
            .ok()
            .unwrap();
        runtime.begin_block();
        let off = events.pop().unwrap();
        assert_eq!(off.beat, end);
        assert!(matches!(off.event, LiveEvent::NoteOff { pitch: 60, .. }));
        let pedal = events.pop().unwrap();
        assert_eq!(pedal.beat, end);
        assert!(matches!(pedal.event, LiveEvent::Sustain { down: true, .. }));
        assert!(events.pop().is_err());
        assert!(!runtime.state.recording.load(Ordering::Acquire));
        assert_eq!(
            f64::from_bits(runtime.state.recording_end.load(Ordering::Relaxed)),
            end
        );
        let (recorded, _events) = RingBuffer::new(1);
        runtime.recorded = recorded;
        runtime.recording = true;
        runtime.state.recording.store(true, Ordering::Release);
        runtime.capture(LiveEvent::AllOff);
        runtime.capture(LiveEvent::AllOff);
        assert!(runtime.state.recording_overflow.load(Ordering::Acquire));
    }
    #[test]
    fn cycle_wraps_at_the_exact_frame_and_survives_pause_mix_and_tempo_changes() {
        let (mut runtime, mut commands, _retired) = runtime();
        let range = CycleRange {
            start: 8.0,
            end: 8.25,
        };
        commands.push(Command::Cycle(Some(range))).ok().unwrap();
        runtime.begin_block();
        runtime.state.looped.store(true, Ordering::Relaxed);
        runtime.state.playing.store(true, Ordering::Relaxed);
        let (start, end) = runtime.cycle_frames.unwrap();
        runtime
            .arrangement
            .seek_beats(range.end - 92.0 / (60.0 * 16000.0));
        assert_eq!(runtime.arrangement.frame(), end - 1);
        runtime.next_frame();
        assert_eq!(runtime.arrangement.frame(), end);
        COUNT.with(|c| c.set(0));
        TRACK.with(|t| t.set(true));
        for _ in 0..(end - start) * 3 + 1 {
            std::hint::black_box(runtime.next_frame());
        }
        TRACK.with(|t| t.set(false));
        assert_eq!(COUNT.with(Cell::get), 0);
        assert_eq!(runtime.arrangement.frame(), start + 1);
        let mut project = dawwny_core::demo_project();
        project.tracks[0].mute = true;
        commands
            .push(Command::Mix(MixSnapshot::new(&project)))
            .ok()
            .unwrap();
        runtime.begin_block();
        assert_eq!(runtime.arrangement.frame(), start + 1);
        runtime.state.playing.store(false, Ordering::Relaxed);
        for _ in 0..100 {
            runtime.next_frame();
        }
        assert_eq!(runtime.arrangement.frame(), start + 1);
        let beat = runtime.arrangement.position_beats();
        project.tempo = 120.0;
        commands
            .push(Command::Replace {
                renderer: Box::new(Renderer::new(compile(&project, 16000).unwrap())),
                preserve: true,
            })
            .ok()
            .unwrap();
        runtime.begin_block();
        assert!((runtime.arrangement.position_beats() - beat).abs() < 0.0002);
        let (start, end) = runtime.cycle_frames.unwrap();
        assert_eq!(end - start, 2000);
        runtime.arrangement.seek_beats(range.end);
        runtime.state.looped.store(false, Ordering::Relaxed);
        runtime.state.playing.store(true, Ordering::Relaxed);
        runtime.next_frame();
        assert_eq!(runtime.arrangement.frame(), end + 1);
        runtime.state.looped.store(true, Ordering::Relaxed);
        runtime.next_frame();
        assert_eq!(runtime.arrangement.frame(), start + 1);
        commands.push(Command::Cycle(None)).ok().unwrap();
        runtime.begin_block();
        assert_eq!(runtime.arrangement.frame(), start + 1);
        assert_eq!(runtime.cycle_frames, None);
        runtime
            .arrangement
            .seek_beats(project.length_bars as f64 * 4.0);
        runtime.next_frame();
        assert_eq!(runtime.arrangement.frame(), 1);
    }
    #[test]
    fn pause_mix_and_graph_changes_keep_the_transport_position() {
        let (mut runtime, mut commands, _retired) = runtime();
        runtime.state.playing.store(true, Ordering::Relaxed);
        for _ in 0..16000 {
            runtime.next_frame();
        }
        let position = runtime.arrangement.frame();
        runtime.state.playing.store(false, Ordering::Relaxed);
        for _ in 0..1000 {
            assert_eq!(runtime.next_frame(), [0.0; 2]);
        }
        assert_eq!(runtime.arrangement.frame(), position);
        let mut project = dawwny_core::demo_project();
        project.tracks[0].mute = true;
        commands
            .push(Command::Mix(MixSnapshot::new(&project)))
            .ok()
            .unwrap();
        runtime.begin_block();
        assert_eq!(runtime.arrangement.frame(), position);
        project.tempo = 120.0;
        commands
            .push(Command::Replace {
                renderer: Box::new(Renderer::new(compile(&project, 16000).unwrap())),
                preserve: true,
            })
            .ok()
            .unwrap();
        let beat = runtime.arrangement.position_beats();
        runtime.begin_block();
        assert!((runtime.arrangement.position_beats() - beat).abs() < 0.0002);
        runtime.state.playing.store(true, Ordering::Relaxed);
        runtime.next_frame();
        assert!(runtime.arrangement.position_beats() > beat);
        runtime.state.playing.store(false, Ordering::Relaxed);
        commands
            .push(Command::Live(LiveEvent::NoteOn {
                channel: 16,
                pitch: 60,
                velocity: 0.8,
            }))
            .ok()
            .unwrap();
        runtime.begin_block();
        let position = runtime.arrangement.frame();
        assert!((0..1600).any(|_| runtime.next_frame()[0].abs() > 0.001));
        assert_eq!(runtime.arrangement.frame(), position);
    }
    #[test]
    fn callback_swaps_seek_mix_and_input_neither_allocate_nor_free() {
        let (mut runtime, mut commands, _retired) = runtime();
        let project = dawwny_core::demo_project();
        let mut next = Box::new(Renderer::new(compile(&project, 16000).unwrap()));
        next.seek_beats(8.0);
        commands
            .push(Command::Replace {
                renderer: next,
                preserve: false,
            })
            .ok()
            .unwrap();
        commands
            .push(Command::Monitor(
                AudioEngine::monitor(&project, 0, 16000).unwrap(),
            ))
            .ok()
            .unwrap();
        commands
            .push(Command::Mix(MixSnapshot::new(&project)))
            .ok()
            .unwrap();
        commands
            .push(Command::Live(LiveEvent::NoteOn {
                channel: 16,
                pitch: 60,
                velocity: 0.8,
            }))
            .ok()
            .unwrap();
        COUNT.with(|c| c.set(0));
        TRACK.with(|t| t.set(true));
        runtime.begin_block();
        runtime.state.playing.store(true, Ordering::Relaxed);
        for _ in 0..16000 {
            std::hint::black_box(runtime.next_frame());
        }
        runtime.publish(0.1);
        TRACK.with(|t| t.set(false));
        assert_eq!(COUNT.with(Cell::get), 0);
        assert_eq!(
            runtime.arrangement.frame(),
            (8.0_f64 * 16000.0 * 60.0 / 92.0).round() as u64 + 16000
        );
    }
    #[test]
    fn midi_decodes_release_sustain_and_rejects_malformed_messages() {
        assert!(matches!(
            decode_midi(&[0x92, 60, 90]),
            Some(LiveEvent::NoteOn {
                channel: 2,
                pitch: 60,
                ..
            })
        ));
        assert!(matches!(
            decode_midi(&[0x92, 60, 0]),
            Some(LiveEvent::NoteOff {
                channel: 2,
                pitch: 60
            })
        ));
        assert!(matches!(
            decode_midi(&[0xb1, 64, 127]),
            Some(LiveEvent::Sustain {
                channel: 1,
                down: true
            })
        ));
        assert!(matches!(
            decode_midi(&[0xb0, 123, 0]),
            Some(LiveEvent::AllOff)
        ));
        assert!(decode_midi(&[0x90, 128, 1]).is_none());
        assert!(decode_midi(&[0x90, 60]).is_none());
    }
}
