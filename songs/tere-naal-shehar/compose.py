#!/usr/bin/env python3
"""Compose an original 60-bar Dawwny arrangement at 93 BPM.

Run from the repository root with:
    python songs/tere-naal-shehar/compose.py

The generated project is deliberately data-only. Dawwny's validator, MIDI
exporter, and renderer are used by the companion Rust example.
"""

from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parent
PROJECT_PATH = ROOT / "tere-naal-shehar.dawwny.json"
TEMPO = 93.0
BAR = 4.0
LENGTH_BARS = 60

SECTIONS = [
    ("Intro", 0, 4),
    ("Verse 1", 4, 8),
    ("Prechorus 1", 12, 4),
    ("Chorus 1", 16, 8),
    ("Interlude", 24, 8),
    ("Verse 2", 32, 8),
    ("Prechorus 2", 40, 4),
    ("Chorus 2", 44, 8),
    ("Outro lift", 52, 4),
    ("Final chorus", 56, 4),
]

COLORS = {
    "keys": [245, 186, 113],
    "pad": [128, 164, 229],
    "guitar": [232, 129, 146],
    "bass": [141, 112, 225],
    "vocal": [244, 215, 123],
    "hook": [109, 211, 199],
    "pulse": [171, 193, 247],
    "kick": [226, 98, 106],
    "clap": [243, 150, 103],
    "hats": [172, 178, 190],
    "perc": [188, 130, 203],
    "riser": [127, 199, 230],
}


def osc(wave: str, level: float, semitones: int = 0, detune: float = 0.0) -> dict:
    return {
        "waveform": wave,
        "level": level,
        "semitones": semitones,
        "detune_cents": detune,
        "pulse_width": 0.5,
    }


def patch(
    attack: float = 0.008,
    decay: float = 0.25,
    sustain: float = 0.55,
    release: float = 0.35,
    cutoff: float = 8000.0,
    reverb: float = 0.0,
    delay: float = 0.0,
    *,
    wave1: str = "saw",
    wave2: str = "saw",
    level1: float = 0.6,
    level2: float = 0.28,
    tune2: int = 0,
    detune2: float = 7.0,
    sub: float = 0.0,
    noise: float = 0.0,
    filter_mode: str = "low_pass",
    resonance: float = 0.15,
    filter_env: float = 0.5,
    lfo_rate: float = 0.4,
    lfo_pitch: float = 0.0,
    lfo_filter: float = 0.0,
    effects: list[dict] | None = None,
) -> dict:
    return {
        "attack": attack,
        "decay": decay,
        "sustain": sustain,
        "release": release,
        "cutoff": cutoff,
        "reverb": reverb,
        "delay": delay,
        "synth": {
            "osc1": osc(wave1, level1),
            "osc2": osc(wave2, level2, tune2, detune2),
            "sub_level": sub,
            "noise_level": noise,
            "filter_mode": filter_mode,
            "resonance": resonance,
            "filter_env": filter_env,
            "lfo_rate": lfo_rate,
            "lfo_pitch": lfo_pitch,
            "lfo_filter": lfo_filter,
        },
        "effects": [{"enabled": True, "effect": effect} for effect in (effects or [])],
    }


def room(size: float, decay: float, mix: float) -> dict:
    return {"type": "reverb", "size": size, "decay": decay, "damping": 0.55, "mix": mix}


def echo(beats: float, mix: float) -> dict:
    return {
        "type": "echo", "beats": beats, "feedback": 0.24,
        "damping": 0.5, "ping_pong": True, "mix": mix,
    }


