# Native sample instruments

Use **Sounds → Sample instruments → Import SoundFont** to open a local SF2 bank. Search its actual preset names and double-click a preset, or choose **Use on track**. This preserves MIDI notes, mixer settings and effects. **Keys / Ctrl+K** plays the selected instrument from the computer keyboard, on-screen keys or a connected MIDI controller. Import runs on a worker; sample processing uses native RustySynth with no browser runtime.

For a compact bank covering piano, orchestral instruments, guitars and percussion, install the optional [GeneralUser GS](https://github.com/mrbumpy409/GeneralUser-GS) by S. Christian Collins:

```powershell
python scripts/install-sound-bank.py
```

The installer pins version 2.0.3 to an upstream commit and verifies SHA256 before replacing a bank. It downloads the bank and its license into the ignored `sample-banks` directory. Samples are not bundled into Dawwny's executable or Git repository. Import `sample-banks/GeneralUser-GS.sf2` from the studio. You can also import another legally obtained SF2 bank.

## Agent workflow

Call `list_sample_presets` with an absolute SF2 file path, optional query, offset and limit. The response returns actual names, the canonical file, bank and zero-based program. Then send `set_sample_instrument` through `apply_commands` with the current revision:

```json
{"type":"set_sample_instrument","track_id":"track-id","sample":{"file":"C:/Music/Soundfonts/bank.sf2","bank":0,"program":24}}
```

Missing files and unknown presets are rejected before an agent transaction is saved. Humans and agents use the same renderer for real-time playback and WAV export. MIDI export writes the bank/program selection; it does not embed the samples. Bank 128 exports on MIDI channel 10 for percussion.

## Resource limits and compatibility

- Banks are shared across tracks and replacement graphs. Weak cache entries release unused banks; no file reads, locks, allocation or destruction occur in the sample loop.
- A bank file is limited to 256 MiB. The process limits the combined estimate for loaded sample data and mapping objects to 256 MiB. This is a sample budget, not a promise of total process RAM.
- The arrangement keeps its 128 note-event slots. SF2 sample-layer polyphony is at most 64 per track and 512 across arrangement samplers; the live monitor has a separate bounded sampler. Sample rates are 16–192 kHz.
- SF2 notes use their bank's sample mapping, velocity zones, envelopes and filter. Dawwny's stereo effects remain available. Dawn oscillator and envelope controls do not rewrite an SF2 instrument.
- Unsupported: SF3 compressed banks, ROM banks, custom SF2 modulators, SFZ, articulation switching, streaming large sample libraries and VST instruments. Sound quality depends on the imported bank. General MIDI coverage does not imply premium orchestral articulation depth.
- Seeking retriggers held sample notes with their remaining gate. It does not reconstruct past sample phase or effects. Release estimates and rack tails are bounded; bank release estimates cap at 30 seconds.
- Projects keep absolute bank paths. Relocate a moved bank by importing it again and selecting its preset; share the bank separately in accordance with its license.

The synthetic test bank is generated during tests and is never presented as an acoustic library. Tests cover velocity zones, gates, same-pitch overlaps, sustain, panic, seek, sharing, malformed containers and zero allocation/free on sample playback.

## Recorded integration check

On 2026-10-05, the pinned GeneralUser GS 2.0.3 bank exposed 287 preset entries (including mirrored drum banks). The audit rendered all 287 at 16 kHz: every entry produced finite, non-silent output at the audit pitches, with a maximum peak of 0.6822 and a 34.0 MiB loading estimate. This checks engine routing and stability; it is not a listening-based rating of every sound or full custom-modulator compatibility. Run it with `cargo run -p dawwny-audio --example audit_samples -- ABSOLUTE_SF2_PATH`.
