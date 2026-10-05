# Research and alternatives

Research date: October 5, 2026. Primary sources include project repositories, crates.io metadata, tagged Bevy source, Clap documentation, id Software source, Valve source, and Epic documentation. No alternative was compiled or interactively tested.

## Search scope and limits

Search used web queries for Bevy consoles, terminals, command dispatch, and autocomplete; crates.io's `bevy console` search; and GitHub repository search for `bevy console`. Follow-up reads inspected the discovered repositories' manifests and READMEs. Smaller source-only projects remain candidates, not recommended dependencies.

The inventory covers the discovered relevant projects and distinguishes adjacent tools. It cannot prove that every public, renamed, unindexed, private, or embedded game console has been found. Re-run these searches before publication and accept additions through documentation updates.

The machine-readable [observations](research-observations.json) preserve repository identities, observed versions, Bevy dependencies, search endpoints, and fetch failures. Upstream READMEs and game source were read for research, not vendored into this repository.

## Closest alternatives

[bevy_console](https://github.com/makspll/bevy-console) is the established Half-Life-inspired option. Its observed 0.18.0 manifest uses Bevy 0.19, `bevy_egui`, and Clap. It provides completion, history, color output, themes, bindings, and log capture. Registry metadata still links the older RichoDemus URL, which resolves to makspll. Companion crates include `bevy_console_derive` and the older `bevy_console_parser`. Its default egui rendering differs from the requested Bevy UI constraint.

[chill_bevy_console](https://github.com/Chillgames-net/bevy_console) is the closest native-UI reference. Observed 0.3.4 uses Bevy 0.19 and UI widgets. Its documentation includes aliases, argument metadata, structured help, dynamic completions, optional embedded fonts, history persistence, and resource properties. `chill_bevy_console_derive` is a companion crate. Its README documents earlier Bevy 0.18 support. Investigate its editing, metadata, and customization behavior before claiming Promptline fills a gap it already covers.

[bevy_minibuffer](https://github.com/shanecelis/bevy_minibuffer) 0.7.0 uses Bevy 0.19 UI and provides text-editor-inspired prompts, system-backed commands, key sequences, completion, and optional asynchronous commands. It integrates `bevy_asky`. `bevy_minibuffer_inspector` connects to the egui inspector. Its prompt model may fit games that need a compact command interface rather than a full scrollback console.

[slowchop_console](https://github.com/slowchop/console) demonstrates a Quake-style Bevy UI console, log capture, typed enum actions, and emitted events. crates.io reports 0.1.3 while the inspected package manifest is 0.2.0; its workspace uses Bevy 0.15. Its author describes limited testing. `slowchop_console_derive` supplies action derivation. Treat this as an architectural reference, not current-version compatibility evidence.

[bevy_dev_console](https://github.com/doonv/bevy_dev_console) uses egui and Bevy 0.14 in the inspected manifest. Its optional parser supports expressions, variables, native functions, and resource inspection. The README describes Git-based installation, while the crates.io 0.0.0 entry reserves the name. A scripting language is outside Promptline's initial scope.

## Other console and command projects

- [bevy_commands](https://github.com/aecsocket/bevy-commands): Bevy 0.11, Clap 4, optional egui and rustyline integration, and a derive companion. The attempted `README.md` URL returned 404; classification comes from the manifest and repository description.
- [bevy_mod_debug_console](https://github.com/hymm/bevy_mod_debug_console): stdin/stdout ECS debugging and an egui-console integration example. The manifest targets Bevy 0.8 while the README retains older usage. Do not infer compatibility from that usage snippet.
- [bevy_mod_console_debug](https://github.com/hymm/bevy_mod_console_debug): graphical ECS console utility with Bevy 0.11. Its README gives little behavior detail.
- [bevy_clap_console](https://github.com/blinfoldking/bevy_clap_console): Bevy 0.9, egui, Clap 4, and a derive companion. The attempted README URL returned 404.
- [DudeGunter/bevy_console](https://github.com/DudeGunter/bevy_console): Bevy 0.19 with Bevy UI/Feathers and custom text input. The author describes it as a personal project with undocumented quirks.
- [rks-malvern/bevy_console](https://github.com/rks-malvern/bevy_console): a text-console prototype. No root Cargo manifest was available in the inspected path, so its Bevy compatibility is unknown.
- [bevy_dev_console_ui](https://github.com/mathiasmellemstuen/bevy_dev_console_ui): a small Bevy 0.13/egui console UI project with minimal README detail.
- [sgodwincs/bevy-console](https://github.com/sgodwincs/bevy-console): a repository carrying the established console's README and package identity, observed at 0.17.1 for Bevy 0.18. Treat it as a related copy, not an independently established design.
- [Hellzbellz123/bevy_console](https://github.com/Hellzbellz123/bevy_console): a described fork/copy of the established console, observed at 0.12.1 for Bevy 0.14. Do not count it as a distinct solution to the native-UI requirement.

## Adjacent tools

[bevy_repl](https://github.com/philiplinden/bevy_repl) runs an external-terminal prompt with Clap, system execution, and log integration. Published 0.4.1 differs from repository 0.5.0, whose manifest targets Bevy 0.19. It is useful for headless games and servers, but does not provide the requested on-screen Bevy UI.

[bevy_rcon](https://github.com/BrianWiz/bevy_rcon) describes remote game-server console access and targets Bevy 0.15.2 in the inspected manifest. Remote control needs a separate authorization and transport design; it is outside the initial plugin.

[bevy-convars](https://github.com/transcendental-experiences/bevy-convars) and [bevy_cvar](https://crates.io/crates/bevy_cvar) expose runtime configuration concepts. The discovered [cvars](https://github.com/martin-t/cvars) is engine-independent. These are candidates for optional future command integration, not console UIs. Their versions and compatibility need direct verification before selection.

[bevy_tui](https://github.com/sstelfox/bevy_tui), [bevy_ratatui](https://github.com/apekros/bevy_ratatui), [bevy_terminal_display](https://docs.rs/bevy_terminal_display/latest/bevy_terminal_display/), and [plurimus](https://github.com/kenianbei/plurimus) render or integrate operating-system terminals. [bevy_ascii_terminal](https://github.com/sarkahn/bevy_ascii_terminal) is a terminal-cell rendering candidate. [bevy_tui_texture](https://github.com/tt-toe/bevy_tui_texture) renders terminal UIs to textures. They solve related presentation problems rather than supplying the requested command console entirely through Bevy UI nodes and text.

The registry search also found [symtropy-devconsole](https://crates.io/crates/symtropy-devconsole), a project-specific inspector/control panel, and [bevy_workbench](https://github.com/Bli-AIk/bevy_workbench), an editor scaffold. GitHub search found [BevyStudio](https://github.com/salom600/BevyStudio), which includes an editor console, and [bevy-tracing-demo](https://github.com/chriamue/bevy-tracing-demo), a tracing demonstration. These are adjacent results whose full behavior was not audited.

Editing candidates include [bevy_text_edit](https://gitlab.com/245project/bevy-plugin/bevy-text-edit) and [bevy_cosmic_edit](https://github.com/StaffEngineer/velo/tree/main/crates/bevy_cosmic_edit). These require release-specific research before use, particularly on 0.18. Bevy 0.19's upstream text input is the first candidate for the current line.

## Lessons from game consoles

[Quake's console source](https://github.com/id-Software/Quake/blob/master/WinQuake/console.c) separates console visibility, scrollback, printed output, and game input destination. Its [screen source](https://github.com/id-Software/Quake/blob/master/WinQuake/gl_screen.c) controls the console's opening movement. Promptline's drawer example should borrow the interaction idea while leaving animation outside command execution. No Quake code or assets should be copied.

[Valve's console dialog source](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/vgui2/vgui_controls/consoledialog.cpp) includes a frame, completion entries, command history, and input handling. [Its game console panel](https://github.com/ValveSoftware/source-sdk-2013/blob/master/src/game/client/vgui_consolepanel.cpp) is a separate integration boundary. This supports a movable-window reference for Half-Life/Source-era interaction. This is SDK evidence, not a direct interactive audit of Half-Life or Counter-Strike 2.

[Steam Support](https://help.steampowered.com/en/faqs/view/4700-D10E-26BE-DDDD) documents enabling the developer console in Counter-Strike 2 and identifies tilde as the English/US keyboard shortcut. It also documents different activation paths for other Valve games. Promptline should expose a visible open control and rebinding. Exact Counter-Strike 2 completion behavior remains unverified by an interactive test; the inspected SDK supports the historical comparison.

[Epic's Console Manager documentation](https://dev.epicgames.com/documentation/unreal-engine/console-variables-cplusplus-in-unreal-engine) describes registration-based enumeration, completion, and help. Its [console settings](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/EngineSettings/UConsoleSettings) include command/variable colors, descriptions, and bounded scrollback. Promptline should derive help and candidates from registration metadata, keeping parser-specific validation separate. Do not imply that a player-facing Unreal console is enabled in every shipped Unreal game.

Ghost text is a requested Promptline behavior. This research does not establish it as a stock feature of the historical game consoles.

## Bevy and Clap findings

[The current Bevy release](https://github.com/bevyengine/bevy/releases/tag/v0.19.1) and [main manifest](https://github.com/bevyengine/bevy/blob/85cf8037abde4868f406c54d1a8fba9c29c53489/Cargo.toml) establish the compatibility targets. [The tagged text-input example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/text/text_input.rs) shows the upstream 0.19 editing integration. Exact composition, styling, and ghost behavior still need a prototype.

[Bevy's LogPlugin source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_log/src/lib.rs) exposes custom layer composition and process-global subscriber initialization. Plan log capture at startup, preserve the ordinary sink, and test already-initialized subscribers. Console filters cannot recover logs discarded by earlier filters.

[Clap's Command API](https://docs.rs/clap/4.6.6/clap/struct.Command.html) supports fallible parsing and metadata inspection. [CommandFactory](https://docs.rs/clap/4.6.6/clap/trait.CommandFactory.html) and [FromArgMatches](https://docs.rs/clap/4.6.6/clap/trait.FromArgMatches.html) permit existing derive-based command types. [clap_complete](https://docs.rs/clap_complete/latest/clap_complete/) marks dynamic completion APIs unstable. Prefer stable metadata and an application completion trait for the first adapter.

[Bevy's examples index](https://bevy.org/examples/) and [logs run page](https://bevy.org/examples/application/logs/) supply the gallery reference: separate runnable examples, descriptions, controls, source access, and renderer-specific pages. Follow [the tagged Wasm guide](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/README.md#wasm) for independent builds and asset copying. Do not reuse the template's native networking demo as browser evidence.

## Follow-up research

The user chose an independent implementation for control over visual design and interaction. Contribution to an existing console is not a prerequisite. The alternatives remain documented without making quality claims from screenshots.

[Seismon](https://github.com/eira-fransham/seismon) is another relevant console implementation inside a Bevy Quake engine. Its inspected main revision `fce4b4850608fbbb4afe871c70be4b2be0ffa7f2` uses Bevy 0.18. The project describes its console as Bevy UI with ordinary systems for commands. Its [extraction issue](https://github.com/eira-fransham/seismon/issues/30) still lists console extraction. Treat it as embedded reference code, not a verified current standalone console crate.

[bevy_tui_texture](https://github.com/tt-toe/bevy_tui_texture) deserves a closer look for the new example. Inspected revision `ad76f318f13891b9087ce1d8bf11fe49e5a667b6` identifies package 0.3.5 for Bevy 0.19. It already includes a CRT scene, input mapping, and a WebGL2 build. It uses Ratatui/WGPU textures, so it is an adjacent implementation rather than the requested Bevy UI console. Its fonts have separate OFL licenses. Use original scene assets and preserve licenses for any adopted assets.

[bevy_retro_shaders](https://github.com/kodaskills/bevy_retro_shaders) at `2982443eea6a9cd8175c9c85388c85965b8af498` identifies package 0.2.0 for Bevy 0.19. Its README documents CRT effects before ordinary UI. Study its effect controls, but prove that Promptline's shader samples the actual console texture.

The follow-up crates.io console search found the same direct console families. A broader command search also returned ECS command helpers such as `bevy_local_commands` and `bevy_commandify`; their names do not establish an interactive console. The broad GitHub README search produced many unrelated collection repositories. Those results are search noise rather than additional console evidence. The inventory is bounded, not exhaustive.

[Bevy's UI-to-texture example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/render_ui_to_texture.rs) includes an offscreen UI camera and virtual pointer routing. [Its UI material example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/ui_material.rs) demonstrates a custom fragment shader. These support the proposed CRT architecture; they do not prove the planned distorted screen interaction.

## Recommendation and unresolved evidence

Build Promptline independently as requested. Use the existing consoles as functional references. Establish visual quality through actual rendered examples and interaction checks. Bevy UI alone is not a unique claim; close alternatives already use it.

Before implementation, verify the 0.18 editing strategy and prototype the current editor's styled input. Before publication, refresh the inventory, validate crate-name availability, test the actual compatibility lines, and inspect real browser examples. No current source proves the planned implementation works.
