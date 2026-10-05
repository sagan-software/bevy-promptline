# Compatibility and dependency policy

Research date: October 5, 2026. These are intended targets; no Promptline build has validated them.

## Observed Bevy versions

[Bevy's latest release](https://github.com/bevyengine/bevy/releases/tag/v0.19.1) is 0.19.1, published August 13, 2026. The [0.18.1 manifest](https://github.com/bevyengine/bevy/blob/v0.18.1/Cargo.toml) supplies the previous-line baseline. Bevy main identifies itself as `0.20.0-dev`; research inspected revision `85cf8037abde4868f406c54d1a8fba9c29c53489`.

The requested window therefore means 0.18, 0.19, and upcoming 0.20. Recheck the release source at implementation kickoff. If the release window has moved, bring the changed targets to plan review before replacing them.

## Release structure

Use matching release lines for the same `bevy_promptline` crate. Proposed initial mapping: Promptline 0.1.x for Bevy 0.18, 0.2.x for Bevy 0.19, and 0.3.x for Bevy 0.20 after its release. Reserve neither crate versions nor compatibility claims during planning.

Maintain `bevy-0.18` and `bevy-0.19` branches after the initial port. Keep development on the newest supported line. Use an isolated preview branch and lockfile for Bevy 0.20. Cherry-pick shared fixes and verify each branch separately. Document minimum tested patch versions and MSRV in every release.

Do not offer additive `bevy_018` and `bevy_019` Cargo features. Their ECS and UI types differ, and Cargo can unify requested features. One compiled plugin integrates with one Bevy minor version. A game selects the matching Promptline release.

"Compatible with the next version" cannot be guaranteed before that version exists. A revision-tested development build is provisional. Promote it only after testing the actual release. Do not follow Bevy main without a recorded revision.

## Version-sensitive boundaries

Keep editing, focus dispatch, UI text layout, and notification wrappers behind private modules. Public command metadata and completion behavior should remain consistent across branches. Bevy integration types may require a major plugin release change during a port.

[Bevy's 0.19 release notes](https://bevy.org/news/bevy-0-19/) introduce text input. Inspect the [tagged example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/text/text_input.rs) and widget source before choosing exact types. Use upstream editing on 0.19 where ghost rendering and highlighting can preserve cursor, selection, and IME behavior.

Bevy 0.18 needs a separate text-editing adapter. Investigate established Bevy UI text input crates and their compatible releases before writing an editor. Reject an adapter that cannot pass Unicode, composition, selection, and browser input gates. This is the highest compatibility risk.

Use [the 0.18 to 0.19 migration guide](https://bevy.org/learn/migration-guides/0-18-to-0-19/) when porting. Resource storage and text internals changed. Plan for 0.20 changes only after inspecting that revision's source and eventual migration guide.

## Dependency budget

The default library enables only Bevy app, ECS, input, UI, and required text integration. Avoid default Bevy features, renderer selection, window creation, 3D rendering, audio, physics, networking, and inspector dependencies in library dependencies.

Proposed optional features are `clap`, `log`, and `clipboard`. Keep persistence out of the initial default build. Let games select their renderer and platform features. Inspect the Cargo feature graph for both a tiny consumer and a feature-rich consumer.

Clap starts with stable 4.6.6 as used by the inspected template, subject to verification at implementation. Keep the adapter on stable Clap APIs. `clap_complete` exposes dynamic completion behind `unstable-dynamic`; it is not a mandatory dependency or promised stable API.

Browser examples use target-specific dependencies. Native clipboard helpers, filesystem persistence, threads, and native inspection tools must not enter the browser default graph.

## CI matrix

Each supported stable branch runs formatting, tests, strict Clippy, documentation tests, changed-path coverage, and personal lints. Run checks for default features, no default features, Clap only, logs only, clipboard, and all supported combinations. Use compile-fail tests for important public misuse cases.

Build every example independently for `wasm32-unknown-unknown`. Run browser interaction tests on WebGL2 in Chromium and Firefox. Keep a separately built WebGPU lane with an explicit supported browser. Validate native behavior on Linux, Windows, and macOS; test text composition and clipboard where automation permits.

Run the 0.20 pinned preview matrix on a schedule and on deliberate revision updates. Record failures as preview blockers. Supported stable releases must remain blocking CI gates.
