# Tere Naal Shehar

An original, upbeat romantic Punjabi-pop composition built as a complete
Dawwny session. The song celebrates being together in a city at night. It has
no borrowed melody, lyric, sample, or chord transcription.

## Open and listen

- [Dawwny project](tere-naal-shehar.dawwny.json) — the editable 60-bar session.
- [Multitrack MIDI](tere-naal-shehar.mid) — 93 BPM, 4/4, 12 named instrument tracks.
- [Current mix](tere-naal-shehar-current.mp3) — matches the saved Dawwny mute state.
- [Full guide preview](tere-naal-shehar-guide.mp3) — guitar, instrumental hook,
  and synthesized vocal guide audible for reference.
- [Singing backing](tere-naal-shehar-instrumental.mp3) — guitar and hook audible;
  vocal guide muted.
- The native 48 kHz, 24-bit stereo WAV renders are in the repository's ignored
  `exports/` directory with matching `-current.wav`, `-guide.wav`, and
  `-instrumental.wav` suffixes.

Tracks **03 Guitar**, **05 Vocal Guide**, and **06 Instrument Hook** are muted
in the delivered Dawwny project as requested. Their notes remain editable and
are retained in the MIDI file. The guide is a monophonic synth that marks the
intended vocal pitches and phrasing; it is not a sung vocal take. The chart
below describes the complete written arrangement, including muted parts.

| Section | Bars (1-based) | Arrangement change |
| --- | ---: | --- |
| Intro | 1–4 | Picked motif, keys, then pad, bass, hats, and pickup fill |
| Verse 1 | 5–12 | Low vocal register, sparse guitar answers, pocket groove |
| Prechorus 1 | 13–16 | Vocal contour climbs; hats and chord pulses build |
| Chorus 1 | 17–24 | Brighter chords, wider pad, strong kick and hook responses |
| Interlude | 25–32 | Instrument hook takes the lead, then a four-bar breakdown |
| Verse 2 | 33–40 | Same harmonic foundation; second-half chord pulses and fills |
| Prechorus 2 | 41–44 | Same five-attack rhythmic cell as the chorus |
| Chorus 2 | 45–52 | Main hook returns with the fuller answering tag |
| Outro lift | 53–56 | Prechorus harmony and buildup prepare the final return |
| Final chorus | 57–60 | Hook reprise and B7-to-E final cadence |

## Musical design

**Key: E major.** The vocal guide spans E4–F#5 (MIDI 64–78). The tonic is
bright and celebratory; C#m and F#m lend romantic warmth without making the
song feel like a slow ballad. Keyboard chords use close voice leading and
shared tones. Chords change within bars, while held pad tones glue the motion
together. The bass follows the slash notes and often anticipates the next
change.

The core verse cycle uses these beats per bar:

| Bar | Harmony |
| --- | --- |
| 1 | Emaj9 (2½ beats) → B/D# (1½) |
| 2 | C#m9 (2½) → G#m7/B (1½) |
| 3 | Amaj9 (2½) → E/G# (1½) |
| 4 | F#m11 (2) → B7sus4 (1) → B7 (1) |

The second four bars vary the bass path and harmonic timing. The prechorus
moves **C#m7 → Aadd9 | F#m7 → B7sus4 → B7** twice, with the last pass using
an A/B suspension. The chorus answers with
**Eadd9 → B/D# | C#m7 → Aadd9 | E/G# → F#m7 → B7sus4 | Amaj9 → B7sus4 → B7**.
The final bar resolves the last dominant to Eadd9 inside the requested
four-bar final chorus.

The prechorus and chorus vocal lines use the same five-note attack rhythm in
every bar: quarter-note offsets **0, 0.55, 1.30, 2.55, 3.05**. The
prechorus rises into the chorus; the chorus repeats a simpler contour. The
picked guitar answers the middle gap in each chorus phrase and the bell lead
answers the ending gap. In the interlude, the bell lead gets its own complete
four-bar melody. The drums use a syncopated electronic kick, snare on 2 and
4, swung eighth hats, and low electronic percussion for a subtle bhangra
push. No traditional Punjabi instrument patch is used.

## FLEX handoff

Image-Line's [MIDI import guide](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/automation_midiimport.htm)
documents the **FLEX** channel type and **Create one channel per track**
option. Import `tere-naal-shehar.mid` from FL Studio's main **File → Import →
MIDI file** flow and enable those options. The MIDI contains the notes,
tempo, names, and General MIDI program hints; Dawwny's custom synth and mix
effects do not travel inside a standard MIDI file.

| Dawwny track | FLEX / FL Studio sound direction |
| --- | --- |
| 01 Chord flow | Warm electric piano or rounded poly keys, short room |
| 02 Common-tone air pad | Dark or airy pad, high-pass below the bass |
| 03 Guitar-style riff | Clean picked electric guitar or short pluck; keep pan right |
| 04 Round sub bass | Mono round bass with controlled sub and light saturation |
| 05 Vocal guide | Soft lead while rehearsing; mute before recording vocals |
| 06 Instrument hook | Bell or clean sine lead, modest delay |
| 07 Chorus chord pulses | Short electric-key attack, lower than the vocal |
| 08–11 Drums | Replace the General MIDI kit notes with your chosen kick, clap, hats, and electronic percussion |
| 12 Transition air | Filtered noise riser or FX sweep |

FLEX's library and installed packs vary, so the table gives sound roles rather
than a pack name that might not be available. Dawwny's native preview uses a
synthesized guitar-like pluck; track 03's MIDI is ready for a convincing
guitar patch in FLEX. Adjust octave and envelope after selecting that patch,
but keep the notes and syncopation for the intended answer pattern.

## Rebuild

From the repository root:

```powershell
python songs/tere-naal-shehar/compose.py
python songs/tere-naal-shehar/render.py
```

`compose.py` recreates the score and overwrites the Dawwny project. Run
`render.py` alone after making edits in Dawwny: it reads the current project,
validates and exports through Dawwny's own Rust code, then makes the MP3
listening copies with FFmpeg. It preserves the editable project and its mute
state. The full-guide and singing-backing previews use temporary copies.

## Reference study

The official [For A Reason](https://www.youtube.com/watch?v=cWIGUPQqnSk) and
[You're U Tho](https://www.youtube.com/watch?v=rlf48XUyjng) credits list a
production palette of guitar, bass, synths, and programmed drums. In
[Spotify's Ikky interview](https://newsroom.spotify.com/2025-10-09/karan-aujla-ikky-punjabi-music/),
he describes Punjabi music meeting hip-hop, R&B, and dancehall influences;
[his earlier interview](https://newsroom.spotify.com/2023-06-15/can-you-hear-me-returns-as-toronto-music-producer-ikky-creates-a-musical-melting-pot/)
also describes the bhangra and pop blend. Those are arrangement references
for this song's instrument roles, emotional center, and groove. All harmonic,
rhythmic, and melodic content here was composed for this project.

## Verification

The generated session loads through Dawwny's validator, exports as a type-1
MIDI file with 12 named parts, and renders without stolen voices. The 60-bar
form is exact, and all pitched notes belong to E major. I revised the bass
passing tones, chorus/verse contrast, vocal/instrument answer spacing, and
final cadence after the first render. The MP3s were checked for duration,
loudness, and the expected differences between the three mixes.
