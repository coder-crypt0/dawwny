# Dawwny MCP connector

`dawwny.exe --mcp` is a local Model Context Protocol server for agent control of a
Dawwny session. It communicates over stdio, so an MCP host launches the native
binary and exchanges JSON-RPC messages through its standard input and output.

## Launch

```text
dawwny.exe --mcp --project C:/music/session.dawwny.json --export-dir C:/music/exports
```

Both path arguments are optional. On Windows the project defaults to
`%LOCALAPPDATA%/dawwny/sessions/untitled.dawwny.json`; exports default to `%LOCALAPPDATA%/dawwny/exports`. Paths are
selected when the process starts. SF2 inspection accepts an explicitly provided absolute `.sf2` path with bounded loading. Export paths remain fixed at startup.

## Tools

- `list_sounds` searches the 2,310-entry Dawn catalog across 12 categories. Results contain lightweight preset metadata; optionally pass `query`, `category`, `offset`, and `limit` (1–100). The response includes `total`, the current `offset`, and `next_offset` when another page exists. The catalog comprises 72 families × 32 generated variations plus six signature presets; this is a recipe count, not a claim that every patch was professionally auditioned.
- `get_sound` takes a stable `preset_id` returned by `list_sounds` and returns the complete editable patch. Use the preset tool or update a track to load it. [Sound workflow](sound-design.md).
- `list_sample_presets` inspects actual preset metadata in a supplied local SF2 bank, with search and pagination. Use `set_sample_instrument` through `apply_commands` to choose a valid bank/program. [Sample workflow](sample-instruments.md).
- `read_project` returns the complete project document and its revision.
- `apply_commands` applies a list of typed musical commands only when
  `expected_revision` equals the stored revision. A stale request fails with a
  conflict and the caller should re-read before retrying.
- `export_midi` writes a MIDI export beneath the configured export directory,
  using the committed revision and a unique ID in the filename, preserving prior exports.
- `render_wav` renders through the native audio boundary and writes beneath
  the configured export directory.

Every mutation is validated and persisted as one transaction. A successful
mutation returns the resulting project, including its new monotonic revision.
The connector does not expose shell execution, arbitrary file reads, or plugin
installation as MCP capabilities.

## Integration

The connector uses the official Rust `rmcp` server macros and stdio transport.
Diagnostics must go to stderr; stdout is reserved for MCP JSON-RPC traffic.
The minimal production feature set is `server`, `macros`, and `transport-io`
with default features disabled, keeping the native binary small.

Agents should treat `apply_commands` as optimistic concurrency: send the
revision returned by the last read or mutation, handle conflicts explicitly,
then rebase intended commands onto the latest project.

## Client configuration

Download the single Windows EXE or build with `cargo build --release -p dawwny-app --locked`. In the studio, use **Agent connection → Copy MCP configuration** to get the exact executable, session and export paths. A generic MCP host configuration is:

```json
{
  "mcpServers": {
    "dawwny": {
      "command": "C:/path/to/dawwny.exe",
      "args": ["--mcp", "--project", "C:/music/session.dawwny.json", "--export-dir", "C:/music/exports"]
    }
  }
}
```

The server initializes a demo session if the selected file does not exist. Existing files are never replaced during startup. Invalid CLI options fail rather than silently falling back to another session. MCP mode opens no studio window or audio device, and errors go to stderr with a failing exit code. Export requests run on a blocking worker outside the protocol loop; the per-process export semaphore allows one job at a time. WAV output is 48 kHz, 24-bit stereo. The full protocol integration test uses the official SDK client, negotiates a session, edits the project, tests stale revision rejection, and verifies MIDI/WAV exports.

The separate `dawwny-mcp` developer binary remains available for existing integrations and uses the same server implementation. It is not needed or included in the portable release. Linux/macOS data defaults follow the user's application data directory; these platform builds remain unverified.

## Sections and cycle ranges

Use `set_sections` to replace the arrangement markers and `set_cycle_range` to select a playback range. Bar positions in sections are zero-based; cycle positions are quarter-note beats, with an exclusive end. A null range restores whole-song cycling. The native studio reads these edits without resetting playback. Its local cycle button still controls whether cycling is enabled. WAV/MIDI exports render the complete song once.

```json
{
  "expected_revision": 4,
  "commands": [
    {"type": "set_sections", "sections": [{"name": "Chorus", "start_bar": 4, "length_bars": 4}]},
    {"type": "set_cycle_range", "range": {"start": 16.0, "end": 32.0}}
  ]
}
```

## Edit recorded performances

Recorded takes become ordinary MIDI clips, visible through `read_project`. `quantize_clip` accepts `track_id`, `clip_id`, and a `grid` of 0.0625–4 quarter-note beats. It aligns note starts without changing duration, velocity or identity; notes near the end move to the latest fitting grid line. `transpose_clip` takes the same IDs and `semitones` from -24 to 24. Any out-of-range note rejects the whole transaction. Both commands preserve the revision/atomicity guarantees of `apply_commands`.
