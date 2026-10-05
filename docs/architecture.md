# Proposed architecture

This document defines intended behavior for review. Type names and signatures are proposals, not implemented API claims.

## Invariants

Commands have unique validated identities. Aliases resolve to one identity. Duplicate names or aliases fail registration with a diagnostic before the app runs. Command metadata does not grant execution permission.

A console is either closed or open with an input focus state. Opening acquires focus; closing releases it and restores a valid prior focus target. Animation progress is presentation state derived from the requested visibility and elapsed time.

An invocation is submitted, rejected, or accepted. Accepted invocations become running, then succeed, fail, or cancel. Parsing failures and authorization failures remain distinct from handler failures. A handler runs at most once for a given accepted invocation.

Ghost text, highlighted spans, and contextual help derive from the actual input, caret, registry revision, and selected candidate. They never become authoritative input until the player accepts completion.

Externally supplied names, token ranges, limits, invocation identities, and completion revisions need validation. Durations use `Duration`. Buffer cursors use validated UTF-8 boundaries and grapheme-aware editor navigation. Do not store raw numeric values for distinct semantic identities interchangeably.

## Modules and integration

Start with one crate and private modules for registry, tokenization, completion, execution, output, editing, UI, and optional adapters. Keep `lib.rs` as re-exports and crate documentation. Add separate crates only when measured dependency or reuse needs justify them.

Expose a default `PromptlinePlugin` plus smaller runtime and UI plugins. A game can use the command runtime with a replacement Bevy UI. The runtime owns no camera, assets, window, renderer, or operating-system terminal.

Propose `ConsoleCommand` for command metadata and parsing into typed input. A registry adapter connects that input to an ordinary Bevy system. Propose `CompletionProvider` for contextual game values. Expose plain registration builders for small commands without a custom derive macro. Document extension traits by name in guides.

Keep ECS mutation in registered systems, not parser or completion callbacks. Parse and describe incomplete input without changing game state. Offer a scheduled read-only completion system for values such as entity names. Snapshot only the small candidate data needed by the UI.

Expose system sets for input, analysis, dispatch, execution, output drain, and presentation. Input capture runs before cooperating gameplay systems. Applications can order their systems and use a public focus run condition. Bevy cannot prevent an unrelated system from reading global keyboard state; the integration guide must show the gate.

## Command text profile

The initial console accepts one UTF-8 command per submission. Names are case-sensitive. Whitespace separates tokens outside quotes.

Single quotes preserve their content literally. Double quotes preserve spaces and permit escaping a quote or backslash. Outside quotes, backslash escapes the next character. Empty quoted tokens remain arguments.

Submission rejects unmatched quotes, a trailing escape, embedded NUL, and embedded line breaks. Typing analysis accepts incomplete quotes and escapes so completion can still work. Preserve original input and source ranges alongside decoded tokens. Do not normalize Unicode spelling.

Support `--` and pass all decoded arguments, including negative values, to the registered parser. Tokenization does not interpret flags. Do not add pipes, redirection, shell expansion, environment substitution, comments, command chaining, or arbitrary process execution. A semicolon is an ordinary character in this profile.

Single-line paste rejects multiple lines before changing the draft and explains the rejection. Limits apply before parsing. A proposed default is 4 KiB per submitted input. Allow applications to configure a validated positive limit.

The same tokenizer feeds completion, highlighting, help, and dispatch. A completion identifies a source token range and replacement text. Serialize replacement text with the same quote rules so accepting a value containing spaces produces the intended argument.

## Optional Clap adapter

Support existing `Parser` types through `CommandFactory` and `FromArgMatches`, without requiring a Promptline derive. Support builder `clap::Command` values through a handler receiving `ArgMatches`. Register a top-level command by its existing name. Preserve its subcommands, aliases, defaults, conflicts, required arguments, custom validators, and `--` behavior.

