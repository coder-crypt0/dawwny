# Dawn factory library

The bank contains **2,310 presets**: 72 authored subtractive sound families with 32 tuned variations each, plus six signature sounds. It covers Bass, Lead, Pad, Keys, Pluck, Bell, Organ, Brass, Strings, Motion, Texture, and Percussive. These are synthesized colors; acoustic category names describe musical roles, not sampled acoustic realism.

Each family has its own oscillator combination, tuning, filter, amplitude envelope and sub/noise balance. Eight tonal/articulation treatments combine with four space treatments (Dry, Room, Wide, Echo). Stable preset IDs are part of the API. This is a family-and-variation library, not a claim that 2,310 unrelated instruments were individually auditioned.

## Browse and play

Open **Sounds** (shortcut **L**). Filter by category and search name, family or tags. Select a result to read its description and preview it. Double-click or choose **Use sound** to apply it to the selected track. Preview plays a separate phrase, stops arrangement playback, and never edits notes or the session. Sound changes preserve track notes and mix settings and participate in autosave/undo.

Stars save favorites locally next to the session directory in `.dawwny-library.json`. The result list only draws visible rows. Patches are built when selected; no sampled audio library or thousands of preset JSON files are shipped.

## Agent access

- `list_sounds` accepts optional `query`, `category`, `offset`, and `limit` (default 24, maximum 100). It returns lightweight metadata, total matches and `next_offset`.
- `get_sound` accepts `preset_id` and returns the complete editable patch.
- `apply_commands` accepts `apply_sound_preset` with `track_id` and `preset_id`. Use the current project revision.

Example IDs: `amber_keys`, `factory.soft-sub.00`, `factory.dusk-wash.10`. The numeric suffix is a stable variation ID; names and tags expose its musical character.

## Audit

Run `cargo run --release -p dawwny-audio --example audit_bank --locked` for a 16 kHz smoke audit of every preset, or append `-- --full` for 48 kHz renders through the complete scheduled tail. Reports are local CSV files under `artifacts/`.

The first 16 kHz pass rendered all 2,310 presets for six seconds each, with a three-second note at the category's audition pitch. All were valid, finite, audible, below the limiter threshold, and distinct under a deterministic waveform hash after quantization to 1e-5. Maximum single-note output peak was 0.126919; minimum whole-excerpt RMS was 0.000435; maximum absolute DC was 0.000029. The RMS includes silence and release and is therefore not a loudness-matching measure.

Automated checks catch silence, unsafe levels, exact audio duplicates and rendering errors. They do not establish pleasantness, mix translation, genre coverage, or subjective similarity. A systematic listening pass and broader musical tests remain necessary before calling the entire bank production-curated.
