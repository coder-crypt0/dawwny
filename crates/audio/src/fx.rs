//! Ordered native stereo effects. State belongs to each slot; buffers never grow on playback.
use dawwny_core::{Effect, EffectSlot};
use std::f32::consts::{PI, TAU};

fn clean(x: f32) -> f32 {
    if x.abs() < 1e-20 { 0.0 } else { x }
}
fn mix(dry: [f32; 2], wet: [f32; 2], amount: f32) -> [f32; 2] {
    [
        dry[0] + (wet[0] - dry[0]) * amount,
        dry[1] + (wet[1] - dry[1]) * amount,
    ]
}
fn amount(e: Effect) -> f32 {
    match e {
        Effect::Reverb { mix, .. }
        | Effect::Echo { mix, .. }
        | Effect::Chorus { mix, .. }
        | Effect::Drive { mix, .. }
        | Effect::Compressor { mix, .. } => mix,
        Effect::Equalizer {
            low_db,
            mid_db,
            high_db,
        } => {
            if low_db == 0.0 && mid_db == 0.0 && high_db == 0.0 {
                0.0
            } else {
                1.0
            }
        }
    }
}
fn enabled(slots: &[EffectSlot]) -> impl Iterator<Item = Effect> + '_ {
    slots
        .iter()
        .filter(|s| s.enabled && amount(s.effect) > 0.0)
        .map(|s| s.effect)
}
pub(crate) fn tail_seconds(slots: &[EffectSlot], tempo: f64) -> f32 {
    enabled(slots)
        .map(|e| match e {
            Effect::Reverb { decay, .. } => decay * 1.25 + 0.2,
            Effect::Echo {
                beats, feedback, ..
            } => {
                let repeats = if feedback > 0.0 {
                    (0.001_f32.ln() / feedback.ln()).ceil() + 1.0
                } else {
                    1.0
                };
                beats * (60.0 / tempo) as f32 * repeats
            }
            Effect::Chorus { .. } => 0.04,
            _ => 0.0,
        })
        .sum::<f32>()
        .min(30.0)
}
fn comb_lengths(rate: u32, size: f32, channel: usize) -> [usize; 4] {
    [0.0297, 0.0371, 0.0411, 0.0437].map(|s| {
        ((s * (0.55 + size * 1.45) + channel as f32 * 0.0011) * rate as f32)
            .round()
            .max(1.0) as usize
    })
}
fn allpass_lengths(rate: u32, channel: usize) -> [usize; 2] {
    [0.005, 0.0017].map(|s| {
        ((s + channel as f32 * 0.0003) * rate as f32)
            .round()
            .max(1.0) as usize
    })
}
fn echo_length(rate: u32, tempo: f64, beats: f32) -> usize {
    (rate as f64 * 60.0 / tempo * beats as f64).round().max(1.0) as usize
}
pub(crate) fn storage_bytes(slots: &[EffectSlot], rate: u32, tempo: f64) -> usize {
    enabled(slots)
        .map(|e| match e {
            Effect::Echo { beats, .. } => echo_length(rate, tempo, beats) * 8,
            Effect::Chorus { .. } => (rate as f32 * 0.032).ceil() as usize * 8 + 16,
            Effect::Reverb { size, .. } => {
                (0..2)
                    .map(|ch| {
                        comb_lengths(rate, size, ch).iter().sum::<usize>()
                            + allpass_lengths(rate, ch).iter().sum::<usize>()
                    })
                    .sum::<usize>()
                    * 4
            }
            _ => 0,
        })
        .sum()
}
struct Line {
    data: Vec<f32>,
    cursor: usize,
}
impl Line {
    fn new(n: usize) -> Self {
        Self {
            data: vec![0.0; n],
            cursor: 0,
        }
    }
    fn read(&self) -> f32 {
        self.data[self.cursor]
    }
    fn write(&mut self, value: f32) {
        self.data[self.cursor] = clean(value);
        self.cursor = (self.cursor + 1) % self.data.len();
    }
}
struct Comb {
    line: Line,
    feedback: f32,
    low: f32,
    coefficient: f32,
}
impl Comb {
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.line.read();
        self.low = clean(self.low + self.coefficient * (delayed - self.low));
        self.line.write(input + self.low * self.feedback);
        delayed
    }
}
struct Reverb {
    combs: [[Comb; 4]; 2],
    allpasses: [[Line; 2]; 2],
    mix: f32,
}
impl Reverb {
    fn new(rate: u32, size: f32, decay: f32, damping: f32, mix: f32) -> Self {
        let cutoff = (20000.0 * 0.025_f32.powf(damping)).min(rate as f32 * 0.45);
        let coefficient = 1.0 - (-TAU * cutoff / rate as f32).exp();
        Self {
            combs: std::array::from_fn(|ch| {
                comb_lengths(rate, size, ch).map(|n| Comb {
                    line: Line::new(n),
                    feedback: 0.001_f32.powf(n as f32 / rate as f32 / decay),
                    low: 0.0,
                    coefficient,
                })
            }),
            allpasses: std::array::from_fn(|ch| allpass_lengths(rate, ch).map(Line::new)),
            mix,
        }
    }
    fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        let mono = (input[0] + input[1]) * 0.5;
        let mut wet = [0.0; 2];
        for (ch, output) in wet.iter_mut().enumerate() {
            *output = self.combs[ch]
                .iter_mut()
                .map(|c| c.process(mono))
                .sum::<f32>()
                * 0.25;
            for line in &mut self.allpasses[ch] {
                let y = line.read() - *output * 0.5;
                line.write(*output + y * 0.5);
                *output = y;
            }
        }
        mix(input, wet, self.mix)
    }
}
struct Echo {
    data: Vec<[f32; 2]>,
    cursor: usize,
    low: [f32; 2],
    coefficient: f32,
    feedback: f32,
    ping_pong: bool,
    mix: f32,
}
impl Echo {
    fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        let old = self.data[self.cursor];
        for (low, sample) in self.low.iter_mut().zip(old) {
            *low = clean(*low + self.coefficient * (sample - *low));
        }
        self.data[self.cursor] = if self.ping_pong {
            [
                clean((input[0] + input[1]) * 0.5 + self.low[1] * self.feedback),
                clean(self.low[0] * self.feedback),
            ]
        } else {
            [
                clean(input[0] + self.low[0] * self.feedback),
                clean(input[1] + self.low[1] * self.feedback),
            ]
        };
        self.cursor = (self.cursor + 1) % self.data.len();
        mix(input, old, self.mix)
    }
}
struct Chorus {
    data: Vec<[f32; 2]>,
    cursor: usize,
    phase: f32,
    step: f32,
    depth: f32,
    rate: f32,
    mix: f32,
}
impl Chorus {
    fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        self.data[self.cursor] = input;
        let mut wet = [0.0; 2];
        for (ch, y) in wet.iter_mut().enumerate() {
            let delay = self.rate
                * (0.012 + 0.006 * self.depth * (TAU * self.phase + ch as f32 * PI * 0.5).sin());
            let position = (self.cursor as f32 - delay).rem_euclid(self.data.len() as f32);
            let index = position.floor() as usize;
            let fraction = position.fract();
            *y = self.data[index][ch] * (1.0 - fraction)
                + self.data[(index + 1) % self.data.len()][ch] * fraction;
        }
        self.cursor = (self.cursor + 1) % self.data.len();
        self.phase = (self.phase + self.step).fract();
        mix(input, wet, self.mix)
    }
}
struct Drive {
    gain: f32,
    coefficient: f32,
    low: [f32; 2],
    mix: f32,
}
impl Drive {
    fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        for (low, x) in self.low.iter_mut().zip(input) {
            *low = clean(*low + self.coefficient * ((x * self.gain).tanh() - *low));
        }
        mix(input, self.low, self.mix)
    }
}
#[derive(Clone, Copy)]
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    z: [[f32; 2]; 2],
}
impl Biquad {
    // RBJ audio EQ cookbook: low shelf / peaking / high shelf.
    fn new(kind: usize, frequency: f32, db: f32, rate: u32) -> Self {
        let a = 10.0_f32.powf(db / 40.0);
        let w = TAU * frequency.min(rate as f32 * 0.4) / rate as f32;
        let c = w.cos();
        let sn = w.sin();
        let alpha = sn / 2.0 * 2.0_f32.sqrt();
        let beta = 2.0 * a.sqrt() * alpha;
        let (b, den) = match kind {
            0 => (
                [
                    a * ((a + 1.0) - (a - 1.0) * c + beta),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * c),
                    a * ((a + 1.0) - (a - 1.0) * c - beta),
                ],
                [
                    (a + 1.0) + (a - 1.0) * c + beta,
                    -2.0 * ((a - 1.0) + (a + 1.0) * c),
                    (a + 1.0) + (a - 1.0) * c - beta,
                ],
            ),
            2 => (
                [
                    a * ((a + 1.0) + (a - 1.0) * c + beta),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
                    a * ((a + 1.0) + (a - 1.0) * c - beta),
                ],
                [
                    (a + 1.0) - (a - 1.0) * c + beta,
                    2.0 * ((a - 1.0) - (a + 1.0) * c),
                    (a + 1.0) - (a - 1.0) * c - beta,
                ],
            ),
            _ => {
                let alpha = sn / (2.0 * 0.8);
                (
                    [1.0 + alpha * a, -2.0 * c, 1.0 - alpha * a],
                    [1.0 + alpha / a, -2.0 * c, 1.0 - alpha / a],
                )
            }
        };
        Self {
            b: b.map(|x| x / den[0]),
            a: [den[1] / den[0], den[2] / den[0]],
            z: [[0.0; 2]; 2],
        }
    }
    fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        let mut output = [0.0; 2];
        for ch in 0..2 {
            let y = self.b[0] * input[ch] + self.z[ch][0];
            self.z[ch] = [
                clean(self.b[1] * input[ch] - self.a[0] * y + self.z[ch][1]),
                clean(self.b[2] * input[ch] - self.a[1] * y),
            ];
            output[ch] = y;
        }
        output
    }
}
struct Compressor {
    threshold: f32,
    slope: f32,
    attack: f32,
    release: f32,
    gain: f32,
    makeup: f32,
    mix: f32,
}
impl Compressor {
    fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        let level = input[0].abs().max(input[1].abs()).max(1e-12);
        let over = (20.0 * level.log10() - self.threshold).max(0.0);
        let target = 10.0_f32.powf(-over * self.slope / 20.0);
        let coefficient = if target < self.gain {
            self.attack
        } else {
            self.release
        };
        self.gain = clean(target + coefficient * (self.gain - target));
        mix(input, input.map(|x| x * self.gain * self.makeup), self.mix)
    }
}
enum Node {
    Reverb(Box<Reverb>),
    Echo(Echo),
    Chorus(Chorus),
    Drive(Drive),
    Eq([Biquad; 3]),
    Compressor(Compressor),
}
pub(crate) struct Rack {
    nodes: Vec<Node>,
}
impl Rack {
    pub(crate) fn new(slots: &[EffectSlot], rate: u32, tempo: f64) -> Self {
        Self {
            nodes: enabled(slots)
                .map(|e| match e {
                    Effect::Reverb {
                        size,
                        decay,
                        damping,
                        mix,
                    } => Node::Reverb(Box::new(Reverb::new(rate, size, decay, damping, mix))),
                    Effect::Echo {
                        beats,
                        feedback,
                        damping,
                        ping_pong,
                        mix,
                    } => Node::Echo(Echo {
                        data: vec![[0.0; 2]; echo_length(rate, tempo, beats)],
                        cursor: 0,
                        low: [0.0; 2],
                        coefficient: 1.0 - damping * 0.95,
                        feedback,
                        ping_pong,
                        mix,
                    }),
                    Effect::Chorus {
                        rate: hz,
                        depth,
                        mix,
                    } => Node::Chorus(Chorus {
                        data: vec![[0.0; 2]; (rate as f32 * 0.032).ceil() as usize + 2],
                        cursor: 0,
                        phase: 0.0,
                        step: hz / rate as f32,
                        depth,
                        rate: rate as f32,
                        mix,
                    }),
                    Effect::Drive { drive, tone, mix } => Node::Drive(Drive {
                        gain: drive,
                        coefficient: 1.0
                            - (-TAU * tone.min(rate as f32 * 0.45) / rate as f32).exp(),
                        low: [0.0; 2],
                        mix,
                    }),
                    Effect::Equalizer {
                        low_db,
                        mid_db,
                        high_db,
                    } => Node::Eq([
                        Biquad::new(0, 180.0, low_db, rate),
                        Biquad::new(1, 1000.0, mid_db, rate),
                        Biquad::new(2, 4000.0, high_db, rate),
                    ]),
                    Effect::Compressor {
                        threshold_db,
                        ratio,
                        attack_ms,
                        release_ms,
                        makeup_db,
                        mix,
                    } => Node::Compressor(Compressor {
                        threshold: threshold_db,
                        slope: 1.0 - 1.0 / ratio,
                        attack: (-1.0 / (rate as f32 * attack_ms * 0.001)).exp(),
                        release: (-1.0 / (rate as f32 * release_ms * 0.001)).exp(),
                        gain: 1.0,
                        makeup: 10.0_f32.powf(makeup_db / 20.0),
                        mix,
                    }),
                })
                .collect(),
        }
    }
    pub(crate) fn process(&mut self, mut input: [f32; 2]) -> [f32; 2] {
        for node in &mut self.nodes {
            input = match node {
                Node::Reverb(n) => n.process(input),
                Node::Echo(n) => n.process(input),
                Node::Chorus(n) => n.process(input),
                Node::Drive(n) => n.process(input),
                Node::Compressor(n) => n.process(input),
                Node::Eq(filters) => {
                    for filter in filters {
                        input = filter.process(input);
                    }
                    input
                }
            };
        }
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn slot(effect: Effect) -> EffectSlot {
        EffectSlot {
            enabled: true,
            effect,
        }
    }
    #[test]
    fn echo_has_exact_timing_and_alternates_channels() {
        let mut rack = Rack::new(
            &[slot(Effect::Echo {
                beats: 0.5,
                feedback: 0.5,
                damping: 0.0,
                ping_pong: true,
                mix: 1.0,
            })],
            48000,
            120.0,
        );
        for i in 0..36001 {
            let y = rack.process(if i == 0 { [1.0; 2] } else { [0.0; 2] });
            let expected = match i {
                12000 => [1.0, 0.0],
                24000 => [0.0, 0.5],
                36000 => [0.25, 0.0],
                _ => [0.0; 2],
            };
            assert_eq!(y, expected, "sample {i}");
        }
    }
    #[test]
    fn bypass_is_exact_and_empty_rack_uses_no_delay_memory() {
        let slots = [
            EffectSlot {
                enabled: false,
                effect: Effect::Reverb {
                    size: 1.0,
                    decay: 8.0,
                    damping: 0.0,
                    mix: 1.0,
                },
            },
            slot(Effect::Echo {
                beats: 4.0,
                feedback: 0.85,
                damping: 0.0,
                ping_pong: false,
                mix: 0.0,
            }),
        ];
        assert_eq!(storage_bytes(&slots, 192000, 30.0), 0);
        let mut rack = Rack::new(&slots, 48000, 120.0);
        assert_eq!(rack.process([0.123, -0.7]), [0.123, -0.7]);
    }
    #[test]
    fn room_produces_a_decaying_stereo_tail() {
        let mut r = Rack::new(
            &[slot(Effect::Reverb {
                size: 0.6,
                decay: 1.0,
                damping: 0.4,
                mix: 1.0,
            })],
            16000,
            120.0,
        );
        let mut early = 0.0;
        let mut late = 0.0;
        let mut width = 0.0;
        for i in 0..48000 {
            let y = r.process(if i == 0 { [1.0; 2] } else { [0.0; 2] });
            assert!(y.into_iter().all(f32::is_finite));
            if i < 16000 {
                early += y[0] * y[0];
                width += (y[0] - y[1]).abs();
            } else if i >= 32000 {
                late += y[0] * y[0];
            }
        }
        assert!(early > 0.001 && width > 0.01 && late < early * 0.001);
    }
    fn rms(effect: Effect, frequency: f32) -> f32 {
        let mut r = Rack::new(&[slot(effect)], 48000, 120.0);
        let mut energy = 0.0;
        for i in 0..48000 {
            let x = (TAU * frequency * i as f32 / 48000.0).sin() * 0.1;
            let y = r.process([x; 2]);
            if i >= 24000 {
                energy += y[0] * y[0];
            }
        }
        (energy / 24000.0).sqrt()
    }
    #[test]
    fn eq_changes_frequency_balance_not_only_overall_gain() {
        let effect = Effect::Equalizer {
            low_db: 12.0,
            mid_db: 0.0,
            high_db: -12.0,
        };
        assert!(rms(effect, 60.0) > rms(effect, 8000.0) * 8.0);
        let mid = Effect::Equalizer {
            low_db: 0.0,
            mid_db: 12.0,
            high_db: 0.0,
        };
        assert!(rms(mid, 1000.0) > rms(mid, 60.0) * 3.0);
    }
    #[test]
    fn compressor_links_channels_and_reduces_sustained_levels() {
        let mut r = Rack::new(
            &[slot(Effect::Compressor {
                threshold_db: -20.0,
                ratio: 4.0,
                attack_ms: 2.0,
                release_ms: 100.0,
                makeup_db: 0.0,
                mix: 1.0,
            })],
            48000,
            120.0,
        );
        let mut y = [0.0; 2];
        for _ in 0..48000 {
            y = r.process([0.8, 0.4]);
        }
        assert!(y[0] < 0.2 && y[0] > 0.1);
        assert!((y[0] / y[1] - 2.0).abs() < 1e-6);
    }
    #[test]
    fn chorus_and_drive_are_audible_and_slot_order_matters() {
        let drive = slot(Effect::Drive {
            drive: 8.0,
            tone: 3000.0,
            mix: 1.0,
        });
        let chorus = slot(Effect::Chorus {
            rate: 0.5,
            depth: 0.8,
            mix: 0.5,
        });
        let mut a = Rack::new(&[drive, chorus], 48000, 120.0);
        let mut b = Rack::new(&[chorus, drive], 48000, 120.0);
        let mut difference = 0.0;
        let mut wet = 0.0;
        for i in 0..48000 {
            let x = (TAU * 220.0 * i as f32 / 48000.0).sin() * 0.2;
            let y = a.process([x; 2]);
            let z = b.process([x; 2]);
            difference += (y[0] - z[0]).abs();
            wet += (y[0] - x).abs();
        }
        assert!(difference > 1.0 && wet > 1.0);
    }
}
