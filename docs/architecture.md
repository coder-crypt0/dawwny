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

## Transport and instrument input

Space pauses/resumes at the current position. Stop returns to beat zero. Click or drag the arrangement ruler to seek, including while stopped. Mix controls update the running graph without discarding voices or effect state; a short ramp smooths gain, pan, mute, solo and master changes.

Tempo, notes, or instrument/effect changes prepare a replacement graph on the control thread. The callback swaps it at a block boundary and preserves musical position, chasing notes that cross the new position. Replaced graphs are reclaimed on the control thread through a bounded return queue. Seeking starts with fresh effect history rather than reconstructing the complete preceding wet tail.

Computer-keyboard, on-screen-keyboard and native MIDI input use a separate live monitor of the selected track. They can sound while the arrangement is paused or stopped. MIDI note-on/off, velocity, sustain and panic are supported. Input recording, pitch bend and expression mappings remain future work.

The native callback consumes bounded queues and does not lock the MIDI producer mutex. Renderer construction, file work and buffer destruction remain on the control thread. Tests exercise callback graph swaps, seeking, mix commands and live input under an allocation/free counter. CPAL stream setup and teardown are outside that measured processing path.

SF2 imports use native RustySynth. Sample data is shared across tracks and prepared graphs; loading and validation occur on control/worker threads. Stereo sample output feeds the same effect rack as Dawn. Bank inspection and preset selection are exposed through MCP with explicit resource bounds. See [sample instruments](sample-instruments.md) for compatibility and memory limits.

MIDI recording copies timed input events into a bounded 4,096-entry callback-to-UI ring. Events use the arrangement audio clock at block boundaries. Queue overflow is explicit. The UI reconstructs gates/velocity/sustain, closes held notes, and adds an ordinary clip through the same validated document path. Finishing acknowledges a block boundary before draining final input, with a bounded 100 ms control-thread timeout; the audio thread never waits. A local recovery project protects takes against external document changes. Recording runs linearly and disables playback cycle; capture does not allocate or free memory in the callback.
