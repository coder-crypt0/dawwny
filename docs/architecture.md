# Architecture

## Native first

The native app draws directly through egui/Glow and plays through CPAL. It does not start a browser engine. OpenGL is the initial renderer to avoid shipping a second GPU abstraction/backend stack. The UI repaints on interaction and at a bounded rate during playback; the audio clock owns musical time.

The canonical project is a versioned, validated document shared by human editing and agents. Editing and serialization happen away from the device callback. The audio engine compiles immutable note schedules before playback, uses a fixed voice pool, and streams offline exports to disk instead of buffering whole songs.

## Boundaries

- `core` has no UI, audio device, or model-provider dependency.
- `audio` consumes core projects and never invokes an LLM.
- `app` provides explicit controls for all supported musical parameters.
- `mcp` exposes schema-checked musical operations. Users bring their preferred MCP client/AI; no API key or AI subscription is built into the app.
- Native third-party plugins belong in a future crash-isolated host process. The browser cannot directly load installed native plugin binaries. Plugin binary upload is not part of the design.

## Performance principles

Bound voice counts and document sizes; compile musical events off the audio thread; avoid locks, allocation, disk, network, and logging inside the callback. Stop needless visual work when idle. Share one DSP implementation between live and offline playback. Measure release executable size, process working set/private memory, render throughput, and callback underruns before claiming an optimization.

## Future web access

A lightweight remote editor will control the same native engine over an authenticated session. It will not duplicate the renderer/DSP/plugins in a Chromium desktop shell. Hostinger deployment is deferred; no site hosting is configured.

## Technical references

- [egui native framework and renderer features](https://docs.rs/eframe/0.33.3/eframe/)
- [CPAL native audio I/O](https://docs.rs/cpal/0.16.0/cpal/)
- [Official MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Steinberg VST3 documentation](https://steinbergmedia.github.io/vst3_dev_portal/)

