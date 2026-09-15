# Dawwny MCP connector

`dawwny-mcp` is a local Model Context Protocol server for agent control of a
Dawwny session. It communicates over stdio, so an MCP host launches the native
binary and exchanges JSON-RPC messages through its standard input and output.

## Launch

```text
dawwny-mcp --project sessions/untitled.dawwny.json --export-dir exports
```

Both arguments are optional. The project defaults to
`sessions/untitled.dawwny.json`; exports default to `exports`. Paths are
selected when the process starts. MCP tool arguments never choose arbitrary
filesystem paths.

## Tools

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

Build both binaries with `cargo build --release --workspace`. In the studio, use **Agent connection → Copy MCP configuration** to get the exact local paths. A generic MCP host configuration is:

```json
{
  "mcpServers": {
    "dawwny": {
      "command": "C:/path/to/dawwny-mcp.exe",
      "args": ["--project", "C:/music/session.dawwny.json", "--export-dir", "C:/music/exports"]
    }
  }
}
```

The server initializes a demo session if the selected file does not exist. Existing files are never replaced during startup. Invalid CLI options fail rather than silently falling back to another session. Export requests run on a blocking worker outside the protocol loop; the per-process export semaphore allows one job at a time. WAV output is 48 kHz, 24-bit stereo. The full protocol integration test uses the official SDK client, negotiates a session, edits the project, tests stale revision rejection, and verifies MIDI/WAV exports.
