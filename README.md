# dawwny

A native, agent-controllable digital audio workstation. Rust audio engine, native GPU-drawn interface, open project files, and a local Model Context Protocol connector. No Electron, Chromium, WebView, cloud account, or bundled AI model.

**Status: foundation under active development.** The shared project format, revision-checked persistence, musical edits, and MIDI import/export are implemented. Playable synthesis, editing, export, and MCP integration follow in independently verified milestones. This is not yet a replacement for a mature production DAW.

## Repository

| Package | Responsibility |
| --- | --- |
| `crates/core` | Versioned project model, validation, commands, persistence, MIDI |
| `crates/audio` | Bounded native synthesis, scheduling, device output, offline WAV rendering |
| `crates/app` | Native arrangement, piano roll, mixer, and sound editor |
| `crates/mcp` | Local MCP server for user-selected agents |
| `docs` | Architecture, milestones, performance methodology, agent protocol |

## Development

Install Rust 1.88 or newer and your platform's native build tools. Windows needs the Visual Studio C++ build tools and Windows SDK. Linux will additionally need ALSA, X11/Wayland and OpenGL development libraries.

```sh
cargo test --workspace
cargo run -p dawwny-app
cargo build --release --workspace
```

Source is public; development and playback stay local. Future remote control will use Hostinger only when deployment is explicitly requested. Installed native plugins require a dedicated host; arbitrary VST/AU/AAX loading is not implemented or implied.

See [architecture](docs/architecture.md) and [roadmap](docs/roadmap.md).
