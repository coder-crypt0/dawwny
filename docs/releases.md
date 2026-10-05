# Windows releases

The distributed app is **one `dawwny.exe`**, including native UI, audio DSP, demo session, stock presets, effects, MIDI support and stdio MCP. Optional SF2 libraries stay external; projects store their bank paths. No sample download is required for the stock synth.

## Reproduce and verify

From a Windows x64 checkout with Rust and C++ Build Tools:

```powershell
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
python scripts/embed-licenses.py --check
cargo build --release --workspace --locked
python scripts/smoke-standalone.py
python scripts/package-windows.py
```

The workspace's `.cargo/config.toml` enables static CRT linkage for `x86_64-pc-windows-msvc`. Normal Windows release builds use the GUI subsystem, so launching the studio does not open a console. MCP clients inherit stdio pipes when launching this same EXE with `--mcp`. The `capture` feature must be excluded from distributed builds.

The standalone smoke test copies only the EXE into a temporary folder, launches the real GUI, verifies its default session under an isolated user-data directory, closes the window and checks successful shutdown. It also negotiates MCP through the copied EXE, edits a project, rejects a stale edit and exports real MIDI/WAV files. The binary check rejects non-system DLL imports, unexpected delay imports and missing embedded notices.

## License notices

`crates/app/assets/licenses.zip` is embedded byte-for-byte. The session menu → **About dawwny & licenses** shows the application license and can save the third-party archive. Regenerate with `python scripts/embed-licenses.py` after changing dependencies. The collector uses the locked normal Windows dependency graph and pinned supplements under `third-party/`. The CI check rejects stale notices.

## Publish

Commit and merge verified source to `main`. Stage the EXE and put its exact size and SHA-256 in the release notes. Create a tag at the merged commit and publish with a notes file:

```powershell
gh release create v0.1.0 artifacts/windows-x64/dawwny.exe --target main --title "dawwny v0.1.0 — Native Windows prototype" --notes-file artifacts/release-notes.md --prerelease
```

Choose a new version for subsequent releases; published assets are not silently replaced. The only uploaded release asset is `dawwny.exe`. GitHub also provides its standard source-code archives automatically.

## Scope of v0.1

This is a MIDI composition prototype with stock synthesis, SF2 import, MIDI recording, arrangement/piano-roll editing, section cycling, live mixing, stock effects and MIDI/WAV export. Audio recording/tracks, automation, native VST hosting and remote access remain future work. Installed acoustic sample banks determine acoustic realism; the 2,310 factory entries are synth variations. Physical MIDI-controller behavior still needs hardware validation.

## v0.1.0 verified artifact

The locally built Windows x64 release is 8,136,192 bytes (7.76 MiB), built with Rust 1.93.1 and the normal feature set. SHA-256: `88aff82619517f952e7a1a773c33946da26aceb5b5e408ba2789d85c5842d82c`.

The final staged file passed GUI startup, default user-data storage and clean shutdown from a folder containing only the EXE. Its stdio MCP mode passed initialization, seven-tool discovery, project mutation, stale-revision rejection and MIDI/WAV export. The separate developer server also passed its process smoke test. The workspace passed 56 tests, Rust formatting and strict Clippy. LLVM inspection independently confirmed the x64 GUI subsystem and the same 17 Windows system DLL imports reported by the packager. The prototype is unsigned. These measurements describe this artifact; rebuilds with another toolchain need their own hash and size verification.
