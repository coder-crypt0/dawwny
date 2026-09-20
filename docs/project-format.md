# Dawwny project format

Dawwny projects are UTF-8 JSON documents. The root object is a `Project` with schema version, stable id, revision, tempo, bar length, master gain, tracks, and sections. Tracks contain clips; clips contain notes whose timing is measured in quarter-note beats.

Projects are validated at every load and write. A project may contain at most 32 tracks, 128 clips total, 32,768 notes total, and 256 bars. Tempo is 30–300 BPM in fixed 4/4. Schema version must be 1. Track, clip, and note IDs must be globally unique within a session; labels are limited to 128 bytes. Gains and velocities are 0–1, pans are -1–1, and note/clip timelines must remain within the project length. ADSR times, filter cutoff, and effect sends have finite bounds enforced by the core validator.

Commands are tagged JSON objects, for example `{ "type": "set_tempo", "tempo": 92.0 }`. Session revisions increase monotonically; stale edits are rejected.

The UI and MCP process share the same file. Every transaction acquires a nonblocking sidecar file lock, checks the expected revision, validates the result, and atomically replaces the JSON file. Contention returns a retryable error. File data is synced before replacement. The UI polls file metadata while idle and keeps external edits out of an in-progress mouse gesture.

JSON reads are limited to 16 MiB. MIDI imports are limited to 4 MiB and 200,000 events. MIDI type 0/1, constant tempo, 4/4, pitches, velocities, and channel separation are supported. Type 2, SMPTE, tempo maps, other time signatures, overlong projects, and unterminated notes fail explicitly. Controller automation, sustain pedals, pitch bend, and arbitrary program changes are not reproduced; this is a note-sequence importer. MIDI export retains all tracks, including muted tracks, while WAV export follows the audible mix. More than 15 melodic tracks reuse MIDI channels in separate MIDI tracks.

`SynthPatch.synth` contains the custom oscillator, filter, and LFO settings; `SynthPatch.effects` is an ordered array of up to eight enabled/bypassed native effects. Missing fields in older sessions default safely. Custom synthesis applies to `instrument: "synth"`; rack effects work on every instrument. [Sound schema and ranges](sound-design.md).
