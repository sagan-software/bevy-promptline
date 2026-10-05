# Console interaction and browser examples

This is a design brief for later implementation. No rendered UI or browser build was created during planning.

## Shared interaction

The console's primary task is entering a command. Keep the input visible and focused while help updates outside it. Show output above the input, completion candidates near the input, and contextual help in reserved space. Long help scrolls independently rather than expanding over the input.

When the draft is empty, show a short list of discoverable commands and their summaries. When the player types `scene ro`, show the suffix of `rotate` as ghost text and explain the command. When the caret follows a space after `scene rotate --speed`, show the speed argument's description, unit, and valid range from command metadata.

Never insert help into the draft. Keep syntax errors local while editing. On submission failure, retain the draft and place a readable error in output. Successful submission records history and clears the draft. Output arrival does not steal focus or change completion selection.

The example scene displays `Press ~ to open the console` and an `Open console` button. Update the key label when bindings change. Close controls remain visible in movable and touch layouts. Keep keyboard navigation, pointer access, 200% zoom, small canvases, and reduced motion in the acceptance checks.

Use typography, spacing, and alignment for hierarchy. Avoid redundant introductory text, nested decorative panels, promotional copy, and full-time toolbars in the minimal example. Advanced controls appear where their example needs them.

## Six independent examples

- `minimal`: the smallest integration without Clap or log capture. Show a rotating primitive over a floor and register `echo`, `color`, and `spin`. Prove command completion, ghost text, help, highlighting, output, and typed delivery. The README tutorial comes from this example.
- `quake_drawer`: a top-anchored console with a dark translucent background, monospace input, and a smooth slide controlled by example systems. `scene speed` and `scene reset` visibly affect the scene. When toggled midway, reverse animation continuously. Reduced motion opens immediately. The core does not own this animation.
- `source_window`: a Half-Life/Source-inspired movable and resizable Bevy UI window over the same scene. Add a title bar, close control, output selection, and history. Pointer capture keeps dragging active outside the title bar. Clamp the window into the resized viewport. Do not copy Valve assets, fonts, logos, or source code.
- `crt_workstation`: a desk and CRT monitor in a dark void. Render the Bevy UI console to the screen texture and apply a WGSL CRT effect to that texture. Demonstrate DOS-inspired commands, file completion, shader controls, and a close-up editing view. Follow [the CRT brief](crt-workstation.md).
- `clap_commands`: register an existing derive parser and a builder command. Use nested `scene spawn`, `scene light`, and `scene rotate` subcommands, flags, enum values, aliases, negative numbers, `--`, help, and version output. Game-provided model names complete dynamically. Show parse errors without terminating the app.
- `kitchen_sink`: combine optional Clap and log capture, command policy, typed lifecycle observers, a pending command, cancellation, output filtering, history search, configurable theme, and layout switching. Provide a bounded log burst command to demonstrate scrollback limits and loss reporting. Retain a compact prompt layout and a light theme as presentation modes. Reuse the smaller examples' systems rather than creating a second command engine.

Share scene and command helpers under `examples/support/`. Each example remains a separate Cargo example target and independent Wasm artifact. Do not create one app whose mode selector substitutes for the six builds.

Use procedural meshes and embedded or openly licensed fonts initially. A recognizable primitive is enough to show visible command effects. Record licenses for any later model assets. Avoid large downloads that delay console interaction.

## Gallery

Use [Bevy's examples gallery](https://bevy.org/examples/) as the functional reference. Each example gets a title, concise purpose, thumbnail captured from its actual build, source link, keyboard controls, and a dedicated run page. Group entries by basic integration, presentation, and advanced integration.

Each run page loads only its example's JavaScript, Wasm, and assets. Show actual loading, fetch failure, unsupported graphics, and WebGL context-loss states. Include retry and a static explanation when rendering fails. Give the canvas focus after an explicit interaction. A page must not intercept Tab while its canvas is unfocused.

Build WebGL2 first for the six required examples. Build WebGPU separately if useful; enabling both in one build is not a fallback strategy. Use the [tagged Bevy Wasm guide](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/README.md#wasm) to produce bindings and copy assets. Match the `wasm-bindgen` CLI to the resolved crate version.

Serve under a project subpath such as `/bevy-promptline/`. Test asset URLs at that subpath and by direct navigation to a run page. Use a local HTTP server for development. The implementation-session prompt authorizes GitHub Pages publication when pasted as the next task. This planning session does not publish anything.

Proposed output layout is `site/<example>/` with one HTML entry, generated bindings, a Wasm binary, and that example's assets. Keep generated outputs outside tracked source. A declarative example inventory drives the index and build list so adding an entry cannot silently omit its build.

## Visual and interaction review

Capture each example at desktop size, Steam Deck size, and a narrow browser viewport. Include open, closed, completion, long help, parse error, selected output, and overflow states. Inspect light and dark text contrast without relying on token color alone.

Run one batched review, fix observed defects together, and confirm with one bounded repeat. Check clipping, caret and ghost alignment, focus restoration, viewport clamping, long Unicode input, and help-region stability. Remove redundant copy and containers before handoff. Report any unavailable native, mobile, or accessibility verification.

Historical styling is example-specific. The default console stays restrained and customizable. Animation, window movement, and themes must use documented extension points without reaching into private library state.