TRACKS = [
    ("keys", "01 Chord flow - warm keys", "keys", 0.49, -0.14,
     patch(0.008, 0.7, 0.46, 0.35, 5800, effects=[room(0.24, 1.1, 0.12)])),
    ("pad", "02 Common-tone air pad", "synth", 0.24, -0.32,
     patch(0.37, 1.05, 0.62, 1.6, 2250, wave1="triangle", wave2="saw",
           level1=0.48, level2=0.2, tune2=12, detune2=8,
           lfo_rate=0.23, lfo_filter=0.13, effects=[
               {"type": "chorus", "rate": 0.27, "depth": 0.45, "mix": 0.21},
               room(0.68, 2.8, 0.16)])),
    ("guitar", "03 Guitar-style riff - FLEX clean pick", "synth", 0.47, 0.28,
     patch(0.003, 0.22, 0.08, 0.22, 4900, wave1="triangle", wave2="pulse",
           level1=0.62, level2=0.2, tune2=12, detune2=3,
           filter_env=1.45, effects=[echo(0.75, 0.15), room(0.28, 1.05, 0.1)])),
    ("bass", "04 Round sub bass", "synth", 0.55, 0.0,
     patch(0.006, 0.22, 0.55, 0.13, 480, wave1="triangle", wave2="sine",
           level1=0.61, level2=0.2, sub=0.37, filter_env=1.05,
           effects=[{"type": "drive", "drive": 1.75, "tone": 3200, "mix": 0.1}])),
    ("vocal", "05 VOCAL GUIDE - mute for singer", "lead", 0.36, 0.0,
     patch(0.022, 0.24, 0.68, 0.17, 4300, reverb=0.045)),
    ("hook", "06 Instrument hook - bell lead", "synth", 0.35, -0.25,
     patch(0.01, 0.3, 0.2, 0.38, 5700, wave1="sine", wave2="triangle",
           level1=0.57, level2=0.23, tune2=12, detune2=0,
           filter_env=0.35, effects=[echo(0.75, 0.17), room(0.34, 1.6, 0.11)])),
    ("pulse", "07 Chorus chord pulses", "keys", 0.21, 0.31,
     patch(0.004, 0.16, 0.23, 0.17, 5000)),
    ("kick", "08 Kick", "drums", 0.72, 0.0, patch(0.002, 0.08, 0.0, 0.11, 12000)),
    ("clap", "09 Snare and clap", "drums", 0.47, -0.06, patch(0.002, 0.1, 0.0, 0.11, 12000)),
    ("hats", "10 Hats", "drums", 0.31, 0.18, patch(0.002, 0.05, 0.0, 0.07, 12500)),
    ("perc", "11 Electronic perc and fills", "drums", 0.27, -0.27,
     patch(0.002, 0.09, 0.0, 0.1, 10500)),
    ("riser", "12 Transition air", "synth", 0.18, 0.45,
     patch(0.21, 0.45, 0.35, 0.42, 5500, wave1="sine", wave2="triangle",
           level1=0.0, level2=0.0, noise=0.66, filter_mode="high_pass",
           filter_env=1.2, effects=[room(0.5, 1.8, 0.17)])),
]

NOTES: dict[str, list[tuple[float, int, float, float]]] = defaultdict(list)


def add(role: str, beat: float, pitch: int, duration: float, velocity: float) -> None:
    assert 0 <= beat < LENGTH_BARS * BAR
    assert 0 < duration and beat + duration <= LENGTH_BARS * BAR + 1e-8
    assert 0 <= pitch <= 127 and 0 < velocity <= 1
    NOTES[role].append((round(beat, 5), pitch, round(duration, 5), round(velocity, 4)))


def at(role: str, bar: int, offset: float, pitch: int, duration: float, velocity: float) -> None:
    add(role, bar * BAR + offset, pitch, duration, velocity)


