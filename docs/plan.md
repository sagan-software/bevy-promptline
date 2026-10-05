# Implementation plan

Implementation begins only after the user reviews this plan and requests the next step. This repository initialization does not authorize a plugin implementation, external repository creation, push, publication, or deployment.

## Recommendation

Build a small command runtime and a default Bevy UI console. Keep Clap and logs optional. Make completion, ghost text, live help, and highlighting share one input analysis model. Prove customization through six independent examples.

Start on Bevy 0.19. Port to 0.18 after the editor behavior passes, then validate a pinned 0.20 preview. Maintaining three Bevy lines costs repeated UI, input, and dependency validation. Keep version adapters private and avoid a multi-version Cargo feature scheme.

The main risks are text editing on 0.18, styled input and ghost alignment on 0.19, the upcoming 0.20 API, and log subscriber installation order. Resolve them before adding all visual variants.

## Requirement checklist

- R1: A reusable plugin integrates into a small Bevy app with ordinary systems and documented traits.
- R2: Default and replacement consoles render entirely with Bevy UI.
- R3: Commands, subcommands, options, and supplied values complete with Tab and reverse with Shift+Tab.
- R4: Ghost text previews a completion without affecting input, history, copying, or submission.
- R5: Contextual help updates while typing without interrupting input or moving its region.
- R6: Input tokens and output severity have customizable highlighting with readable non-color diagnostics.
- R7: Existing Clap derive and builder commands parse through an optional adapter with nested command behavior intact.
- R8: Default consumers have no Clap dependency; logs are also optional.
- R9: Typed command delivery and lifecycle notifications distinguish submission, rejection, execution, and completion.
- R10: Layout, theme, bindings, motion, and window interaction use public customization contracts.
- R11: Bounded output accepts game writes and optional Bevy logs without recursive capture or input stalls.
- R12: Matching releases support Bevy 0.18 and 0.19; 0.20 remains preview until release validation.
- R13: Six examples compile independently to Wasm and have dedicated gallery pages.
- R14: The README links alternatives; research distinguishes observed compatibility from tested compatibility.
- R15: Quake-style sliding and Half-Life-style movable examples prove presentation flexibility.

## Milestone 0: Review and resolve contracts

Review the name, feature boundaries, command grammar, lifecycle ordering, Clap limits, example list, and compatibility policy. Confirm the default dependency budget and the no-shell policy.

Refresh official release sources. Record versions, MSRV, renderer features, and compatible editing dependencies. Inspect actual upstream APIs before fixing public signatures. Review the next milestone's acceptance cases before production code.

Deliver an updated plan with any changed release facts. A choice is implementation-ready only when its required source evidence and observable behavior are explicit.

## Milestone 1: Convert the template and prove external API shape

Make a runnable minimal guide and public integration tests first. Run the smallest focused test and record failure caused by the missing Promptline API. Then replace the template game package and remove unrelated gameplay dependencies.

Create a narrow module tree, documented skeletal types, and crate-root re-exports. Keep command metadata, parsing, dispatch, and UI construction separately testable. Document private items as well as public items. Update Cargo and Nix names together and regenerate the lockfile.

Implement registration, one-command tokenization, typed system delivery, direct output, utility help, and lifecycle status. Run focused external tests after each coherent slice. Add a separate internal test module after the public behavior passes.

Exit evidence covers R1, R8, and the initial R9 contract. Confirm a tiny no-Clap consumer and inspect its dependency graph. Commit this coherent milestone under the configured user identity.

## Milestone 2: Resolve text input and analysis

Prototype the upstream 0.19 editor integration with real cursor, selection, IME, and horizontal scrolling. Use Bevy UI exclusively. Test token styling and ghost alignment before treating the prototype as the final editor.

Implement shared analysis, completion replacement ranges, live help, syntax roles, candidate navigation, history, and focus gating. Keep work proportional to changed input and bounded candidate sets. Provide the minimal example with visible scene effects.

Exit evidence covers R2 through R6. If upstream editing prevents a required behavior, record the exact limitation and choose the smallest Bevy UI adaptation. Do not hide a failed editor constraint with a screenshot-only workaround.

## Milestone 3: Add optional Clap integration

Implement derive registration through standard Clap traits and builder registration through `ArgMatches`. Reuse the existing command registry rather than introducing a second command runtime.

Run conformance fixtures for subcommands, aliases, help/version, conflicts, defaults, custom value parsers, variadic values, empty values, negative values, `--`, multicall, and external subcommands. Test unknown and missing arguments independently. Keep parser errors and game-handler errors distinct.

Verify no handler runs on help, version, or invalid input. A validator with a counter must remain untouched during live metadata analysis. Document unsupported host-dependent inputs and the completion-provider extension.

Exit evidence covers R7 and the no-Clap half of R8. The `clap_commands` example must run in the browser as well as native builds.

## Milestone 4: Add bounded output and log capture

Implement bounded scrollback, visible-row rendering, copy, filters, pause display, stable scroll position, and unread counts. Add the optional tracing layer and subscriber-composition guide.

