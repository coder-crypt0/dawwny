use crate::{
    fx::{Rack, storage_bytes, tail_seconds},
    synth::{CompiledSynth, SynthVoice},
};
use anyhow::{Result, ensure};
use dawwny_core::{Instrument, Project, validate};
use std::{
    f32::consts::{FRAC_PI_4, TAU},
    path::Path,
};

pub const VOICE_LIMIT: usize = 128;
#[derive(Clone, Copy, Debug)]
struct Event {
    frame: u64,
    gate: u64,
    increment: f32,
    pitch: u8,
    velocity: f32,
    track: usize,
}
#[derive(Clone, Copy, Debug)]
struct Channel {
    instrument: Instrument,
    gain: f32,
    pan: [f32; 2],
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    filter: f32,
    reverb: f32,
    delay: f32,
    synth: CompiledSynth,
    rack: [Option<dawwny_core::EffectSlot>; 8],
}
#[derive(Clone, Debug)]
pub struct RenderPlan {
    events: Vec<Event>,
    tracks: Vec<Channel>,
    pub sample_rate: u32,
    pub song_frames: u64,
    pub total_frames: u64,
    pub tempo: f64,
    master: f32,
}
impl RenderPlan {
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    pub fn duration_seconds(&self) -> f64 {
        self.total_frames as f64 / self.sample_rate as f64
    }
    pub fn event_storage_bytes(&self) -> usize {
        self.events.capacity() * std::mem::size_of::<Event>()
    }
}
#[derive(Clone, Debug, Default)]
pub struct RenderStats {
    pub frames: u64,
    pub peak: f32,
    pub stolen_voices: u64,
}

pub fn compile(project: &Project, sample_rate: u32) -> Result<RenderPlan> {
    validate(project)?;
    ensure!(
        (8_000..=192_000).contains(&sample_rate),
        "Supported sample rates: 8000–192000 Hz"
    );
    let spb = sample_rate as f64 * 60.0 / project.tempo;
    let song_frames = (project.length_bars as f64 * 4.0 * spb).round() as u64;
    let mut tracks = vec![];
    let mut events = vec![];
    let mut tail = 0.05f32;
    let solo = project.tracks.iter().any(|t| t.solo);
    let mut effect_storage = 0;
    for track in &project.tracks {
        if track.mute || (solo && !track.solo) {
            continue;
        }
        let index = tracks.len();
        let patch = &track.patch;
        let angle = (track.pan + 1.0) * FRAC_PI_4;
        effect_storage += storage_bytes(&patch.effects, sample_rate, project.tempo);
        if patch.delay > 0.0 {
            effect_storage += (sample_rate as f64 * 60.0 / project.tempo * 0.75) as usize * 4;
        }
        if patch.reverb > 0.0 {
            effect_storage += (sample_rate as f32 * 0.16) as usize * 4;
        }
        ensure!(
            effect_storage <= 64 * 1024 * 1024,
            "Effect delay buffers exceed the 64 MiB session budget at this sample rate. Bypass slots or shorten echoes."
        );
        tracks.push(Channel {
            instrument: track.instrument,
            gain: track.gain,
            pan: [angle.cos(), angle.sin()],
            attack: patch.attack * sample_rate as f32,
            decay: patch.decay * sample_rate as f32,
            sustain: patch.sustain,
            release: patch.release * sample_rate as f32,
            filter: 1.0
                - (-TAU * patch.cutoff.min(sample_rate as f32 * 0.45) / sample_rate as f32).exp(),
            reverb: patch.reverb,
            delay: patch.delay,
            synth: CompiledSynth::new(&patch.synth, patch.cutoff, sample_rate),
            rack: std::array::from_fn(|i| patch.effects.get(i).copied()),
        });
        tail = tail.max(
            patch.release
                + tail_seconds(&patch.effects, project.tempo)
                + if patch.reverb > 0.0 || patch.delay > 0.0 {
                    3.0
                } else {
                    0.05
                },
        );
        for clip in &track.clips {
            for note in &clip.notes {
                if note.velocity <= 0.0 {
                    continue;
                }
                events.push(Event {
                    frame: ((clip.start + note.start) * spb).round() as u64,
                    gate: (note.duration * spb).round().max(1.0) as u64,
                    increment: TAU * 440.0 * 2.0f32.powf((note.pitch as f32 - 69.0) / 12.0)
                        / sample_rate as f32,
                    pitch: note.pitch,
                    velocity: note.velocity,
                    track: index,
                });
            }
        }
    }
    events.sort_by_key(|e| e.frame);
    Ok(RenderPlan {
        events,
        tracks,
        sample_rate,
        song_frames,
        total_frames: song_frames + (tail * sample_rate as f32).ceil() as u64,
        tempo: project.tempo,
        master: project.master_gain,
    })
}