# Low, compact keyboard voicings leave the vocal's upper-mid register clear.
# Common tones persist through the short passing chords.
CHORDS = {
    "Emaj9": ([56, 59, 63, 66], 40),
    "Eadd9": ([56, 59, 64, 66], 40),
    "B/D#": ([54, 59, 63, 66], 39),
    "C#m9": ([56, 59, 63, 64], 37),
    "C#m7": ([56, 59, 61, 64], 37),
    "G#m7/B": ([54, 56, 59, 63], 35),
    "Amaj9": ([56, 59, 61, 64], 33),
    "Aadd9": ([57, 59, 61, 64], 33),
    "E/G#": ([56, 59, 64], 32),
    "F#m11": ([57, 59, 61, 64], 30),
    "F#m7": ([57, 61, 64, 66], 30),
    "B7sus4": ([57, 59, 64, 66], 35),
    "A/B": ([57, 59, 61, 64], 35),
    "B7": ([57, 59, 63, 66], 35),
}

VERSE = [
    [(0, 2.5, "Emaj9"), (2.5, 1.5, "B/D#")],
    [(0, 2.5, "C#m9"), (2.5, 1.5, "G#m7/B")],
    [(0, 2.5, "Amaj9"), (2.5, 1.5, "E/G#")],
    [(0, 2, "F#m11"), (2, 1, "B7sus4"), (3, 1, "B7")],
    [(0, 2, "Emaj9"), (2, 2, "B/D#")],
    [(0, 2, "C#m9"), (2, 1, "B/D#"), (3, 1, "E/G#")],
    [(0, 2.5, "Amaj9"), (2.5, 1.5, "F#m11")],
    [(0, 2, "B7sus4"), (2, 2, "B7")],
]
PRE = [
    [(0, 2, "C#m7"), (2, 2, "Aadd9")],
    [(0, 2, "F#m7"), (2, 1, "B7sus4"), (3, 1, "B7")],
    [(0, 2, "C#m7"), (2, 2, "Aadd9")],
    [(0, 1.5, "F#m7"), (1.5, 1.5, "A/B"), (3, 1, "B7")],
]
CHORUS = [
    [(0, 2, "Eadd9"), (2, 2, "B/D#")],
    [(0, 2, "C#m7"), (2, 2, "Aadd9")],
    [(0, 2, "E/G#"), (2, 1, "F#m7"), (3, 1, "B7sus4")],
    [(0, 2, "Amaj9"), (2, 1, "B7sus4"), (3, 1, "B7")],
]


def harmony_for_bar(bar: int) -> list[tuple[float, float, str]]:
    if bar == 59:
        # Final dominant-to-tonic cadence inside the requested last bar.
        return [(0, 2, "Amaj9"), (2, .5, "B7sus4"), (2.5, .5, "B7"), (3, 1, "Eadd9")]
    if bar < 4:
        return VERSE[bar]
    if bar < 12:
        return VERSE[(bar - 4) % 8]
    if bar < 16:
        return PRE[bar - 12]
    if bar < 24:
        return CHORUS[(bar - 16) % 4]
    if bar < 28:
        return CHORUS[bar - 24]
    if bar < 32:
        return VERSE[bar - 28]
    if bar < 40:
        return VERSE[bar - 32]
    if bar < 44:
        return PRE[bar - 40]
    if bar < 52:
        return CHORUS[(bar - 44) % 4]
    if bar < 56:
        return PRE[bar - 52]
    return CHORUS[bar - 56]


PAD_TONES = {
    "Emaj9": [59, 66], "Eadd9": [59, 66],
    "C#m9": [59, 63], "C#m7": [61, 64],
    "Amaj9": [61, 64], "Aadd9": [61, 64],
    "F#m11": [57, 61], "F#m7": [57, 61],
    "B7sus4": [57, 61], "E/G#": [59, 64],
}


