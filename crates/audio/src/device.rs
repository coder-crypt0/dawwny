use crate::{LiveEvent, Renderer, compile, dsp::MixSnapshot};
use anyhow::{Context, Result, bail, ensure};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dawwny_core::Project;
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
}
struct Runtime {
    arrangement: Box<Renderer>,
    monitor: Box<Renderer>,
    commands: Consumer<Command>,
    retired: Producer<Box<Renderer>>,
    midi: Consumer<LiveEvent>,
    state: Arc<Telemetry>,
    tail: u64,
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
                Command::Live(event) => self.monitor.live_event(event),
            }
        }
        for _ in 0..256 {
            let Ok(event) = self.midi.pop() else {
                break;
            };
            self.monitor.live_event(event);
        }
        if self.state.panic.swap(false, Ordering::AcqRel) {
            self.monitor.live_event(LiveEvent::AllOff);
            self.tail = 0;
        }
    }
    fn next_frame(&mut self) -> [f32; 2] {
        let mut stereo = [0.0; 2];
        if self.state.playing.load(Ordering::Relaxed) {
            if self.state.looped.load(Ordering::Relaxed)
                && self.arrangement.frame() >= self.arrangement.song_frames()
            {
                self.arrangement.rewind();
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
        Ok(Self {
            stream: None,
            state: Arc::default(),
            commands: None,
            retired: None,
            midi_events: Arc::new(Mutex::new(producer)),
            midi_consumer: Some(consumer),
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
            let runtime = Runtime {
                arrangement: Box::new(Renderer::new(compile(project, self.rate)?)),
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
        if self.position_beats() >= project.length_bars as f64 * 4.0 {
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
        (
            Runtime {
                arrangement: Box::new(Renderer::new(compile(&p, 16000).unwrap())),
                monitor: AudioEngine::monitor(&p, 0, 16000).unwrap(),
                commands,
                retired,
                midi,
                state: Arc::default(),
                tail: 0,
            },
            producer,
            consumer,
        )
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