Test log installation before and after subscriber initialization, caller-owned subscribers, multiple Apps, burst overflow, concurrent writers, disconnects, recursive output, and drop summaries. Native and browser examples must preserve their ordinary log sinks.

Implement pending command completion and cooperative cancellation only to the extent required by the kitchen-sink example. Prove queued commands and UI input remain responsive while a command is pending.

Exit evidence covers R11 and completes R9. Record observed resource costs without claiming allocation or frame-time guarantees that were not measured.

## Milestone 5: Prove customization with six examples

Expose UI roles and theme resources. Build the drawer, movable window, compact prompt, and kitchen-sink configurations through those public contracts. Share scene helpers and keep each Cargo example independent.

Test interrupted animations, reduced motion, window dragging and resizing, focus restoration, touch open controls, small-canvas clamping, and gameplay input gating. Review desktop and narrow views in a bounded pass and confirm fixes.

Exit evidence covers R10, R13, and R15. No example may use a private library field to achieve its customization.

## Milestone 6: Port and validate the compatibility window

Create the 0.18 release branch and resolve its editor adapter. Run the same behavioral acceptance suite rather than only compilation. Prepare the isolated 0.20 preview at a recorded revision and identify every changed API.

Run feature and target matrices on each line. Check minimal dependencies, native platforms, all six Wasm outputs, and browser input. Keep stable failures blocking; label preview failures explicitly.

Exit evidence covers R12 for released versions only. Future release compatibility remains pending until its actual release is tested.

## Milestone 7: Build the gallery and release documentation

Create the static examples index and run pages from the example inventory. Add actual screenshots, source links, loading/error states, controls, and subpath-safe asset URLs. Test every direct example URL.

Write the README guide from the compiled minimal example. Add customization, Clap, log capture, input policy, performance limits, supported versions, and alternatives documentation. Finish with a separate prose-only edit after code gates pass. Run examples and doctests last.

Prepare local release artifacts and a publication checklist. GitHub repository creation, Pages deployment, and crates.io publication remain separate authorized actions.

## Focused acceptance cases

For registration, test duplicate command names, alias collisions, invalid identities, and successful independent registrations. Test authorization on both UI and programmatic submission. Rejection must reach neither dispatch nor a game side effect.

For tokenization, test empty input, whitespace, empty quoted arguments, Unicode, escaped spaces, quotes, backslashes, `--`, a negative number, unmatched quotes, trailing escapes, NUL, newlines, and exact replacement round trips. Assert original input preservation and decoded token ranges.

For completion, test no candidates, one candidate, a longer common prefix, candidate cycling, reverse cycling, middle-of-token replacement, quoted values, hidden commands, caret changes, and registry changes. If asynchronous providers are added, stall request A, start B, resolve B, resolve A last, and assert B remains visible.

For editing, test IME Enter and Tab, grapheme deletion, emoji sequences, selection, undo/redo, history draft restoration, opening without a stray toggle character, focus loss, unavailable clipboard, and multiline paste rejection before draft mutation.

For notifications, submit several commands in one frame and assert order, one execution per acceptance, zero execution per rejection, and one terminal status per invocation. Assert help/version produces informational output without dispatch.

For logs and output, test exact configured entry and byte limits, overlong messages, overflow count, drain budgets, scrolling during arrival, filtering, close/reopen, and layer disconnect. Stall a pending command and prove another command and input analysis progress before it finishes.

For customization, build a replacement UI with only public contracts. Test role removal, UI cleanup, custom font metrics, wrapping, viewport resize, interrupted animation, and a dragged window outside its title bar.

For browsers, test canvas focus, Tab capture only while focused, direct subpath navigation, missing assets, unsupported graphics, a failed Wasm fetch, log rendering, and scene changes after command execution. Compilation alone is not a browser interaction result.

## Required validation during implementation

Use the converted repository's Nix entry points and run exact named gates. At minimum run `nix flake check`, `cargo fmt --all -- --check`, `cargo check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, personal lints, and `cargo test --doc` through the project environment when required.

Run the supported feature combinations separately and build each of the six examples for `wasm32-unknown-unknown`. Run the configured coverage command, initially `nix run .#coverage` if retained during conversion. Inspect changed production branches and cover every reachable error, default, conversion, and exhaustion path. Record exact gaps and reasons.

Do not make incompatible renderer or tooling features part of an all-features build. If upstream features cannot coexist, document the exact incompatible combination and adjust the crate's supported feature design before claiming the named gate passes.

Use pure Rust tests, focused Worlds, and small headless Apps before window tests. UI layout and input require bounded rendered checks in addition to deterministic tests. Aim for 100% changed-path coverage. Coverage records execution; behavioral assertions establish correctness.

Before each commit, inspect the scoped staged diff, configured identity, message, and trailers. Commit each validated milestone. Publish no changes until remote writes are authorized.

## Planning handoff boundary

Initialization preserves the template and adds this planning packet. No proposed command API, plugin, example, gallery, compatibility branch, or CI workflow has been implemented. Review the contracts before starting milestone 1.