def compose_harmony() -> None:
    for bar in range(LENGTH_BARS):
        events = harmony_for_bar(bar)
        assert abs(sum(d for _, d, _ in events) - BAR) < 1e-8
        chorus_bar = 16 <= bar < 24 or 44 <= bar < 52 or 56 <= bar < 60
        key_level = 0.64 if chorus_bar else (0.51 if bar < 4 else 0.55)
        for beat, duration, name in events:
            tones, root = CHORDS[name]
            # Tiny strum offsets and overlap make the progression breathe.
            for i, pitch in enumerate(tones):
                st = bar * BAR + beat + i * 0.012
                dur = min(duration + 0.08 - i * 0.012, LENGTH_BARS * BAR - st)
                add("keys", st, pitch, dur, key_level - i * 0.026)
            if bar >= 2:
                base_vel = 0.78 if chorus_bar else 0.58
                if 24 <= bar < 32:
                    base_vel = 0.56
                # Each new harmony owns a bass attack; short pickups lead to the next.
                add("bass", bar * BAR + beat, root, min(duration * 0.72, 1.35), base_vel)
                if duration >= 1.5:
                    # Slash chords need a chord tone, not the fifth of their
                    # bass note (D# -> A# would leave the E-major palette).
                    color_tone = {"B/D#": 42, "E/G#": 40}.get(name, root + 7)
                    add("bass", bar * BAR + beat + duration - 0.53,
                        color_tone, 0.34, base_vel * 0.67)
        first = events[0][2]
        if bar >= 1 and not (28 <= bar < 30):
            pad_tones = PAD_TONES.get(first, CHORDS[first][0][:2])
            pad_vel = (
                0.46 if chorus_bar else
                0.35 if 12 <= bar < 16 or 40 <= bar < 44 or 52 <= bar < 56 else
                0.24 if 4 <= bar < 12 or 32 <= bar < 40 else
                0.31
            )
            for i, pitch in enumerate(pad_tones):
                end = 2.97 if bar == 59 else 3.78
                at("pad", bar, i * 0.03, pitch, end - i * 0.03, pad_vel)
            if bar == 59:
                for pitch in (59, 64):
                    at("pad", bar, 3.0, pitch, .96, .39)

        # Midrange offbeat chord fragments add movement only in bigger sections.
        if (12 <= bar < 28) or (36 <= bar < 60):
            if not (28 <= bar < 32):
                for off, name in [(1.5, events[0][2]), (3.5, events[-1][2])]:
                    tones = CHORDS[name][0]
                    for i, pitch in enumerate(tones[:2]):
                        at("pulse", bar, off + i * 0.009, pitch + 12, 0.24, 0.37 - i * 0.025)


# A two-bar picked identity. It answers the lead in verses and takes the front
# in the instrumental interlude. Notes remain in the E-major pitch collection.
GUITAR_A = [(.0, 71, .29), (.75, 68, .34), (1.5, 66, .25), (2.25, 68, .33), (3.5, 71, .29)]
GUITAR_B = [(.0, 73, .29), (.75, 71, .3), (1.5, 68, .29), (2.75, 66, .3), (3.5, 68, .28)]
GUITAR_C = [(.0, 73, .29), (.75, 76, .33), (1.5, 73, .25), (2.75, 68, .3), (3.5, 71, .28)]
GUITAR_D = [(.0, 71, .28), (.75, 69, .29), (1.5, 66, .26), (2.75, 69, .3), (3.5, 71, .28)]


