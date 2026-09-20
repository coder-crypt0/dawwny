# Dawn synth and stock effects

Dawn is a native subtractive synthesizer shared by the studio and MCP. Select **Dawn synth** in Sound, or load a sound preset. No model, plugin installation, or sample download is required.

## Design a sound

Two oscillators offer sine, triangle, saw, and pulse waves with independent level, semitone tuning, fine detune, and pulse width. A sine sub oscillator and deterministic noise source add weight and texture. The amplitude envelope shapes attack, decay, sustain, and release. A resonant low-pass, high-pass, or band-pass filter uses the same envelope for cutoff motion. A per-voice sine LFO modulates pitch and cutoff.

Saw and pulse discontinuities use PolyBLEP corrections. Triangle uses a truncated odd-harmonic series below Nyquist. Oscillators above the supported Nyquist margin are suppressed; this avoids folding ultrasonic fundamentals back into the audible range. The filter uses a topology-preserving state-variable design. Modulation coefficients update every eight samples. This is a compact subtractive engine, not a sample-based acoustic instrument collection or a wavetable/FM engine.

## Effects rack

Open **Sound → Effects**. Add up to eight slots, switch them on/off, move them earlier/later, or remove them. Processing follows the visible order. Every slot owns its state, so two echoes or reverbs remain independent.

| Effect | Controls and implementation |
| --- | --- |
| Space | Stereo diffuse reverb; room size, approximate RT60 decay, damping, wet/dry |
| Echo | Tempo delay in quarter-note beats, feedback, damping, ping-pong, wet/dry |
| Drift | Stereo fractional-delay chorus; rate, depth, wet/dry |
| Heat | Soft saturation and low-pass tone filter; drive, tone, wet/dry |
| Tone | Low shelf at 180 Hz, mid bell at 1 kHz, high shelf at 4 kHz, ±18 dB |
| Glue | Stereo-linked peak compressor; threshold, ratio, attack, release, makeup, wet/dry |

Rack bypass and zero wet mix pass audio through exactly. The original reverb/delay controls remain under **Legacy ambience** to preserve old sessions. They precede the rack; set them to zero when using only rack ambience. Track gain precedes the rack and therefore affects compressor input. The master soft limiter follows the complete mix. Saturation is not oversampled in this version.

DSP buffers allocate before playback. The complete session has a 64 MiB delay-buffer budget at the selected sample rate; overly large echo racks fail with an actionable message before allocation. Rack export tails are capped at 30 seconds plus the instrument release and legacy tail. Very long feedback tails therefore fade out at that bound. Live edits still restart playback after autosave.

## Agent workflow

1. Call `read_project` and `list_sounds`.
2. Apply `apply_sound_preset` using a returned preset ID, or use `update_track` with `instrument: "synth"` and a complete `patch`.
3. Modify `patch.synth` and the ordered `patch.effects` array. Preserve fields you want to keep: patch updates replace the complete patch.
4. Use the returned revision for subsequent edits or WAV export. Re-read after a conflict.

The schema supplies units and enum choices. Numeric ranges are validated for both enabled and bypassed effects. Existing schema-1 documents load with the new synth defaults and an empty rack; older builds cannot read newly added fields.

## References and checks

The filter equations follow [Andy Simper's trapezoidal SVF analysis](https://www.cytomic.com/files/dsp/SvfLinearTrapOptimised.pdf). The reverb uses an independently implemented [Schroeder comb/allpass topology](https://www.dsprelated.com/freebooks/pasp/Schroeder_Reverberators.html). EQ coefficients follow [Robert Bristow-Johnson's audio EQ cookbook](https://www.w3.org/TR/audio-eq-cookbook/).

Tests check frequency across sample rates, finite extreme modulation, audible parameter changes, exact echo timing/channel alternation, reverb decay, EQ frequency selectivity, compression, effect order, buffer limits, old sessions, and zero allocation/free calls during the sample loop. These are engineering checks, not a claim of listening approval for every possible patch.
