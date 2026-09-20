//! Dawn: two band-limited oscillators, sub/noise, and a modulated TPT filter.
use dawwny_core::{CustomSynth, FilterMode, Oscillator, Waveform};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug)]
struct Osc {
    waveform: Waveform,
    level: f32,
    ratio: f32,
    width: f32,
}
impl From<Oscillator> for Osc {
    fn from(o: Oscillator) -> Self {
        Self {
            waveform: o.waveform,
            level: o.level,
            ratio: 2.0_f32.powf((o.semitones as f32 + o.detune_cents / 100.0) / 12.0),
            width: o.pulse_width,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct CompiledSynth {
    oscillators: [Osc; 2],
    settings: CustomSynth,
    cutoff: f32,
    rate: f32,
}
impl CompiledSynth {
    pub(crate) fn new(s: &CustomSynth, cutoff: f32, rate: u32) -> Self {
        Self {
            oscillators: [s.osc1.into(), s.osc2.into()],
            settings: *s,
            cutoff,
            rate: rate as f32,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct SynthVoice {
    phases: [f32; 2],
    sub_phase: f32,
    lfo_phase: f32,
    random: u32,
    integrator: [f32; 2],
    control_tick: u8,
    g: f32,
    pitch_ratio: f32,
}
impl Default for SynthVoice {
    fn default() -> Self {
        Self {
            phases: [0.0, 0.17],
            sub_phase: 0.0,
            lfo_phase: 0.0,
            random: 0x6d2b79f5,
            integrator: [0.0; 2],
            control_tick: 0,
            g: 0.0,
            pitch_ratio: 1.0,
        }
    }
}
// Polynomial correction at discontinuities; phases are cycles, not radians.
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}
fn oscillator(wave: Waveform, phase: f32, dt: f32, width: f32) -> f32 {
    if dt >= 0.49 || dt <= 0.0 {
        return 0.0;
    }
    match wave {
        Waveform::Sine => (TAU * phase).sin(),
        Waveform::Saw => 2.0 * phase - 1.0 - poly_blep(phase, dt),
        Waveform::Pulse => {
            let width = width.clamp(dt, 1.0 - dt);
            (if phase < width { 1.0 } else { -1.0 }) + poly_blep(phase, dt)
                - poly_blep((phase - width).rem_euclid(1.0), dt)
        }
        Waveform::Triangle => {
            // Truncated odd-harmonic series: no harmonics above Nyquist.
            let mut signal = 0.0;
            for n in (1..=15).step_by(2) {
                let nf = n as f32;
                if nf * dt >= 0.49 {
                    break;
                }
                let sign = if n % 4 == 1 { 1.0 } else { -1.0 };
                signal += sign * (TAU * phase * nf).sin() / (nf * nf);
            }
            signal * 8.0 / (PI * PI)
        }
    }
}
fn clean(v: f32) -> f32 {
    if v.abs() < 1e-20 { 0.0 } else { v }
}
impl SynthVoice {
    pub(crate) fn sample(&mut self, c: &CompiledSynth, frequency: f32, envelope: f32) -> f32 {
        let s = &c.settings;
        let lfo = (TAU * self.lfo_phase).sin();
        self.lfo_phase = (self.lfo_phase + s.lfo_rate / c.rate).fract();
        // Filter and pitch control at 1/8 sample rate keeps transcendental work bounded.
        if self.control_tick == 0 {
            self.pitch_ratio = if s.lfo_pitch == 0.0 {
                1.0
            } else {
                2.0_f32.powf(lfo * s.lfo_pitch / 12.0)
            };
            let cutoff = (c.cutoff * 2.0_f32.powf(s.filter_env * envelope + s.lfo_filter * lfo))
                .clamp(20.0, c.rate * 0.45);
            self.g = (PI * cutoff / c.rate).tan();
        }
        self.control_tick = (self.control_tick + 1) & 7;
        let dt = frequency * self.pitch_ratio / c.rate;
        let mut input = 0.0;
        for (phase, osc) in self.phases.iter_mut().zip(c.oscillators) {
            let step = dt * osc.ratio;
            if osc.level > 0.0 {
                input += oscillator(osc.waveform, *phase, step, osc.width) * osc.level;
            }
            *phase = (*phase + step).fract();
        }
        if dt * 0.5 < 0.49 {
            input += (TAU * self.sub_phase).sin() * s.sub_level;
        }
        self.sub_phase = (self.sub_phase + dt * 0.5).fract();
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        input += (self.random as f32 / u32::MAX as f32 * 2.0 - 1.0) * s.noise_level;
        input *= 0.35;
        // Topology-preserving state-variable filter (Simper): stable up to Nyquist.
        let k = 2.0 - s.resonance * 1.9;
        let a1 = 1.0 / (1.0 + self.g * (self.g + k));
        let a2 = self.g * a1;
        let a3 = self.g * a2;
        let v3 = input - self.integrator[1];
        let band = a1 * self.integrator[0] + a2 * v3;
        let low = self.integrator[1] + a2 * self.integrator[0] + a3 * v3;
        self.integrator = [
            clean(2.0 * band - self.integrator[0]),
            clean(2.0 * low - self.integrator[1]),
        ];
        match s.filter_mode {
            FilterMode::LowPass => low,
            FilterMode::BandPass => band * k,
            FilterMode::HighPass => input - k * band - low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tuning_tracks_sample_rate_and_semitone_offsets() {
        for rate in [8_000, 48_000, 192_000] {
            for semitones in [0, 12] {
                let mut s = CustomSynth::default();
                s.osc1.waveform = Waveform::Sine;
                s.osc1.semitones = semitones;
                s.osc2.level = 0.0;
                s.sub_level = 0.0;
                s.filter_env = 0.0;
                let c = CompiledSynth::new(&s, 3000.0, rate);
                let mut v = SynthVoice::default();
                let mut last = 0.0;
                let mut crossings: i32 = 0;
                for i in 0..rate {
                    let x = v.sample(&c, 220.0, 1.0);
                    if i > rate / 4 && last <= 0.0 && x > 0.0 {
                        crossings += 1;
                    }
                    last = x;
                }
                let expected = if semitones == 0 { 165 } else { 330 };
                assert!(
                    (crossings - expected).abs() <= 1,
                    "rate={rate} crossings={crossings}"
                );
            }
        }
    }
    #[test]
    fn noise_is_alive_and_extreme_modulation_stays_finite() {
        let mut s = CustomSynth::default();
        s.osc1.level = 0.0;
        s.osc2.level = 0.0;
        s.sub_level = 0.0;
        s.noise_level = 1.0;
        let mut v = SynthVoice::default();
        let c = CompiledSynth::new(&s, 8000.0, 48000);
        assert!(
            (0..512)
                .map(|_| v.sample(&c, 440.0, 1.0).abs())
                .sum::<f32>()
                > 1.0
        );
        for rate in [8000, 48000, 192000] {
            s.resonance = 0.9;
            s.lfo_filter = 3.0;
            s.filter_env = 4.0;
            s.lfo_pitch = 2.0;
            let c = CompiledSynth::new(&s, 20000.0, rate);
            let mut v = SynthVoice::default();
            for _ in 0..rate {
                let x = v.sample(&c, 12000.0, 1.0);
                assert!(x.is_finite() && x.abs() < 8.0);
            }
        }
    }
    #[test]
    fn saw_discontinuity_is_smoothed() {
        let dt = 440.0 / 48000.0;
        assert_eq!(oscillator(Waveform::Saw, 0.0, dt, 0.5), 0.0);
        let a = oscillator(Waveform::Saw, 1.0 - dt * 0.1, dt, 0.5);
        let b = oscillator(Waveform::Saw, dt * 0.1, dt, 0.5);
        assert!((a - b).abs() < 0.5);
    }
}