#[derive(Clone, Copy, Default)]
struct Voice {
    active: bool,
    age: u64,
    gate: u64,
    phase: f32,
    phase2: f32,
    increment: f32,
    velocity: f32,
    pitch: u8,
    track: usize,
    filtered: f32,
    last_noise: f32,
    random: u32,
    synth: SynthVoice,
}
fn envelope(age: f32, c: &Channel) -> f32 {
    if age < c.attack {
        age / c.attack
    } else if age < c.attack + c.decay {
        1.0 - (1.0 - c.sustain) * (age - c.attack) / c.decay
    } else {
        c.sustain
    }
}
impl Voice {
    fn sample(&mut self, c: &Channel, rate: f32) -> f32 {
        let age = self.age as f32;
        let env = if self.age < self.gate {
            envelope(age, c)
        } else {
            envelope(self.gate as f32, c) * (1.0 - (age - self.gate as f32) / c.release).max(0.0)
        };
        if age >= self.gate as f32 + c.release {
            self.active = false;
            return 0.0;
        }
        let seconds = age / rate;
        if c.instrument == Instrument::Synth {
            let output = self
                .synth
                .sample(&c.synth, self.increment * rate / TAU, env);
            self.age += 1;
            return output * env * self.velocity;
        }
        let signal = match c.instrument {
            Instrument::Synth => unreachable!("Dawn synth handled above"),
            Instrument::Keys => {
                (self.phase.sin() + 0.22 * (self.phase * 2.0).sin() * (-seconds * 3.0).exp())
                    * (-seconds * 0.7).exp()
                    * 0.36
            }
            Instrument::Pad => {
                (self.phase.sin() * 0.62
                    + self.phase2.sin() * 0.28
                    + (self.phase * 2.0).sin() * 0.1)
                    * 0.3
            }
            Instrument::Bass => (self.phase.sin() * 0.88 + (self.phase * 2.0).sin() * 0.12) * 0.55,
            Instrument::Lead => {
                (self.phase.sin() * 0.78
                    + (self.phase * 2.0).sin() * 0.16
                    + (self.phase * 3.0).sin() * 0.06)
                    * 0.38
            }
            Instrument::Drums => {
                self.random ^= self.random << 13;
                self.random ^= self.random >> 17;
                self.random ^= self.random << 5;
                let noise = self.random as f32 / u32::MAX as f32 * 2.0 - 1.0;
                let high = noise - self.last_noise;
                self.last_noise = noise;
                match self.pitch {
                    35 | 36 => {
                        self.phase = (self.phase
                            + TAU * (48.0 + 110.0 * (-seconds * 35.0).exp()) / rate)
                            % TAU;
                        self.phase.sin() * (-seconds * 9.0).exp() * 0.85
                            + high * (-seconds * 150.0).exp() * 0.08
                    }
                    38 | 40 => {
                        (noise * 0.6 + (TAU * 185.0 * seconds).sin() * 0.25)
                            * (-seconds * 14.0).exp()
                            * 0.75
                    }
                    42 | 44 => high * (-seconds * 55.0).exp() * 0.24,
                    46 => high * (-seconds * 10.0).exp() * 0.22,
                    _ => (noise * 0.2 + self.phase.sin() * 0.5) * (-seconds * 12.0).exp() * 0.5,
                }
            }
        };
        if !(c.instrument == Instrument::Drums && matches!(self.pitch, 35 | 36)) {
            self.phase = (self.phase + self.increment) % TAU;
        }
        self.phase2 = (self.phase2 + self.increment * 1.003) % TAU;
        self.filtered += c.filter * (signal - self.filtered);
        // Flush subnormal state so long decays cannot trigger slow floating-point paths.
        if self.filtered.abs() < 1e-20 {
            self.filtered = 0.0;
        }
        self.age += 1;
        self.filtered * env * self.velocity
    }
}

