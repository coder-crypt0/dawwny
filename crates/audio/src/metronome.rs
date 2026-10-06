use dawwny_core::MetronomeSettings;
use std::f32::consts::TAU;

/// A monitor-only cue computed from absolute sample positions, with no beat accumulator.
pub(crate) struct Metronome {
    rate: f32,
    frames_per_beat: f64,
    duration: u64,
    attack: f32,
    settings: MetronomeSettings,
    master_gain: f32,
}

impl Metronome {
    pub(crate) fn new(
        rate: u32,
        tempo: f64,
        settings: MetronomeSettings,
        master_gain: f32,
    ) -> Self {
        Self {
            rate: rate as f32,
            frames_per_beat: rate as f64 * 60.0 / tempo,
            duration: (rate as f64 * 0.025).round() as u64,
            attack: (rate as f32 * 0.0005).max(1.0),
            settings,
            master_gain,
        }
    }

    pub(crate) fn set_tempo(&mut self, tempo: f64) {
        self.frames_per_beat = self.rate as f64 * 60.0 / tempo;
    }

    pub(crate) fn set_mix(&mut self, settings: MetronomeSettings, master_gain: f32) {
        self.settings = settings;
        self.master_gain = master_gain;
    }

    pub(crate) fn sample(&self, frame: u64) -> f32 {
        if !self.settings.enabled || self.settings.gain == 0.0 || self.master_gain == 0.0 {
            return 0.0;
        }
        // Half-frame lookahead matches the renderer's rounding, even at fractional beat periods.
        let beat = ((frame as f64 + 0.5) / self.frames_per_beat).floor() as u64;
        let onset = (beat as f64 * self.frames_per_beat).round() as u64;
        if frame < onset || frame - onset >= self.duration {
            return 0.0;
        }
        let age = (frame - onset) as f32;
        let downbeat = beat.is_multiple_of(4);
        let frequency = if downbeat { 1480.0 } else { 1046.0 };
        let level = if downbeat { 0.4 } else { 0.26 };
        let remaining = 1.0 - age / (self.duration - 1) as f32;
        let envelope = remaining * remaining * remaining * (age / self.attack).min(1.0);
        (TAU * frequency * age / self.rate).sin()
            * envelope
            * level
            * self.settings.gain
            * self.master_gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_tempos_seek_and_long_positions_use_exact_sample_boundaries() {
        for rate in [8000, 16000, 44100, 48000, 192000] {
            for tempo in [30.0, 92.0, 137.3, 300.0] {
                let click = Metronome::new(
                    rate,
                    tempo,
                    MetronomeSettings {
                        enabled: true,
                        gain: 0.5,
                    },
                    0.7,
                );
                for beat in [0, 1, 3, 4, 521, 1023] {
                    let frame = (beat as f64 * rate as f64 * 60.0 / tempo).round() as u64;
                    if frame > 0 {
                        assert_eq!(click.sample(frame - 1), 0.0);
                    }
                    assert_eq!(click.sample(frame), 0.0);
                    assert!(click.sample(frame + 1).abs() > 0.000001);
                    assert_eq!(click.sample(frame + click.duration), 0.0);
                    assert!(click.sample(frame + click.duration / 2).is_finite());
                }
            }
        }
    }

    #[test]
    fn bar_accent_levels_and_tempo_updates_change_the_real_cue() {
        let mut click = Metronome::new(
            48000,
            120.0,
            MetronomeSettings {
                enabled: true,
                gain: 1.0,
            },
            1.0,
        );
        let peak = |click: &Metronome, start: u64| {
            (0..1200)
                .map(|n| click.sample(start + n).abs())
                .fold(0.0, f32::max)
        };
        assert!(peak(&click, 0) > peak(&click, 24000) * 1.4);
        let original = click.sample(10);
        click.set_mix(
            MetronomeSettings {
                enabled: true,
                gain: 0.5,
            },
            0.4,
        );
        assert!((click.sample(10) - original * 0.2).abs() < 1e-6);
        click.set_tempo(60.0);
        assert_eq!(click.sample(24010), 0.0);
        assert!(click.sample(48010).abs() > 0.0);
        click.set_mix(
            MetronomeSettings {
                enabled: false,
                gain: 1.0,
            },
            1.0,
        );
        assert_eq!(click.sample(10), 0.0);
    }
}
