# Validation evidence

Goal: implement the plugin and six independent native/Wasm examples, publish the gallery, and verify README media and every deployed example. Completion remains unverified.

## Kickoff, October 5, 2026

The checkout was clean at `98fed30`. No remote was configured. The configured author and committer identity is Liam Curry <liam@sagan.software>. Existing history and template attribution remain authoritative. GitHub reports no existing `Sagan-Software/bevy-promptline` repository. No remote has been created yet.

Official release refresh confirmed Bevy 0.19.1, published August 13, 2026, and main revision `85cf8037abde4868f406c54d1a8fba9c29c53489` with version 0.20.0-dev and Rust minimum 1.96.0. The selected current stable target remains 0.19.1. Previous-line and preview validation remain pending.

Sources: [release API](https://api.github.com/repos/bevyengine/bevy/releases/latest), [preview revision](https://github.com/bevyengine/bevy/commit/85cf8037abde4868f406c54d1a8fba9c29c53489), [text input](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/text/text_input.rs), [UI texture](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/render_ui_to_texture.rs), [logging](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_log/src/lib.rs).

Upstream `EditableText` owns editing and composition. Its widget plugin routes focused keyboard, pointer, and IME events. The runtime will remain independent of windows and rendering. UI adapters must preserve the upstream editor rather than approximate text metrics.

## Required gates

- `nix flake check`
- `cargo fmt --all -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Personal lint dispatcher through the installed Nix-safe workflow
- `cargo test --doc`
- Supported feature combinations separately
- Six separate `wasm32-unknown-unknown` example builds
- Configured coverage command, `nix run .#coverage`, with changed production branch inspection
- Runtime, editing, completion, Clap, output, logging, policy, notification, and customization acceptance cases in `docs/plan.md`
- Native and browser behavior on each supported Bevy release branch; pinned preview separately
- Desktop, Steam Deck, and narrow rendered views
- Real CRT capture, inspected GIF and poster, reproducible script and provenance
- Deployed gallery, every direct example, project-subpath assets, fetch/error states, command effects, CRT input, README GIF
- Final staged diff, configured commit identity, message, and trailer audit

## Invariant scratchpad

Command identities validate at registration. Names and aliases share one collision domain. Policy filters discovery and validates every submission source.

Submission has one owner and progresses through submitted, rejected or accepted, running, and one terminal outcome. Accepted invocations preserve queue order. Parser and handler failures remain distinct.

Input text remains authoritative. Tokens retain decoded values and UTF-8 source ranges. Analysis accepts incomplete text; submission rejects malformed text. Ghost, help, and syntax roles derive from draft and metadata.

Limits use positive validated counts. History, pending invocations, retained output, capture queue, and per-frame drain remain bounded.

The console visibility has closed and open variants. Open focus has editing and released variants. Custom presentation derives animation from visibility.

Clap is optional and supplies final parsing. Live metadata analysis cannot run custom validators. Capture, display, and formatting settings remain separate.

## Verification status

Implementation, coverage, native execution, browser interaction, compatibility ports, README capture, and deployment have not passed yet. No compatibility or online-delivery claim is made.

## Public API checkpoint

Before production implementation, `rustc --edition=2024 --test tests/command_text.rs -o /tmp/promptline-red` failed with E0432 because `bevy_promptline` did not exist. [Captured diagnostic](evidence/api-red.txt). A runnable minimal typed-command guide and external runtime tests were also written before production behavior.

The converted module skeleton separates identities, tokenization, metadata, registry, execution, output, completion, and UI. Conversion removes the template game, physics, network, particle, and inspector code within the authorized scope. Existing commits and license remain intact.

## Dependency and editing observations

`cargo tree --edges normal --no-default-features --prefix none` found no Clap, `bevy_render`, `bevy_winit`, PBR, Avian, Lightyear, Hanabi, or inspector dependency in the library graph. The resolved optional Clap version is 4.6.7. The project environment loads Rust 1.98.1; the declared Bevy-compatible minimum is 1.96.0 and has not been tested yet.

Bevy 0.19.1 applies queued editor changes in `PostUpdate`, in `EditableTextSystems`. Its Parley 0.9 `PlainEditor` applies one style to the text. Token coloring therefore needs a Bevy UI overlay prototype with matching shaping and actual editor geometry. Styling and ghost alignment are not verified yet.

For 0.18, [bevy_ui_text_input](https://github.com/ickshonpe/bevy_ui_text_input) documents 0.7 compatibility but explicitly lacks IME and syntax highlighting. It does not satisfy the required editor profile. [bevy_text_edit](https://github.com/dothanhtrung/bevy-text-edit) documents 0.8 for Bevy 0.18 and IME/selection support. Its [current manifest](https://gitlab.com/245project/bevy-plugin/bevy-text-edit/-/raw/master/Cargo.toml) also introduces gamepad, state, picking, and optional native clipboard dependencies. No adapter has been selected or tested.

The Nix default shell retains AI link setup and native build libraries. Remote-inspection tooling remains an explicit package rather than a default-shell prerequisite for the console library.

## Runtime progress

Created [Sagan-Software/bevy-promptline](https://github.com/sagan-software/bevy-promptline) as a public repository and attached `origin` over HTTPS. No commit has been pushed and no gallery has been deployed.

The external runtime tests passed before adding the separate internal test module. The current focused suite passes 15 tests: five public text-profile cases, seven runtime cases, two output cases, and one internal incomplete-decoding case. [Captured results](evidence/runtime-tests.txt).

The behavioral tests cover ordered typed delivery, independent registrations, primary and alias collisions, malformed identities, parser rejection, policy rejection for editor and programmatic origins, unknown commands, malformed input, input length, frame acceptance overflow, terminal failure and cancellation, removed handler systems, and exact output entry/text limits. These are behavioral assertions; measured coverage remains pending.

Submission diagnostics initially exposed raw argument text. The regression failed before introducing a private text wrapper with redacted debug output, then passed. Draft analysis also started with a missing-public-function failure before implementation. [Draft API failure](evidence/draft-api-red.txt).

The exact gate `cargo clippy --all-targets --all-features -- -D warnings` passes through the Nix project environment. [Captured result](evidence/core-clippy.txt). This proves compilation and linting, not functionality of the still-unimplemented optional adapters.

The declared compiler minimum remains the template's 1.96.1, matching its Clippy configuration. It has not been tested. Current local validation uses Rust 1.98.1.

The personal lint pass identified two remaining API-policy diagnostics after genuine defects were repaired. `CommandName::as_str` remains available alongside its standard `AsRef<str>` implementation. `Output::push` consumes and retains its owned String without cloning it. The boundary lint source diagnoses all public String parameters without inspecting consumption, despite its stated exception. These diagnostics remain unresolved; no lint allowance or target configuration change was added.

Adjusted two links in the relocated template guide to refer to `../ai/AGENTS.md` and `../LICENSE`. The original guide remains recoverable from the baseline commits.

## Editor and presentation progress

Command-name completion now passes its first public behavior test. It reads permitted metadata without invoking a parser, keeps the draft unchanged while previewing, suppresses ghost text during selection or composition, and accepts an explicit token replacement. Argument completion, candidate cycling, syntax roles, and rendered alignment remain pending.

The Bevy UI tree exposes root, input, output, help, ghost, candidate, and open-control roles. Headless construction creates no camera or window. Four UI tests pass for construction, focus acquisition and restoration, repeated visibility requests, actual-editor-value submission, and preserving rejected input. Focus regressions were observed before repair. These tests do not establish rendered correctness.

Runtime dispatch now runs in PostUpdate after the UI analysis boundary, allowing submission to use Bevy's applied editor value in the same frame. Public schedule sets describe input, analysis, dispatch, output, and presentation ordering. The full test and lint suites must be rerun after these changes.

The latest recorded coverage run was 99.03% of aggregate source lines and 100% of functions. Its remaining aggregate misses include cfg(test) assertion branches and duplicate test/non-test function instances. This does not pass the configured 100% line gate. Coverage must be refreshed after editor changes, and production branches still require review.

The runtime guide executes natively and prints `hello`, as captured in [runtime-guide.txt](evidence/runtime-guide.txt). No graphical native example, Wasm example, browser interaction, shader, compatibility port, media capture, or deployment has been validated yet.

## Native graphical prototype

The independent `minimal` example now opens a Bevy window and draws a procedural rotating triangle over a floor. Real native input through CUA opened the console, typed `ec`, displayed an aligned `ho` ghost suffix, accepted `echo` with Tab, and executed it with Enter. `color red` changed the primitive to red. `spin 0` stopped its rotation. Escape closed the console.

A submitted `spin invalid` retained the input and produced a readable rejection diagnostic. Captured evidence: [ghost preview](evidence/native/minimal-ghost.png), [command effects](evidence/native/minimal-color-spin.png), and [retained rejection](evidence/native/minimal-rejection.png). These are real application screenshots, not generated mockups.

The app was launched through the Nix environment with display and session variables read from the running Plasma process. The GPU initialized as AMD RADV VANGOGH.

The rendered check found that help vanished during argument entry. The code now falls back to permitted active-command metadata; this repair still needs a rendered confirmation. Syntax highlighting, argument completion, cycling, history, customizable theme/bindings, clipboard/IME browser behavior, and the remaining examples remain pending.

A Wasm release build of minimal was started with `cargo build --example minimal --target wasm32-unknown-unknown --profile wasm-release`. Compilation alone will not establish browser behavior.

## Optional Clap and first browser proof

The optional `ClapAppExt` registers existing derive types through `CommandFactory` and `FromArgMatches`, or builder commands with `ArgMatches` handlers. Both use the existing registry and dispatch owner. Help and version produce accepted informational completion without a started-handler notification.

Seven focused Clap fixtures pass for nested typed delivery, negative numbers, help/version, empty quoted values, variadic values, `--`, defaults, conflicts, unknown/missing/invalid values, aliases, `no_binary_name`, multicall, external subcommands, nested metadata completion, and UTF-8 command labels. The multicall fixture follows Clap's documented main-applet configuration. [Results](evidence/clap-tests.txt).

The nested metadata fixture asserts `rotate`, `--axis`, and `x/y/z` candidates and argument help, while a custom validator counter stays at zero during analysis. Unicode and quoted-space labels start with a failing registration fixture before the adapter-specific identity boundary was added. Ordinary `CommandName::from_str` retains its ASCII identifier profile; Clap labels accept nonempty UTF-8 without NUL or line breaks and preserve their spelling.

The no-default-features library check passes after adding required UI picking features. This does not select a window backend, 3D renderer, or Clap for a tiny consumer. [Check](evidence/default-check.txt).

The minimal Wasm release build completed. Browser bindings use the official wasm-bindgen 0.2.129 CLI, matching Cargo.lock. The downloaded archive's SHA-256 matched the official release digest `82d12bb940e2d4e72e0d5605387fc1b8ca179044e012b620f0ce4e7440e8320e`.

A local HTTP preview runs at `http://127.0.0.1:8765/minimal/`. The in-app browser is unavailable, so validation used a new Firefox tab. Real browser input opened the console, typed `ec`, accepted the ghost with Tab, printed `browser proof`, changed the primitive to green, and set rotation to zero. [Screenshot](evidence/browser/minimal-firefox.png). This is local WebGL2 behavior, not a deployment result.

Current unresolved work includes completion cycling, broader arity/global-option completion, highlighting, history, clipboard/IME, log capture, pending/cancellation, presentation customization, five remaining independent examples, CRT mapping/shader, compatibility branches, media capture, CI, and deployment. All required final gates remain applicable.

## History, completion, and structured capture checkpoint

Accepted public commands now enter bounded history. Sensitive commands do not enter history. Up and Down recall commands and restore the saved draft. Manual edits start a new browsing session.

The first forward Tab expands a longer common prefix. Later Tab and Shift+Tab actions cycle the frozen candidates. Ghost text previews the next forward action. Invalid or missing input clears stale candidates, ghost text, and help. The regression first reproduced stale `echo` discovery after policy revocation.

The optional `log` feature now supplies a caller-installed layer, live layer-local filter reload, bounded capture, retained typed records, display selection, and custom formatting. Tests cover previously disabled callsites, another unchanged sink, upstream rejection, malformed updates, disconnected reload, typed and redacted fields, span updates, native thread context, and 128-bit values. A contended queue test proves that a writer finishes before the app releases its drain guard. A stalled burst proves overflow reporting, insertion order, and the 64-record drain budget.

The all-feature suite passes 52 tests. Strict `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all -- --check` pass. The minimal example passes `cargo check --features log,clap --example minimal --target wasm32-unknown-unknown --profile wasm-release`. These results are saved in `docs/evidence/history-log-*` and `wasm-adapters-check.txt`. This Wasm check does not establish browser logging behavior.

Personal lints still fail with four diagnostics. Repeated Clap feature gates require consolidation.

The other diagnostics target `CommandName::as_str`, `CommandHistory::next`, and owned `Output::push` text. `CommandName` already implements `AsRef<str>`. History navigation returns a value borrowed from the mutable history; standard `Iterator::Item` cannot borrow that particular `next` call. Output consumes, truncates, and stores the owned string, matching the lint's stated exception. Public signatures remain preserved. The exact audit is saved in `personal-audit-current.txt`; the gate has not passed.

The template's private-documentation lint changed from `forbid` to `deny` because Clap derive emits an internal `allow(clippy::restriction)`. Handwritten private documentation remains checked. Independent formatter switches retain all valid combinations; the narrow Clippy expectation records that reason.

Logging rows and controls still need UI integration. Custom themes, bindings, highlighting, completion providers, pending commands, remaining examples, compatibility branches, README capture, and deployment remain unfinished. Earlier native and Firefox screenshots demonstrate the earlier minimal build. They do not validate the current integrated feature set. No implementation commit or gallery deployment is claimed.

A wide-integer boundary regression first retained 8,200 field bytes against the 8,192-byte event budget. Capture now discards a value that cannot fit its complete typed width. An unrepresentable span update removes its obsolete prior value. The regression and final passing suite are saved in `log-budget-red.txt` and `history-log-full-tests.txt`.

## Plain registration checkpoint

`input::CommandBuilder` and `input::ParseDecision` now register author-supplied parsing through `PromptlineAppExt::register_builder`. This path needs no `ConsoleCommand` implementation. Parser-produced information prints and finishes without starting the handler. Ordinary parsing retains the existing identity, permission, and execution checks.

The public test first failed on missing API. A separate skeletal-API checkpoint compiled the intended type relationships and failed on missing construction. The implemented tests verify the `inc` alias, a positive increment of 9, help without mutation, zero and missing-argument rejection, denied parsing, atomic collision rejection, and preserved original dispatch. An internal test verifies release of an owned parser capture.

The all-feature suite now passes 55 tests. Strict Clippy passes. Evidence is saved in `command-builder-*`. A final doc-comment heading spacing fix followed these runs; the personal dispatcher then passed its Clippy stage and reported three private-lint diagnostics.

The repeated Clap gate diagnostic is resolved. The remaining personal diagnostics are `CommandHistory::next`, `CommandName::as_str`, and owned `Output::push` text, with the contract reasons recorded above. Markdown checks report no candidate diagnostics in the latest audit. The personal gate remains failed. Rendering, remaining examples, compatibility, coverage, media, and deployment work remain open.

## Resumed implementation, October 6, 2026

A new active goal continues the original implementation and publication scope.
The prior chat ended with a context-window error. Its latest source, staged
changes, six Wasm artifacts, and native/browser evidence remain in this checkout.
No push or deployment occurred during this resumed work.

The official release refresh still selects [Bevy 0.19.1](https://github.com/bevyengine/bevy/releases/tag/v0.19.1)
and [Clap 4.6.7](https://github.com/clap-rs/clap/releases/tag/v4.6.7).
[Rust 1.99.0](https://github.com/rust-lang/rust/releases/tag/1.99.0) is the latest
stable compiler. This checkout retains its reproducible 1.98.1 environment and
1.96.1 declared minimum; the minimum and newest compiler remain untested here.
Manifest dependency ordering changed without changing selected dependencies.

Long Clap help painted over input and contextual help in Firefox. The new
regression failed on the input's nonzero shrink factor before repair. Output
now has a clipping ancestor, and input cannot shrink. All 14 UI tests pass.

A separate host test proves compact mode hides both output and its clipping
ancestor, then drawer mode restores both. The rebuilt Clap browser example
confirms the repair, including accepted `scene rotate --speed -2` input.
[Browser confirmation](evidence/browser/clap-help-clipped.png),
[failed regression](evidence/output-clip-red.txt),
[UI results](evidence/output-clip-ui-tests.txt), and
[compact results](evidence/resume-compact-tests.txt) preserve the evidence.
Clipping does not complete scrollback navigation; scrolling, stable position,
and unread counts remain unfinished.

Local Firefox interaction also executed `echo window proof` in the movable
window and `scene speed 0` in the drawer. Escape closed the drawer. The logging
example accepted `demo burst 1100`, retained 1,031 records including seven
startup records, reported 76 capture losses and zero evictions, then accepted
`demo theme light`. Screenshots are saved in `evidence/browser/source-command.png`,
`quake-scene-reduced.png`, `kitchen-log-burst.png`, and `kitchen-light-theme.png`.
These results validate local artifacts, not deployed URLs or the full matrix.
The native CUA adapter does not expose dragging, so live window dragging and
curved-screen selection dragging remain unverified.

The 320-by-480 drawer check found a clipped shortcut hint.
[Captured defect](evidence/browser/quake-narrow-hint-clipping.png).
Repair the shared hint's width and confirm the narrow view before publishing.
The other five Wasm artifacts still need rebuilding after the source changes.

The exact format, `cargo check`, strict all-target/all-feature Clippy, and
`cargo test --doc` gates pass. The final default suite passes 50 tests including its
compile-fail doctest. [Full results](evidence/resume-full-tests.txt). Captured gate results
use the `evidence/resume-*` filenames.

An earlier all-feature test run ended
with status 143 while linking and does not count as a passing result.
The overlapping coverage run was interrupted when swap filled; current coverage
remains unverified. Run memory-heavy builds sequentially with two Cargo jobs.

The personal dispatcher now reports four Rust diagnostics; its Markdown,
Python, Nix, and OpenAPI stages finish without candidate diagnostics.
The Rust gate remains failed. No lint suppression or target configuration was
added. The exact result is [resume-personal-lints.txt](evidence/resume-personal-lints.txt).

`CommandName::as_str` already has the standard `AsRef<str>` implementation.
`CommandHistory::next` lends a result from the current mutable borrow, which a
standard Iterator associated item cannot express. `Output::push` consumes,
truncates, and retains its owned String, matching the lint's stated consumption
exception. Renaming `gameplay_input_allowed` would change an existing public
function name. Preserve these public contracts unless an API migration is
explicitly approved; do not describe the personal gate as passing.

Remaining work includes the narrow hint repair, scrollback navigation, token highlighting, and output severity styling,
pending execution/cancellation, the remaining acceptance boundaries, refreshed
coverage and Nix checks, feature and compatibility matrices, browser/IME and
clipboard checks, all rebuilt examples, thumbnails, real README media,
publication, and direct deployed-browser validation. The goal remains active.

## Pending execution checkpoint

The pending API uses `command::pending::PendingAppExt` as a separate opt-in
trait. The original `PromptlineAppExt` required methods remain unchanged.
A compatibility fixture first failed on the accidentally added required method,
then passed after registration moved to its separate trait. Shared identity
errors keep their original public variants and error sources.

Handlers return an immediate outcome or transfer a receiving endpoint.
A consuming completion endpoint provides one terminal send; a compile-fail
fixture proves that it cannot finish twice. The runtime polls without waiting
for worker data. Cancellation remains cooperative. Capacity defaults to 64
retained invocations and preserves existing work when new work exceeds it.

[The API guide](pending.md) records failure handling, ownership, and costs.

Twelve public pending tests pass for stalled work, concurrent dispatch and
analysis, selective cancellation, acknowledgement, worker failure, endpoint
loss, capacity, unavailable limits, UI origin, metadata and permission rejection,
opaque debug output, and terminal diagnostic redaction. A separate fixture
proves that old extension-trait implementations compile. Private tests check
retention and removed runtime state.

A terminal-error regression exposed a sensitive argument in a lifecycle message
before repair. Accepted metadata sensitivity now controls both immediate and
pending terminal failures. Redacted commands emit
`Failure("command failed for a sensitive command")`; public diagnostics remain
unchanged. [Failed regression](evidence/pending-sensitive-red.txt).

The final suite with `clap,log` passes 84 tests including two compile-fail
doctests. Five kitchen-sink tests cover deadline completion, cancellation,
identity cleanup, excess work, invalid arguments, and compact layout.
The exact format, `cargo check`, strict all-target/all-feature Clippy, and doc
gates pass. Evidence uses the `evidence/pending-*-final.txt` files.
Personal lints still report the same four public API-policy diagnostics; new
module, dependency-cycle, resource-access, and documentation diagnostics are
resolved. No lint suppression or target configuration changed.

The `kitchen_sink` Wasm artifact rebuilt successfully with its existing feature
profile. Firefox executed `wait`, then `echo during-pending` while invocation 1
remained running. `cancel` requested cancellation, and `demo history` brought
`Task cancelled` and invocation 1's `Finished(Cancelled)` into the visible output.
[Browser proof](evidence/browser/pending-cancellation.png).

The shared shortcut hint now wraps at a 320-by-480 viewport.
[Narrow confirmation](evidence/browser/kitchen-narrow-hint-fixed.png).
The narrow log panel remains cramped, and newest output rows can sit below the
current clip until later output moves the displayed tail. Output scrolling
remains required. The other five artifacts require refresh after these changes.

The configured coverage command previously finished at 92.86% of source lines,
below its required 100% floor. That run predates pending execution and the
module moves. [Recorded report](evidence/resume-coverage.txt). Current coverage
must be refreshed, and the uncovered lines must be tested without lowering the
floor. Scrolling, token highlighting, remaining platform and compatibility
checks, README media, gallery publishing, and deployed validation remain open.

## Scrollback checkpoint

`command::output::OutputView` keeps following and reviewing as separate modes.
Review counts arrivals by entry rather than inferring line counts. Clearing
output or replacing it with an earlier arrival sequence resets the baseline.
The default viewport assembles every retained entry instead of a fixed tail.
Source and entry-index buffers reuse capacity and refresh on output changes.

Wheel interaction uses Bevy's tagged unit conversion and actual layout sizes.
Following positions at the bottom after layout. Review anchors use retained
entry identity, shaped UTF-8 line ranges, and logical offset within a line.
Missing anchors after eviction clamp to retained history. A sticky `Latest`
control shows arrival state without becoming part of the editable draft.
[The scrolling profile](scrollback.md) records behavior and performance limits.

Three public scrollback cases and four focused geometry/interaction cases pass.
Geometry cases shape Bevy's bundled font through the actual Parley pipeline.
They cover arrival anchors, eviction, scale conversion, wheel units, nonfinite
deltas, unread counts, and explicit resumption. The full `clap,log` suite passes
91 tests including two compile-fail doctests. Exact formatting, `cargo check`,
strict all-target/all-feature Clippy, and doc gates pass. Evidence uses the
`evidence/scrollback-*-tests.txt` files.

Personal lints report the four previously recorded public API-policy diagnostics.
No new module or documentation diagnostic remains. No lint policy changed.

The `kitchen_sink` Wasm rebuild passes. Firefox displays the newest rows,
reviews earlier wrapped help after native wheel input, retains that help position
while `echo arrival while reviewing` appends five entries, and shows the arrival
count. Clicking `Latest` returns to the new result.
[Review and arrivals](evidence/browser/scrollback-review-arrival.png),
[latest output](evidence/browser/scrollback-latest.png).

The new control's middle-dot separator is missing from the bundled font and must
be replaced with supported text. Source-window selection, CRT wheel forwarding,
keyboard paging, touch scrolling, and rendering performance need further work.

The coverage run before this change finished at 93.22% of source lines and
failed the configured 100% floor. [Report](evidence/pending-coverage.txt).
Current scrolling code needs fresh coverage and direct evidence for its
remaining reachable branches. The complete implementation and online delivery
goal remains active.

The native CUA scroll adapter initially lacked its `ydotool` executable.
Installed the trusted Nix package into a task-created user-bin dependency path.
Its virtual device emitted X11 wheel events, but Firefox did not deliver wheel
DOM events. A compatibility wrapper now routes absolute movement and wheel
clicks through the session's X11 input path.

All verification input still uses CUA. This repair made Firefox wheel events and actual console review observable.

The wrapper is `/home/deck/.local/bin/ydotool`; the package binaries remain in
the Nix store. A task-started user daemon supports other ydotool operations.
The temporary pointer diagnostic wrapper was replaced after investigation.
No credentials, device ACLs, user mouse settings, or project lint policies changed.
Native drag remains unavailable through the documented CUA app interface.

## Selectable scrollback and CRT wheel checkpoint

The Source-window example now uses shared scrollback instead of a 24-entry
tail. A public regression first failed when an editable replacement received
no transcript. It now verifies all 30 Unicode entries, replacement of an
existing role, unchanged editor change ticks during idle, and later arrivals.
A private test verifies exact UTF-8 entry ranges, clearing, and source capacity
reuse. [Initial failure](evidence/selectable-scrollback-red.txt).

CRT wheel conversion preserves line and pixel units, both deltas, all four
touch phases, the virtual pointer identity, and the mapped image target.
A missed glass hit does not forward scrolling, even during a clamped drag.
The focused test first failed because conversion was absent.
[Initial failure](evidence/crt-wheel-red.txt).

The final `clap,log` suite passes 93 tests, including two compile-fail doctests.
The Source-window test and five CRT tests pass. Exact formatting, `cargo check`,
`cargo test`, strict all-target/all-feature Clippy, and doc tests pass.
Evidence uses `evidence/selectable-scrollback-*.txt`.

The unread label uses an ASCII separator because the bundled font lacked the
middle-dot glyph. Fresh browser checks for this label, selectable scrollback,
and CRT wheel routing remain pending.

Personal lints again report only the four recorded public API-policy
diagnostics. Markdown, Python, Nix, and OpenAPI checks pass. The predicate,
intent-comment, and paragraph diagnostics found during this checkpoint were
fixed; all code gates were rerun afterward. No lint policy changed.

The exact `nix run .#coverage` gate reports 93.59% source-line coverage and
fails the required 100% floor. It lists 176 uncovered lines, including 32 in
`src/ui/scrollback.rs`. [Report and exact lines](evidence/selectable-scrollback-coverage.txt).

Scrollback still needs direct evidence for anchor capture, role and parent
skips, geometry guards, invalid sequence handling, and label theme updates.
The new CRT `drive` wheel branch has unit-tested conversion but no current
native or browser execution evidence. These gaps remain open because their
focused tests and rebuilt-example interaction checks are not finished.

The public GitHub repository remains accessible with push and administration
permissions. Its Pages endpoint returns 404; no gallery deployment is complete.
Highlighting, interaction checks, coverage closure, compatibility branches,
README media, and deployed-example verification remain open.

## Anchor coverage and arrival exhaustion checkpoint

New tests capture Unicode entry identity from actual shaped lines, restore the
captured position, and discard anchors when following resumes. They cover
empty geometry, invalid scale, missing ranges, unavailable viewport parents,
unmatched roles, hidden-label theme changes, idle text change ticks, and inert
pointer targets outside the viewport hierarchy. No label-theme defect was
found.

The exhaustion regression first failed because unchecked arrival arithmetic
panicked after eviction, leaving retained output empty. Output now reserves
its next identity before truncation, eviction, or accounting changes.
Exhaustion panics with `output arrival sequence exhausted` and preserves
retained entries, bytes, loss count, and the last identity.
[Initial failure](evidence/output-exhaustion-red.txt).

The internal fixture seeds `u64::MAX - 1`, accepts the final `u64::MAX` arrival,
then verifies failure on the next write. A public setup would require 2^64
writes. The same test passes with this crate's overflow checks disabled, and
the verbose compiler log confirms `-C overflow-checks=off` on the test crate.
[Unchecked test](evidence/output-exhaustion-unchecked.txt).
The test uses a command-local [Cargo package override](https://doc.rust-lang.org/cargo/reference/profiles.html#overrides);
no manifest or project configuration changed.

The final `clap,log` suite passes 100 tests including two compile-fail doctests.
Default tests, Source-window and CRT tests, exact formatting, `cargo check`,
strict all-target/all-feature Clippy, and doc tests pass.
Evidence uses `evidence/anchor-final-*.txt`. Personal lints report only the four
recorded public API-policy diagnostics.

The final cases verify that repeated review scrolling preserves the unread
baseline, temporarily missing text components preserve retained output, and
unavailable shaped lines preserve review coordinates. Scrollback has 100%
line, function, and region coverage. Overall source-line coverage is 95.35%;
the exact coverage gate still fails its 100% floor with 144 uncovered lines
elsewhere. [Final coverage and exact gaps](evidence/anchor-coverage-final.txt).

The unhit `None` region at `src/output/mod.rs:174` is unreachable through
Output's public methods. An empty queue has zero retained bytes and remains
below the positive entry limit. Truncation ensures incoming text fits the byte
budget, so `is_full` returns false before `pop_front` can return `None`.
No coverage threshold, exclusion, or lint policy changed.

## Shared syntax analysis checkpoint

Public highlighting tests first failed on missing span and diagnostic APIs,
then failed on empty spans and missing local diagnostics. A Clap regression
failed because `rotate` had a value role rather than a subcommand role.
The shared decoder now emits presentation facts with its existing tokens.
One metadata cursor feeds Clap completion and syntax without running validators.
[API failure](evidence/highlight-api-red.txt),
[shape failure](evidence/highlight-shape-red.txt),
[Clap failure](evidence/highlight-clap-red.txt).

The UI guidance regression first displayed command help for an unfinished
quote. It now displays local quote, escape, unknown-command, NUL, and line-break
guidance while retaining the exact draft and emitting no command notification.
The existing stale-discovery regression still clears candidates and ghost text
after permission changes; help now contains the exact local tokenizer error.
[Guidance failure](evidence/highlight-guidance-red.txt).

All 109 `clap,log` tests pass, including three compile-fail doctests.
Exact formatting, `cargo check`, `cargo test`, strict all-target/all-feature
Clippy, example tests, and doc tests pass. Final evidence uses
`evidence/highlight-final-*.txt`.

The default personal lint run reports the four recorded API-policy diagnostics.
An additional `--fast --all-features` run needed the project Nix shell for
`libudev` metadata. It reports no Clap diagnostics after cursor refactoring,
but exposes unresolved logging diagnostics and additional public API-policy
conflicts. [All-feature report](evidence/highlight-personal-all.txt).
No API was removed, renamed, or suppressed to satisfy that report.

The exact coverage gate reports 95.24% source-line coverage and fails its
100% floor with 157 uncovered lines. The diagnostic and span modules have
100% line coverage; `completion/highlight.rs` has 91.06%. The default feature
scope does not execute its optional adapter paths.
[Coverage and exact gaps](evidence/highlight-coverage.txt).

Configurable visible
input colors, output severity colors, native and browser interaction checks,
version branches, README media, and deployment remain open.
[The current highlighting contract](highlighting.md).

## Output severity colors, October 6, 2026

Default plain output now styles retained entries without changing their text,
severity, order, or source ranges. `input::highlight::SeverityColors` provides
optional debug, warning, and error overrides. Trace shares debug styling;
informational output uses the theme. Sections reuse entities and string
capacity, inherit root font changes, and disappear after clearing or editor
replacement. Selectable output retains its existing single-color editor.

Public tests verify exact concatenated text and all five severity colors,
theme and font changes, section reuse, clearing, and reassigned role isolation.
The selectable-output test also verifies that replacement removes old sections.
Private tests cover both default palettes and explicit overrides.
The missing API and initial flat-color failures are recorded in
`evidence/output-style-api-red.txt` and `evidence/output-style-shape-red.txt`.

All eight Rust checks pass: formatting, check, default tests, `clap,log` tests,
Source-window tests, CRT tests, strict all-target/all-feature Clippy, and doc
tests. The feature suite passes 113 tests, including three compile-fail
doctests. Final results use `evidence/output-style-*.txt`.
The kitchen-sink Wasm rebuild passes.

Fresh Firefox checks verify error colors in dark and light themes at desktop
width and in a 320 by 480 viewport. Review preserves the visible position
after an arrival and shows `5 new | Latest`; clicking `Latest` restores the
newest output. Evidence is in `evidence/browser/output-severity-*.png`.
The wheel automation timed out after applying its action; screenshots verified
the resulting review state and subsequent controls. Native checks remain open.

The exact coverage command still fails its 100% floor at 95.49%, with 157
uncovered lines. Both `ui/colors.rs` and `ui/scrollback.rs` have 100% line,
function, and region coverage. The final report lists the remaining gaps in
history, completion, registry, keyboard, and other UI paths.
The default personal lint reports the same four API-policy diagnostics;
the additional logging diagnostics recorded by the all-feature run remain open.

Visible input colors, selectable output colors, coverage closure, version
validation, README media, full Nix checks, six-example runtime validation,
and deployment remain unfinished. The goal remains active.

## Input glyph colors, October 7, 2026

`input::highlight::HighlightColors` now styles input through Bevy's extracted
glyph colors. It preserves the editor's source, layout, clipping, selection,
cursor, composition, and ghost ownership. Seven independent optional overrides
cover every syntax role. [The rendering contract](highlighting.md) records
fallback behavior and rendering costs.

The first browser check exposed an incorrect layout source. Editable input
uses its own Parley layout rather than the ordinary `ComputedTextBlock`
buffer. The implementation and CPU fixture now use Bevy's editable-text
layout. Public tests verify actual editor-generated glyph geometry, colors,
node splits, shadow isolation, emoji, selection overrides, mismatched glyph
counts, invalid analysis, reassigned roles, stale analysis, and pending
layout. Private tests cover palette defaults, cache change detection, cluster
boundaries, decorator nodes, and invalid extraction ranges.

All eight required Rust checks pass after the last code/test edit. The
`clap,log` suite passes 120 tests, including three compile-fail doctests.
The CRT example passes five tests. Evidence uses `evidence/input-colors-*.txt`.
The default dependency graph contains no Clap dependency, and the kitchen-sink
Wasm rebuild passes.

The default personal lint reports only the four recorded
API-policy diagnostics. All-feature logging diagnostics remain unfinished.

Fresh Firefox checks show command, subcommand, option, quoted, incomplete,
and error colors in dark and light themes. Pointer selection works. A
320-by-480 viewport clips a long highlighted input horizontally while retaining
the caret and readable guidance. Screenshots are in
`evidence/browser/input-colors-*.png`. A synthetic Ctrl+A attempt inserted an
`a`; keyboard-modifier handling remains unverified. Real IME and native
interaction checks remain unfinished.

The exact coverage gate fails at 96.29%, with 144 uncovered aggregate lines.
Highlight analysis caching, glyph projection, and palette modules have 100%
line, function, and region coverage. Renderer functions have 100% coverage;
renderer lines have 98.97% coverage. Its remaining gap is the duplicate
`cfg(test)` copy of the role guard at `src/ui/highlight/render.rs:53`.
The public reassigned-role fixture exercises the normal copy; the private
decorator/range fixture exercises the test copy. Every renderer source line
has a hit across those copies, but the aggregate gate still counts a miss.

No threshold, exclusion, or lint allowance changed.
[Instantiation evidence](evidence/input-colors-render-instantiations.txt).

Selectable output colors, performance measurement, keyboard-modifier checks,
coverage closure, supported-version validation, README media, full Nix checks,
six-example runtime validation, and deployment remain unfinished.

## Logging formatting checkpoint, 2026-10-07

`FieldValue` now implements `Display` through its retained numeric types,
literal text, and redaction marker. Built-in formatters delegate field
formatting to that implementation. Public signatures and formatter output
remain unchanged. Capture still retains the accepted prefix when a debug
formatter fails and skips redacted debug callbacks.

The `log` suite passes, including eleven public capture tests and the private
sink-error test. All eight Rust checks pass after the final source/test edit.
[Logging tests](evidence/logging-tests.txt) and `evidence/logging-final-*.txt`
record these results. Fresh logging branch coverage and the configured
100% coverage gate remain pending.

The all-feature personal lint fixes remove logging implementation diagnostics.
Its remaining public API policy conflicts concern 27 reachable crate-root
names, `CommandHistory::next`, `CommandName::as_str`, owned `Output::push`
text, `gameplay_input_allowed`, and the four `FormatOptions` boolean fields.
Renaming these public items would change the preserved API. The personal
lint remains unresolved; no allowance or configuration changed.

## Transfer checkpoint, 2026-10-07

The user requested a checkpoint commit and push to the remote default branch
so implementation can continue on another machine. The original completion
goal remains unfinished. [The handoff](handoff.md) records the remaining work
and generated artifacts that Git does not transfer.

New regressions reproduce Space moving after a shifted quote, Ctrl+A
inserting text, and held Control surviving window focus loss. Console editors
now receive keyboard messages in order through Bevy's existing observers.
The opaque focused-input event still comes from Bevy's public dispatcher.

During console dispatch, observers see chronological logical keys and the
current keyboard batch. The original message collection and final frame key
state return afterward. Callback-generated keyboard messages wait for the
next input update. Other widgets retain Bevy's batch route.

This adapter follows Bevy 0.19.1's primary-window requirement. It derives
chronological keys from `KeyboardInput` messages. Later direct mutations of
`ButtonInput<Key>` do not replace that message stream. A host that already
replaced Bevy's standard dispatcher keeps its own integration. Controlling
sources are [focused dispatch](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_input_focus/src/lib.rs),
[text input](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ui_widgets/src/text_input.rs),
and [keyboard state](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_input/src/keyboard.rs).

The kitchen-sink log viewport exposes every retained matching row and keeps
its loss summary outside scrolling content. Manual scrolling begins review;
reaching the bottom resumes following. Geometry converts physical pixels to
logical scroll units through the inverse scale factor. Idle frames preserve
scroll change detection and do not reformat rows. Idle capture drains also
preserve `LogStore` change detection.

All ten final Rust checks pass in `evidence/input-logs-*.txt`. The personal lint still
reports nine preserved public API policy conflicts. Current branch coverage,
a fresh Wasm rebuild, and visual confirmation of these final changes remain
unfinished. Browser log screenshots show the earlier build's filter reload,
format changes, 976 losses after one 2,000-event burst, and 1,952 losses with
61 evictions after the second burst. Clearing records retained both counters.
The reproduced Ctrl+A failure is saved before the keyboard repair.