struct Comb {
    data: Vec<f32>,
    cursor: usize,
}
impl Comb {
    fn new(n: usize) -> Self {
        Self {
            data: vec![0.0; n.max(1)],
            cursor: 0,
        }
    }
    fn process(&mut self, input: f32, feedback: f32) -> f32 {
        let old = self.data[self.cursor];
        let value = input + old * feedback;
        self.data[self.cursor] = if value.abs() < 1e-20 { 0.0 } else { value };
        self.cursor += 1;
        if self.cursor == self.data.len() {
            self.cursor = 0;
        }
        old
    }
}
struct Effects {
    delay: Option<Comb>,
    reverb: Option<[Comb; 4]>,
    rack: Rack,
}
impl Effects {
    fn new(c: &Channel, rate: u32, tempo: f64) -> Self {
        let slots: Vec<_> = c.rack.iter().flatten().copied().collect();
        Self {
            rack: Rack::new(&slots, rate, tempo),
            delay: (c.delay > 0.0).then(|| Comb::new((rate as f64 * 60.0 / tempo * 0.75) as usize)),
            reverb: (c.reverb > 0.0).then(|| {
                [
                    Comb::new((rate as f32 * 0.0297) as usize),
                    Comb::new((rate as f32 * 0.0371) as usize),
                    Comb::new((rate as f32 * 0.0411) as usize),
                    Comb::new((rate as f32 * 0.0437) as usize),
                ]
            }),
        }
    }
    fn process(&mut self, input: f32, c: &Channel) -> [f32; 2] {
        let echo = self.delay.as_mut().map_or(0.0, |d| d.process(input, 0.34)) * c.delay * 0.5;
        let mut wet = [0.0; 2];
        if let Some(combs) = &mut self.reverb {
            for (i, comb) in combs.iter_mut().enumerate() {
                wet[i % 2] += comb.process(input, 0.68 - i as f32 * 0.025) * 0.2 * c.reverb;
            }
        }
        self.rack.process([
            (input + echo) * c.pan[0] + wet[0],
            (input + echo) * c.pan[1] + wet[1],
        ])
    }
}

pub struct Renderer {
    plan: RenderPlan,
    voices: [Voice; VOICE_LIMIT],
    effects: Vec<Effects>,
    cursor: usize,
    frame: u64,
    stolen: u64,
}
impl Renderer {
    pub fn new(plan: RenderPlan) -> Self {
        let effects = plan
            .tracks
            .iter()
            .map(|c| Effects::new(c, plan.sample_rate, plan.tempo))
            .collect();
        Self {
            plan,
            voices: [Voice::default(); VOICE_LIMIT],
            effects,
            cursor: 0,
            frame: 0,
            stolen: 0,
        }
    }
    pub fn frame(&self) -> u64 {
        self.frame
    }
    pub fn song_frames(&self) -> u64 {
        self.plan.song_frames
    }
    pub fn total_frames(&self) -> u64 {
        self.plan.total_frames
    }
    pub fn stolen_voices(&self) -> u64 {
        self.stolen
    }
    pub fn rewind(&mut self) {
        self.cursor = 0;
        self.frame = 0;
        self.voices.fill(Voice::default());
    }
    pub fn next_frame(&mut self) -> [f32; 2] {
        while self.cursor < self.plan.events.len()
            && self.plan.events[self.cursor].frame <= self.frame
        {
            let e = self.plan.events[self.cursor];
            self.cursor += 1;
            let index = self
                .voices
                .iter()
                .position(|v| !v.active)
                .unwrap_or_else(|| {
                    self.stolen += 1;
                    self.voices
                        .iter()
                        .enumerate()
                        .max_by_key(|(_, v)| v.age)
                        .map_or(0, |(i, _)| i)
                });
            self.voices[index] = Voice {
                active: true,
                gate: e.gate,
                increment: e.increment,
                pitch: e.pitch,
                velocity: e.velocity,
                track: e.track,
                random: 0x1234567u32.wrapping_add(self.cursor as u32 * 73),
                ..Default::default()
            };
        }
        let mut channels = [0.0f32; 32];
        for voice in &mut self.voices {
            if voice.active {
                let c = &self.plan.tracks[voice.track];
                channels[voice.track] += voice.sample(c, self.plan.sample_rate as f32) * c.gain;
            }
        }
        let mut output = [0.0f32; 2];
        for (i, effects) in self.effects.iter_mut().enumerate() {
            let mixed = effects.process(channels[i], &self.plan.tracks[i]);
            output[0] += mixed[0];
            output[1] += mixed[1];
        }
        self.frame += 1;
        // Short start/end fades prevent transport clicks; bounded soft limiting protects output.
        let fade = (self.frame as f32 / 128.0).min(1.0)
            * ((self.plan.total_frames.saturating_sub(self.frame)) as f32 / 256.0).min(1.0);
        for sample in &mut output {
            *sample = (*sample * self.plan.master).tanh() * 0.95 * fade;
        }
        output
    }
}

pub fn render_wav(project: &Project, path: &Path, sample_rate: u32) -> Result<RenderStats> {
    let mut renderer = Renderer::new(compile(project, sample_rate)?);
    if let Some(p) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(p)?;
    }
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    let mut peak = 0.0f32;
    let frames = renderer.total_frames();
    for _ in 0..frames {
        for sample in renderer.next_frame() {
            peak = peak.max(sample.abs());
            writer.write_sample((sample * 8_388_607.0).round() as i32)?;
        }
    }
    writer.finalize()?;
    Ok(RenderStats {
        frames,
        peak,
        stolen_voices: renderer.stolen,
    })
}