def compose_guitar() -> None:
    # Intro establishes the riff; one new element arrives in each of four bars.
    for bar in range(4):
        pattern = [GUITAR_A, GUITAR_B, GUITAR_C, GUITAR_D][bar]
        for off, pitch, dur in pattern:
            if bar == 0 and off > 2.3:
                continue
            at("guitar", bar, off, pitch, dur, 0.56 if bar < 2 else 0.62)

    # Verses keep the same contour but thin it where the vocal is active.
    for start in (4, 32):
        for n in range(8):
            bar = start + n
            pattern = [GUITAR_A, GUITAR_B, GUITAR_C, GUITAR_D][n % 4]
            for off, pitch, dur in pattern:
                if off in (0.0, 1.5):
                    continue
                if start == 32 and n >= 4 and off == 3.5:
                    pitch += 12 if pitch < 70 else 0
                at("guitar", bar, off, pitch, dur, 0.53 if start == 4 else 0.57)

    # Prechorus removes the riff until the final pickup into each chorus.
    for bar in (15, 43, 55):
        for off, pitch in [(2.75, 73), (3.25, 75), (3.75, 76)]:
            at("guitar", bar, off, pitch, 0.19, 0.5)

    # Choruses use two picked answers per bar, out of the vocal's way.
    answer_pitches = [76, 73, 78, 76]
    for first in (16, 20, 44, 48, 56):
        for n in range(4):
            bar = first + n
            at("guitar", bar, 1.98, answer_pitches[n], 0.27, 0.48)

    # Eight-bar bridge: first half sings the riff openly, second half strips
    # back to the intro guitar before verse 2 enters.
    for n in range(8):
        bar = 24 + n
        pattern = [GUITAR_A, GUITAR_B, GUITAR_C, GUITAR_D][n % 4]
        for off, pitch, dur in pattern:
            if n >= 4 and off in (1.5, 3.5):
                continue
            at("guitar", bar, off, pitch, dur, 0.62 if n < 4 else 0.54)


VERSE_VOCAL = [
    [(.5, 71, .4), (1.15, 68, .65), (2.5, 66, .43), (3.22, 68, .5)],
    [(.25, 68, .47), (1.1, 64, .43), (1.75, 66, .45), (2.62, 71, .72)],
    [(.5, 73, .42), (1.15, 71, .67), (2.5, 68, .72)],
    [(.25, 69, .45), (1.0, 68, .46), (1.75, 66, .62), (3.12, 71, .53)],
    [(.5, 71, .39), (1.15, 68, .58), (2.4, 66, .39), (3.08, 68, .64)],
    [(.25, 73, .44), (1.02, 71, .43), (1.75, 68, .46), (2.65, 66, .73)],
    [(.5, 73, .4), (1.15, 71, .64), (2.5, 68, .42), (3.18, 69, .43)],
    [(.25, 69, .44), (1.0, 68, .46), (1.75, 66, .6), (3.1, 71, .56)],
]

# The prechorus and chorus share an exact five-attack rhythmic cell. The
# prechorus melody climbs within that cell; the chorus opens at its peak.
PRE_VOCAL = [
    [68, 71, 73, 71, 69],
    [69, 73, 75, 73, 71],
    [71, 73, 76, 73, 71],
    [69, 71, 73, 76, 75],  # A/B suspension resolves to B7's D#
]
CHORUS_VOCAL = [
    [71, 73, 76, 75, 71],
    [68, 71, 73, 71, 69],
    [71, 73, 76, 78, 76],
    [73, 71, 69, 71, 76],
]
VOCAL_RHYTHM = [(0.0, .42), (.55, .42), (1.3, .62), (2.55, .36), (3.05, .47)]


def compose_vocal() -> None:
    for first in (4, 32):
        for n, phrase in enumerate(VERSE_VOCAL):
            bar = first + n
            for off, pitch, duration in phrase:
                if first == 32 and n in (2, 6) and pitch == 73:
                    pitch = 76  # verse 2 is a little more open emotionally
                at("vocal", bar, off, pitch, duration, 0.59 if n < 4 else 0.63)

    for first in (12, 40, 52):
        for n, phrase in enumerate(PRE_VOCAL):
            bar = first + n
            for (off, duration), pitch in zip(VOCAL_RHYTHM, phrase):
                at("vocal", bar, off, pitch, duration, 0.64 + n * 0.023)

    for first in (16, 20, 44, 48, 56):
        for n, phrase in enumerate(CHORUS_VOCAL):
            bar = first + n
            for (off, duration), pitch in zip(VOCAL_RHYTHM, phrase):
                at("vocal", bar, off, pitch, duration, 0.7 if first < 44 else 0.73)


