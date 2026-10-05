# Studio interface rules

Dawwny follows the interaction principles in Apple's [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines), particularly [buttons](https://developer.apple.com/design/human-interface-guidelines/buttons), [toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars), and [layout](https://developer.apple.com/design/human-interface-guidelines/layout). It remains a native Windows/Linux interface rather than an imitation of Apple's platform materials.

The shared values in `crates/app/src/design.rs` define roles across the library, arrangement, sound designer, mixer and floating windows:

| Role | Corner radius in logical points | Use |
| --- | ---: | --- |
| Control | 8 | Buttons, fields, selection rows |
| Surface | 12 | Editor panels, channel strips, grouped content |
| Window | 14 | Floating editor windows |
| Musical content | 6 | MIDI clips, notes, keyboard keys, sections |

Grid lines and boundaries stay straight for timing alignment. Tiny meters and marks use smaller radii. These are intentional differences in role, rather than per-screen decoration. Spacing uses an 8-point baseline, a 6-point compact row gap and 12-point internal panel padding.

Green identifies the principal playback action and selected tools. Amber identifies mute and active cycle ranges everywhere. Track colors identify musical content. Muting dims a clip without hiding its notes. Standard button interactions provide hover/press feedback and accessibility semantics, including on the custom arrangement canvas.

Toolbars group document, transport and workspace commands. Secondary instrument and effect choices use menus or tabs. At narrower widths the synthesis editor changes from three columns to stacked groups. Default native pointer controls have a 30-point interaction height; compact track mute/solo controls use smaller bounded targets within their lanes. This build does not claim touch-first support or full platform HIG compliance.

Regression checks render all editor tabs at 940, 1440 and 1920 logical pixels. A pointer-event test double-clicks a rendered section button and verifies its stored range and playhead. Native captures use the opt-in `capture` feature; normal builds exclude capture dependencies.
