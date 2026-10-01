# Studio interface

Dawwny uses a native egui/Glow workspace. The arrangement remains the center of composition; the library is a leading sidebar, and the piano roll, sound designer, and mixer share a resizable lower editor. Expand gives detailed editing the full workspace without opening another window.

The design follows the grouping and hierarchy guidance in Apple's [toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars) and [sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars), adapted to a native Windows application:

- Project actions live at the leading edge, transport and timing form one group, and view/export actions sit at the trailing edge.
- The sound library can collapse when composition needs more space. Search, category filtering, favorites, and preview are available without opening nested dialogs.
- Rounded workspace surfaces, controls, clips, notes, and mixer strips use a consistent radius scale. Flat musical grids preserve accurate alignment and timing.
- Most interface chrome is neutral. Track colors identify musical material; the lime accent marks transport and active controls.
- Windows uses the locally installed Segoe UI font, with egui's bundled fonts as fallback. No Apple fonts, icons, or other proprietary assets are redistributed.
- Familiar shortcuts, visible action names, hover descriptions, note context menus, and non-destructive preview reduce the need to learn a new workflow.

The list virtualizes visible rows rather than constructing thousands of widgets. Search results cache until filters change; preset DSP settings are generated only when needed. The interface draws on interaction and at a limited rate during playback instead of continuously animating an idle workspace.

This is an iterative native prototype. The current piano canvas lacks full keyboard editing and screen-reader navigation; drag feedback, focus behavior, contrast, and compact-screen layouts need continued usability work. HIG-informed styling does not imply complete Apple HIG compliance or parity with a mature DAW.
