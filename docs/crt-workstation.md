# CRT workstation example

The `crt_workstation` example is a required presentation demonstration. It replaces the separate compact-prompt target; the kitchen-sink example retains compact layout mode. The project still ships six independent browser examples.

## Visual direction

Place a wooden desk, beige computer case, keyboard, and old CRT monitor in a dark void. Use a warm desk light and readable green or amber phosphor text. Frame the monitor prominently enough that command entry is useful in a browser. Provide a close-up view and a wider desk view.

Model the initial scene with procedural Bevy meshes or original locally created assets. Record asset and font licenses. The monitor should have visible depth, a bezel, glass, restrained material variation, and intentional lighting. Avoid using a plain floating rectangle as the final demonstration.

The console remains Bevy UI. Render its UI camera into an offscreen texture, then sample that texture on the monitor's screen mesh through a custom WGSL material. Keep this presentation and its 3D dependencies in the example. The core plugin must not require PBR, CRT shaders, or a desktop model.

Use [Bevy's tagged UI-to-texture example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/render_ui_to_texture.rs) for camera targeting and pointer routing. Use [the tagged UI material example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/ui_material.rs) for shader integration constraints. Inspect the selected version's 3D material APIs before implementation.

## Shader scope

Apply scanlines, phosphor tint, a soft vignette, subtle RGB separation, and adjustable barrel distortion only to the screen texture. Keep the desk, monitor housing, and other UI unaffected. A full-scene retro filter cannot substitute for a shader applied to console content.

Use a fragment shader compatible with WebGL2. Align uniforms as required by Bevy's renderer. Keep bloom optional because it adds renderer and pass costs. Bound screen-texture resolution and GPU work. Resize deliberately rather than reallocating the target every frame.

Proposed controls are `crt scanlines`, `crt curvature`, `crt tint`, `crt reset`, and a shader-disabled comparison. Use dimensionless validated ranges for strengths and normalized curvature. The implementation must document exact ranges and units beside the shader uniforms and command metadata.

Keep ordinary text stable. Do not add flashing, rolling bars, or intermittent glitches by default. Provide a reduced-effects mode and a flat close-up mode for small screens. When reduced motion is requested, disable moving noise and animated screen effects. Keep text, help, selection, and ghost completion legible in every mode.

[bevy_retro_shaders](https://github.com/kodaskills/bevy_retro_shaders) is a useful effect reference, but its documentation places the effect before ordinary UI rendering. That position alone does not apply CRT effects to console text. [bevy_tui_texture's CRT example](https://github.com/tt-toe/bevy_tui_texture/blob/main/examples/retro_crt.rs) demonstrates the scene idea with Ratatui textures. Promptline will render its own Bevy UI console.

## Input mapping

Clicking the glass focuses the console and switches to the close-up view. Escape releases console focus before returning to the wider view. Provide a visible focus control for touch and keyboard users. Camera controls and gameplay input are gated while console editing owns focus.

Raycast only the screen mesh. Convert the hit's normalized UV coordinate through the exact sampling transform used by the shader, then into texture pixel coordinates. Define horizontal orientation and vertical origin explicitly. Pointer mapping must use the sampled coordinate, not an independently guessed inverse distortion.

If the shader maps displayed UV to source UV, apply that same map to the mesh hit. If the mapping is outside the source image or masked by the rounded glass border, reject the hit. Require a one-to-one mapping over the supported interaction region; constrain curvature so selection does not have ambiguous display positions.

Forward pointer movement, button presses, releases, and leave events to the virtual UI pointer. Clear stale pointer state after a miss or focus loss. Continue an active selection drag until release, including when the pointer leaves the glass, using a documented clamp policy. Scale texture coordinates correctly under viewport resize and device pixel ratio changes.

Test the four ordered screen corners, center, edges, curvature extremes, masked borders, leaving the screen, and drag release. Assert coordinate error within one source-texture pixel. Compare rendered caret placement to the clicked glyph in a real browser. GPU shader compilation alone does not prove input mapping.

## DOS-inspired commands

Use a virtual `C:\>` prompt and a small read-only in-memory filesystem containing a guide, example command descriptions, and scene configuration text. Implement `HELP`, `DIR`, `CD`, `TYPE`, `CLS`, `ECHO`, and `VER` in the example with the shared command runtime.

Resolve these command names without ASCII case sensitivity within this example. Preserve argument spelling and the actual draft. Keep the core's ordinary case-sensitive name profile unchanged. Display the current virtual directory in the prompt.

The supported subset uses simple relative file names and child-directory names, such as `CD DOCS` and `TYPE README.TXT`. Document the accepted name grammar, root-parent behavior, unknown names, and quoted echo values. Keep the shared tokenizer's escaping rules. Full DOS path syntax, batch files, executables, real filesystem access, and DOS emulation are outside this example.

Print useful help when a player types an incomplete command. Provide file and directory completion, ghost text, and command highlighting in the phosphor theme. Let commands alter the scene and shader through the same typed system integration as other examples.

## README recording

Capture the completed browser example at a fixed camera and viewport. The recording must show a brief desk view, focus on the monitor, typed text, Tab acceptance, contextual help, `DIR`, `TYPE`, a shader change, and a return to the wider scene. Use real keyboard/pointer interaction, not a substituted animation or pre-rendered console video.

Create a looping GIF around 10 to 15 seconds long at roughly 800 pixels wide and 15 to 20 frames per second. These are recording targets, not guarantees. Inspect palette artifacts and small text. Optimize the GIF toward 8 MiB or less while preserving readable text. Keep a static poster and a higher-quality WebM or MP4 link when useful.

Save a reproducible capture script, its seed and scene settings, and the final media under `docs/media/`. Add an actual GIF image to the README with descriptive alt text and a direct link to the live example. Do not place a fake placeholder GIF in the current planning README.

## Completion evidence

Build and run this example independently in native Bevy and browser WebGL2. Validate shader-enabled and shader-disabled rendering, both camera views, keyboard editing, pointer selection, ghost alignment, help, all DOS-inspired commands, shader controls, focus loss, viewport resizing, and reduced effects.

Capture desktop, Steam Deck, and narrow-view screenshots. Review the scene's materials, lighting, typography, contrast, and interaction in one batch. Fix observed defects and confirm them in one bounded repeat. Run the deployed example through the same interaction sequence before calling online delivery complete.
