# Bevy Promptline

A Bevy UI command console with typed command delivery, completion, ghost text,
contextual help, syntax highlighting, history, pending commands, and optional
Clap integration and log capture.

Work in progress. The repository contains the plugin, regression tests, and six
independent example targets. Release validation, compatibility ports, README
media, and gallery deployment remain unfinished. See [the handoff](docs/handoff.md)
and [validation evidence](docs/validation.md) before continuing.

## Run locally

```sh
nix develop --command cargo run --example minimal
nix develop --command cargo run --example kitchen_sink --features clap,log
nix develop --command cargo run --example crt_workstation --features crt-example
```

The host owns its window, renderer, and camera. Start with
[the minimal example](examples/minimal.rs) and its
[command registrations](examples/support/playground.rs). Clap and log capture
are optional features; the default dependency graph contains no Clap.

## Bevy compatibility

| `bevy_promptline` | Bevy | Declared Rust minimum |
| --- | --- | --- |
| 0.3 (`main`) | 0.20 | 1.97.1 |
| 0.2 (commit `2a3ad35`) | 0.19 | 1.96.1 |

The checkout pins Bevy 0.20.0 and Rust 1.98.1. Bevy 0.20.0 declares Rust
1.97.1 as its minimum; that minimum has not been validated here. Bevy 0.18
compatibility remains unfinished.

## Continue implementation

- [Handoff and remaining work](docs/handoff.md)
- [Implementation plan and acceptance gates](docs/plan.md)
- [Architecture and behavioral contracts](docs/architecture.md)
- [Six examples and gallery](docs/examples.md)
- [Compatibility policy](docs/compatibility.md)
- [Log filtering and formatting](docs/logging.md)
- [Highlighting](docs/highlighting.md)
- [Scrollback](docs/scrollback.md)
- [Pending commands](docs/pending.md)
- [CRT workstation and recording](docs/crt-workstation.md)
- [Implementation-session prompt](docs/implementation-prompt.md)

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

### Other consoles and references

- [Seismon](https://github.com/eira-fransham/seismon): a Quake engine with a Bevy UI console and system-backed commands. Its inspected main branch uses Bevy 0.18; extraction into a standalone console crate remains tracked separately.
- [bevy_tui_texture](https://github.com/tt-toe/bevy_tui_texture): Ratatui-based UI textures with an existing CRT workstation and WebGL2 example. Useful presentation reference; its rendering differs from the Bevy UI requirement.
- [bevy_retro_shaders](https://github.com/kodaskills/bevy_retro_shaders): a CRT effect reference whose documented pass precedes ordinary UI rendering.
- [DudeGunter/bevy_console](https://github.com/DudeGunter/bevy_console), [rks-malvern/bevy_console](https://github.com/rks-malvern/bevy_console), and [bevy_dev_console_ui](https://github.com/mathiasmellemstuen/bevy_dev_console_ui): smaller console projects with varying documentation and compatibility.
- [sgodwincs/bevy-console](https://github.com/sgodwincs/bevy-console) and [Hellzbellz123/bevy_console](https://github.com/Hellzbellz123/bevy_console): related copies of the established console.
- [bevy_rcon](https://github.com/BrianWiz/bevy_rcon): remote console access. [bevy-convars](https://github.com/transcendental-experiences/bevy-convars), [bevy_cvar](https://crates.io/crates/bevy_cvar), and [cvars](https://github.com/martin-t/cvars): configuration-variable tools.
- [bevy_tui](https://github.com/sstelfox/bevy_tui), [bevy_ratatui](https://github.com/apekros/bevy_ratatui), [bevy_terminal_display](https://docs.rs/bevy_terminal_display/latest/bevy_terminal_display/), [plurimus](https://github.com/kenianbei/plurimus), and [bevy_ascii_terminal](https://github.com/sarkahn/bevy_ascii_terminal): adjacent terminal integrations and renderers.
- [symtropy-devconsole](https://crates.io/crates/symtropy-devconsole), [bevy_workbench](https://github.com/Bli-AIk/bevy_workbench), and [BevyStudio](https://github.com/salom600/BevyStudio): project-specific developer panels and editor tools.

## Local baseline

This repository starts from [Sagan-Software/template-bevy](https://github.com/sagan-software/template-bevy) at `da85b65ffbad26f78d23c953d379a5b0d624e1ed`. Its original guide is preserved in [docs/template-readme.md](docs/template-readme.md).

Initialization uses fresh local Git history. There is no remote, publication, or deployment. Implementation requires a separate user instruction after plan review.

## License

The copied template is MIT licensed. Its [LICENSE](LICENSE) and attribution remain unchanged. The proposed plugin license is MIT, subject to review before publication.
