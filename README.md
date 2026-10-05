# Bevy Promptline

A planned Bevy UI console for custom game commands, ghost completion, contextual help, syntax highlighting, and in-game logs.

Status: research and planning only, prepared October 5, 2026. No Promptline plugin, command API, or web demo exists yet. The Rust and Nix files are the unchanged `template-bevy` baseline.

The working repository name is `bevy-promptline`; the proposed crate name is `bevy_promptline`. Name searches found no exact match and the crates.io and organization repository endpoints returned 404. This does not reserve either name or establish trademark clearance.

## Review the plan

- [Implementation plan and acceptance gates](docs/plan.md)
- [Proposed architecture and behavioral contracts](docs/architecture.md)
- [Console interaction and six browser examples](docs/examples.md)
- [Bevy compatibility and release policy](docs/compatibility.md)
- [Research, game references, and project inventory](docs/research.md)
- [Template provenance and conversion boundary](docs/template.md)
- [Product requirements](PRODUCT.md)

## Planned behavior

Register game commands through traits and Bevy systems. Use existing Clap derive types or builder commands through an optional adapter. Keep the default dependency graph free of Clap.

Tab completes commands, subcommands, flags, and supplied argument values. Ghost text previews a completion without changing input. A separate help area explains the selected command or current argument while typing. The editor and output use Bevy UI text and nodes.

Applications can replace the layout, theme, input bindings, and animation. The default console provides a usable starting point. Examples demonstrate a top drawer, a movable window, a compact prompt, and more advanced command integration.

The target Bevy lines are 0.18, 0.19, and the upcoming 0.20. The 0.20 target remains provisional until its release passes validation. Compatibility requires matching crate releases, as explained in [the compatibility policy](docs/compatibility.md).

## Alternatives

These projects may already fit a game's needs. Versions below are observations, not compatibility test results.

- [bevy_console](https://github.com/makspll/bevy-console): an egui console with Clap, completion, history, themes, and log capture. Published 0.18.0 targets Bevy 0.19.
- [chill_bevy_console](https://github.com/Chillgames-net/bevy_console): a Bevy UI console with command metadata and dynamic completion. Published 0.3.4 targets Bevy 0.19; its documented 0.1 line targets 0.18.
- [bevy_minibuffer](https://github.com/shanecelis/bevy_minibuffer): Bevy UI prompts, command systems, key sequences, and completion. Version 0.7.0 targets Bevy 0.19.
- [slowchop_console](https://github.com/slowchop/console): a Bevy UI Quake-style console with logs and typed action events. Its repository targets Bevy 0.15.
- [bevy_dev_console](https://github.com/doonv/bevy_dev_console): an egui console with an optional scripting parser and log inspection, targeting Bevy 0.14 in the inspected manifest.
- [bevy_repl](https://github.com/philiplinden/bevy_repl): a terminal REPL using Clap and Bevy systems. Published 0.4.1 and repository 0.5.0 are different snapshots.
- [bevy_commands](https://github.com/aecsocket/bevy-commands): command dispatch with Clap and optional egui or terminal integration, targeting Bevy 0.11.
- [bevy_mod_debug_console](https://github.com/hymm/bevy_mod_debug_console), [bevy_mod_console_debug](https://github.com/hymm/bevy_mod_console_debug), and [bevy_clap_console](https://github.com/blinfoldking/bevy_clap_console): older ECS debugging and console projects.

The [full inventory](docs/research.md) includes small prototypes, copies of existing projects, console variables, remote consoles, and terminal renderers. Search cannot establish an exhaustive list of all public or private projects.

## Local baseline

This repository starts from [Sagan-Software/template-bevy](https://github.com/sagan-software/template-bevy) at `da85b65ffbad26f78d23c953d379a5b0d624e1ed`. Its original guide is preserved in [docs/template-readme.md](docs/template-readme.md).

Initialization uses fresh local Git history. There is no remote, publication, or deployment. Implementation requires a separate user instruction after plan review.

## License

The copied template is MIT licensed. Its [LICENSE](LICENSE) and attribution remain unchanged. The proposed plugin license is MIT, subject to review before publication.