def compose_hook() -> None:
    # Instrumental hook intentionally has a different contour from the vocal.
    four_bar = [
        [(.0, 83, .53), (.75, 80, .32), (1.48, 78, .34), (2.75, 80, .38)],
        [(.25, 80, .45), (1.0, 76, .33), (1.75, 73, .38), (2.75, 76, .52)],
        [(.0, 83, .5), (.72, 85, .31), (1.5, 83, .38), (2.75, 80, .45)],
        [(.25, 81, .38), (1.0, 80, .33), (1.75, 78, .45), (2.75, 80, .55)],
    ]
    for n in range(4):
        for off, pitch, dur in four_bar[n]:
            if n == 0 and off == 0:
                continue
            at("hook", n, off, pitch, dur, 0.46)
    for n in range(4):
        for off, pitch, dur in four_bar[n]:
            at("hook", 24 + n, off, pitch, dur, 0.62)
    # Verse 2 gains small hook answers as its second half opens up.
    for bar, pitch in [(35, 76), (37, 78), (39, 76)]:
        at("hook", bar, 2.03, pitch, .34, 0.34)

    # The singer has priority in choruses. Guitar answers the middle pause;
    # the bell answers the end, so neither melody doubles the other.
    for first in (16, 20, 44, 48, 56):
        for n in range(4):
            bar = first + n
            response = [[80, 78], [76, 73], [83, 80], [81, 80]][n]
            if bar == 59:
                response = [80, 76]
            at("hook", bar, 3.56, response[0], .19, 0.4)
            if first in (20, 48, 56):
                at("hook", bar, 3.78, response[1], .18, 0.35)


def drum(role: str, bar: int, beat: float, pitch: int, velocity: float) -> None:
    at(role, bar, beat, pitch, 0.11 if role != "hats" else 0.075, velocity)


def compose_drums() -> None:
    for bar in range(LENGTH_BARS):
        intro = bar < 4
        verse = 4 <= bar < 12 or 32 <= bar < 40
        pre = 12 <= bar < 16 or 40 <= bar < 44 or 52 <= bar < 56
        chorus = 16 <= bar < 24 or 44 <= bar < 52 or 56 <= bar < 60
        interlude_front = 24 <= bar < 28
        interlude_back = 28 <= bar < 32

        if bar >= 2 and not (28 <= bar < 30):
            hits = [0.0, 2.5] if verse else [0.0, 1.5, 2.0, 2.75]
            if pre:
                hits = [0.0, 1.75, 2.5]
            if intro:
                hits = [0.0] if bar == 2 else [0.0, 2.5]
            if interlude_back:
                hits = [0.0, 2.5]
            for i, off in enumerate(hits):
                drum("kick", bar, off, 36, (0.91 if chorus else 0.7) - (0.08 if i else 0))
            if chorus and bar % 4 == 3 and bar != 59:
                drum("kick", bar, 3.5, 36, 0.58)
            if bar == 59:
                drum("kick", bar, 3.0, 36, 0.86)

        fill_bar = bar in (3, 15, 23, 27, 31, 43, 51, 55)
        if bar >= 3 and not (28 <= bar < 30):
            for off in (1.0, 3.0):
                if fill_bar and off == 3.0:
                    continue
                drum("clap", bar, off, 38, 0.76 if chorus else 0.54)
            if pre or (chorus and bar % 4 == 3):
                drum("clap", bar, 2.75, 38, 0.27)

        if bar >= 1:
            step = 0.25 if pre and bar % 4 >= 2 else 0.5
            if interlude_back or intro:
                step = 0.5
            index = 0
            off = 0.0
            while off < 4.0 - 1e-8:
                # A slight offbeat delay creates bounce without muddying kicks.
                swung = off + (0.055 if (index % 2 == 1 and step == 0.5) else 0)
                if not (intro and bar == 1 and off < 2):
                    if bar == 59 and off > 3.0:
                        index += 1
                        off = index * step
                        continue
                    vel = 0.3 if abs(off - round(off)) < 1e-6 else 0.41
                    if chorus:
                        vel += 0.07
                    drum("hats", bar, swung, 42, vel)
                index += 1
                off = index * step
            if (chorus and bar != 59) or interlude_front:
                drum("hats", bar, 3.5, 46, 0.39)
            if bar == 59:
                drum("hats", bar, 3.0, 49, 0.52)

        # Modern electronic low tom and tick supply the bhangra-like push.
        # GM percussion notes are retained for FL Studio drum replacement.
        if verse or chorus or interlude_front:
            for off, pitch, vel in [(0.75, 45, 0.31), (2.25, 37, 0.25), (3.25, 45, 0.37)]:
                drum("perc", bar, off, pitch, vel if chorus else vel * 0.85)

        if fill_bar:
            for off, vel in [(3.0, 0.29), (3.25, 0.36), (3.5, 0.44), (3.75, 0.55)]:
                drum("clap", bar, off, 38, vel)
            # Small pitched lift into the next section, with a quieter tail.
            at("riser", bar, 2.5, 78, 0.38, 0.35)
            at("riser", bar, 3.0, 80, 0.37, 0.39)
            at("riser", bar, 3.5, 83, 0.38, 0.43)


