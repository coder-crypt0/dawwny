use crate::{Renderer, compile};
use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

#[derive(Default)]
struct Telemetry {
    playing: AtomicBool,
    frame: AtomicU64,
    peak: AtomicU32,
    device_error: AtomicBool,
}
pub struct AudioEngine {
    stream: Option<cpal::Stream>,
    state: Arc<Telemetry>,
    rate: u32,
    tempo: f64,
    name: String,
}
impl AudioEngine {
    pub fn new() -> Result<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .context("No default output device")?;
        let config = device.default_output_config()?;
        Ok(Self {
            stream: None,
            state: Arc::default(),
            rate: config.sample_rate().0,
            tempo: 120.0,
            name: device
                .name()
                .unwrap_or_else(|_| "Default audio output".into()),
        })
    }
    pub fn play(&mut self, project: &dawwny_core::Project, looped: bool) -> Result<()> {
        self.stop();
        let device = cpal::default_host()
            .default_output_device()
            .context("No default output device")?;
        let supported = device.default_output_config()?;
        let config = supported.config();
        self.rate = config.sample_rate.0;
        self.tempo = project.tempo;
        self.name = device
            .name()
            .unwrap_or_else(|_| "Default audio output".into());
        let renderer = Renderer::new(compile(project, self.rate)?);
        self.state = Arc::default();
        self.state.playing.store(true, Ordering::Relaxed);
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                build::<f32>(&device, &config, renderer, self.state.clone(), looped)
            }
            cpal::SampleFormat::I16 => {
                build::<i16>(&device, &config, renderer, self.state.clone(), looped)
            }
            cpal::SampleFormat::U16 => {
                build::<u16>(&device, &config, renderer, self.state.clone(), looped)
            }
            other => {
                self.state.playing.store(false, Ordering::Relaxed);
                bail!("Audio format {other:?} is not supported by this build");
            }
        };
        match stream.and_then(|s| {
            s.play()?;
            Ok(s)
        }) {
            Ok(s) => {
                self.stream = Some(s);
                Ok(())
            }
            Err(e) => {
                self.state.playing.store(false, Ordering::Relaxed);
                Err(e)
            }
        }
    }
    pub fn stop(&mut self) {
        self.state.playing.store(false, Ordering::Relaxed);
        self.stream.take();
        self.state.frame.store(0, Ordering::Relaxed);
        self.state.peak.store(0, Ordering::Relaxed);
    }
    pub fn is_playing(&self) -> bool {
        self.state.playing.load(Ordering::Relaxed)
    }
    pub fn position_beats(&self) -> f64 {
        self.state.frame.load(Ordering::Relaxed) as f64 / self.rate as f64 * self.tempo / 60.0
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
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut renderer: Renderer,
    state: Arc<Telemetry>,
    looped: bool,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let failure = state.clone();
    Ok(device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut peak = 0.0f32;
            for frame in output.chunks_mut(channels) {
                let stereo = if state.playing.load(Ordering::Relaxed) {
                    if looped && renderer.frame() >= renderer.song_frames() {
                        renderer.rewind();
                    }
                    if !looped && renderer.frame() >= renderer.total_frames() {
                        state.playing.store(false, Ordering::Relaxed);
                        [0.0; 2]
                    } else {
                        renderer.next_frame()
                    }
                } else {
                    [0.0; 2]
                };
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
            state.frame.store(
                renderer.frame().min(renderer.song_frames()),
                Ordering::Relaxed,
            );
            let previous = f32::from_bits(state.peak.load(Ordering::Relaxed));
            state
                .peak
                .store(peak.max(previous * 0.9).to_bits(), Ordering::Relaxed);
        },
        move |_| {
            failure.device_error.store(true, Ordering::Relaxed);
            failure.playing.store(false, Ordering::Relaxed);
        },
        None,
    )?)
}