Use fallible argument parsing. Never call Clap's exiting parse functions, `Error::exit`, or process termination from the adapter. Render `DisplayHelp` and `DisplayVersion` as successful informational output without executing a game handler. Other Clap errors produce rejected invocations with readable diagnostics and the original error source where feasible.

Choose an explicit `argv[0]` policy at registration. Default registration supplies the command name as the binary token. Respect an explicitly registered `no_binary_name` command. Treat multicall and external-subcommand configurations as separate acceptance cases rather than assuming nested subcommands cover them.

Clap controls final argument validation. Stable metadata supplies names, flags, value hints, possible values, descriptions, and usage. The live helper must not repeatedly run arbitrary custom validators while typing. A custom parser can accept values that metadata cannot enumerate. Permit a game completion provider to supplement those values.

"Any Clap command" has a boundary. A parser accepting an arbitrary filesystem path cannot imply browser filesystem access or enumerate game assets. A parser requiring non-UTF-8 `OsString` input cannot receive that value from a UTF-8 console. Document those limits. Do not rewrite parser behavior to make an incompatible command appear supported.

Treat environment-dependent defaults, filesystem validators, and application-specific side effects as host behavior. Native and browser fixtures must prove their intended profiles. Completion coverage is not proof of parse compatibility.

Sources: [Command](https://docs.rs/clap/4.6.6/clap/struct.Command.html), [CommandFactory](https://docs.rs/clap/4.6.6/clap/trait.CommandFactory.html), [FromArgMatches](https://docs.rs/clap/4.6.6/clap/trait.FromArgMatches.html), and [dynamic completion status](https://docs.rs/clap_complete/latest/clap_complete/).

## Completion and live help

Offer command names, subcommands, flags, enum values, and game-supplied values at the caret. Match prefixes by default. Keep any future fuzzy matching opt-in. Rank exact matches first, then prefix matches in lexical order with a stable identity tie-breaker. Hide commands and candidates that policy makes unavailable to the current actor.

If one candidate matches, Tab inserts it. If several candidates share a longer prefix, Tab inserts that prefix and opens the list. Otherwise Tab selects the first candidate. Further Tab presses cycle forward, and Shift+Tab cycles backward. Freeze the cycle list until the draft changes. Enter submits the resulting draft; it never silently accepts a highlighted candidate instead.

Show ghost text only for a selected prefix completion at the end of the token with no active selection or IME composition. Show the untyped suffix in a muted span aligned with the input layout. Tab accepts it through the same replacement operation. Never place ghost text in history, submitted input, copied text, or the accessible editable value.

Place command summary, usage, and current argument description in a fixed help region beside or below the input. Update it when input, caret, selection, or registry metadata changes. Reserve its space to prevent typing from moving. Full help appears in scrollback only on explicit request.

Use revision identities for asynchronous candidate providers. When a provider completes for a stale draft, discard its result. Bound the candidate count and work per frame. Start with synchronous metadata and read-only game snapshots; add asynchronous providers only after a real use case requires them.

## Editing and highlighting

Support caret movement, Home/End, selection, deletion, word navigation, undo/redo, clipboard where available, and command history. Preserve the draft while navigating history. Up/Down navigate candidates when the list is active; otherwise they navigate history. Escape dismisses candidates first, then closes the console.

Respect IME composition. Enter during composition commits the composition rather than submitting. Tab during composition does not complete. Opening the console must not insert the toggle character. Provide rebinding and a visible open button for browser, international keyboard, touch, and Steam Deck use.

Highlight command names, subcommands, options, argument values, quoted text, and syntax errors with configurable roles. Highlight incomplete input without reporting a submission failure. Keep diagnostic text available without relying on color. Clip input, selection, and ghost spans consistently during horizontal scrolling.

Assess upstream 0.19 editing before designing custom rendering. If it cannot render token styling and ghost text without breaking editing, prototype a separate Bevy UI overlay using its layout data. Do not duplicate text shaping with guessed character widths.

## Notifications and execution

Propose typed lifecycle payloads for submitted, accepted, rejected, started, and finished invocations, plus visibility and focus changes. Use Bevy messages for buffered system consumers and targeted events for explicitly observer-driven integration. Document the version-specific wrappers; "events" in a user's workflow does not require every notification to use the same Bevy primitive.

Assign an `InvocationId` and `ConsoleId` at submission. Carry the registered command identity and source category where known. A submitted notification precedes validation. A rejection prevents dispatch. An acceptance precedes exactly one handler call. One terminal notification follows success, failure, or explicit cancellation.

Typed command delivery has one execution owner. Observers can inspect lifecycle notifications without executing the command again. Multiple invocations submitted in one frame retain their queue order. Document when deferred ECS changes become visible to later handlers.

Keep execution on Bevy's schedule for ordinary game systems. Long work returns a pending task handle and reports completion later. Never hold world access across background work. Cancellation requests cooperate with handlers; they cannot undo completed side effects. Do not promise forced cancellation.

Bound the invocation queue. Reject overflow visibly rather than dropping commands. A proposed capacity is 64. Native and Wasm use the same status contract without requiring a particular async runtime.

## Output and logs

Use structured output entries with sequence identity, severity, source, optional invocation identity, and styled text spans. A game system can write output without Clap or log capture. Default output is plain text, so escape sequences cannot operate on the editor or execute commands.

Keep a bounded scrollback, proposed at 2,000 entries and 2 MiB total retained text. Limit a single entry to 16 KiB with a visible truncation marker. Evict oldest entries and report loss counts. Render only visible rows plus bounded overscan. When the player scrolls upward, preserve their position and show an unread count instead of forcing the newest line into view.

The optional log feature supplies a `tracing` layer installed through Bevy's `LogPlugin::custom_layer` or a caller-owned subscriber. Create its queue before subscriber initialization. Adding the console after logging starts cannot retrofit a global subscriber. Do not install a second global subscriber or suppress ordinary native/browser logging.

Drain a bounded cross-thread queue on the main thread. Apply capture filters before formatting. Use nonblocking enqueue, a dropped-entry counter, and a per-frame drain budget. Do not log from the layer's overflow handling or recursively mirror console output into capture.

Support level and target filters, pause display, search, copy, and clear in the advanced example. Preserve structured fields for inspection where useful. Document that compile-time and upstream subscriber filters can prevent entries from reaching the console. Do not promise capture of every log.

The detailed [logging contract](logging.md) controls EnvFilter behavior, runtime reload, retained structured records, and custom formatters. The [CRT brief](crt-workstation.md) controls offscreen rendering and screen-pointer mapping.

Source: [tagged Bevy logging implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_log/src/lib.rs).

## Customization, policy, and costs

Expose documented UI role components for the root, input, output, help, candidate list, drag region, and controls. A game can spawn its own Bevy UI tree and bind it to the runtime model. Theme resources control fonts, spacing, colors, token styles, selection, caret, and ghost contrast. Custom systems can animate or move role entities without private entity lookups.

Default registration contains only explicit console utility commands such as help, clear, and history. Do not expose world mutation, quit, cheats, reflection, networking, or operating-system access by default. Games choose commands and actor policy. Hidden UI is not authorization; validate policy on programmatic submission too.

Commands may mark arguments as sensitive. Redact them from echoed input, history, logs, and lifecycle observation payloads. Keep raw parser input short-lived and restricted to the execution owner. Disable persistence by default. Permission-aware metadata prevents help and completion from exposing unavailable commands.

Analyze only when input or relevant metadata changes. Registration stores reusable metadata. Prefix completion is initially O(N + returned text) over N relevant candidates with a capped result list. Token analysis is O(L) in input length L. Retained history and output remain bounded by configured limits. Benchmark realistic registries and log bursts before adding indexes or buffer machinery.

Screen-reader integration depends on Bevy's platform support. Test accessible names, focus traversal, completion announcements, and output announcements in native and browser builds. Record unsupported paths explicitly. A static HTML guide provides instructions but does not prove the canvas console accessible.