def build_project() -> dict:
    NOTES.clear()
    compose_harmony()
    compose_guitar()
    compose_vocal()
    compose_hook()
    compose_drums()

    sections = [
        {"name": name, "start_bar": start, "length_bars": bars}
        for name, start, bars in SECTIONS
    ]
    tracks = []
    for role, name, instrument, gain, pan, sound in TRACKS:
        clips = []
        role_notes = sorted(NOTES[role], key=lambda n: (n[0], n[1]))
        for section_name, start_bar, bars in SECTIONS:
            lo, hi = start_bar * BAR, (start_bar + bars) * BAR
            here = [n for n in role_notes if lo <= n[0] < hi]
            if not here:
                continue
            encoded = []
            for i, (beat, pitch, duration, velocity) in enumerate(here):
                # Dawwny requires each note to remain inside its clip.
                duration = min(duration, hi - beat)
                encoded.append({
                    "id": f"n-{role}-{start_bar}-{i}", "pitch": pitch,
                    "start": round(beat - lo, 5), "duration": round(duration, 5),
                    "velocity": velocity,
                })
            clips.append({
                "id": f"c-{role}-{start_bar}", "name": section_name,
                "start": lo, "length": bars * BAR, "notes": encoded,
            })
        tracks.append({
            "id": f"t-{role}", "name": name, "color": COLORS[role],
            "instrument": instrument, "gain": gain, "pan": pan,
            "mute": role in {"guitar", "vocal", "hook"},
            "solo": False, "patch": sound, "clips": clips,
        })
    project = {
        "schema_version": 1, "id": "tere-naal-shehar-v1",
        "name": "Tere Naal Shehar", "revision": 0,
        "tempo": TEMPO, "length_bars": LENGTH_BARS, "master_gain": 0.72,
        "tracks": tracks, "sections": sections,
    }
    assert len(tracks) <= 32
    assert sum(len(t["clips"]) for t in tracks) <= 128
    assert sum(len(c["notes"]) for t in tracks for c in t["clips"]) <= 32768
    assert SECTIONS[-1][1] + SECTIONS[-1][2] == LENGTH_BARS
    return project


if __name__ == "__main__":
    project = build_project()
    PROJECT_PATH.write_text(json.dumps(project, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {PROJECT_PATH}")
    print(f"Tracks: {len(project['tracks'])}; clips: {sum(len(t['clips']) for t in project['tracks'])}; "
          f"notes: {sum(len(c['notes']) for t in project['tracks'] for c in t['clips'])}")
