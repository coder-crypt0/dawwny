# Delivery milestones

Each milestone is committed, checked, pushed, and merged to the public repository separately. Completed functionality and limitations are documented with the code.

- [x] M0: native Rust workspace, architecture, shared project types, public GitHub repository
- [x] M1: validated edits and persistence, demo composition, MIDI import/export
- [x] M2: real native synthesis, transport, effects, streaming WAV export
- [x] M3: native arrangement, note properties, synthesizer/effects controls, mixer, save/load, searchable 2,310-entry sound library (72 families × 32 variations + 6 signatures)
- [x] M4: local MCP tools editing the same session with revision checks
- [ ] M5: measured release build and integration smoke checks

Later development: automation lanes, audio recording and editing, hardware MIDI, tempo maps, undo persistence, plugin delay compensation, crash-isolated VST3 hosting, plugin presets, stem freezing, authenticated remote control, collaborative sessions, and Hostinger hosting. These are not part of the initial feature-completeness claim.

## Live workflow follow-up

- [x] Seek a stopped or running transport; pause and resume without rewinding
- [x] Live mute/solo, gain, pan, master and loop changes
- [x] Preserve musical position when human or agent edits replace the graph
- [x] Computer/on-screen keyboard audition and native MIDI note/velocity/sustain input
- [ ] Record incoming MIDI; pitch bend and controller mapping
- [ ] Native sample instruments with accurate acoustic names and curated listening checks
- [ ] Audio tracks/recording and crash-isolated VST3 hosting
