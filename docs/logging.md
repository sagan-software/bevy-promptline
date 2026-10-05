# Logging capture, filters, and formatting

This is a proposed contract for the optional `log` feature. No capture layer or UI controls have been implemented.

## What Bevy supplies

Bevy logging uses `tracing` and bridges the Rust `log` facade through `LogTracer`. A tracing layer receives event metadata and recorded fields before terminal text formatting. Promptline should capture that structured event, enqueue it, and render it through Bevy UI on the app thread.

Use `LogPlugin::custom_layer` during initial plugin construction. Preserve the existing native or browser log sink. Capture is process-wide when attached to a global subscriber; it does not infer which Bevy App originated an arbitrary event. Document that boundary for applications with several Apps. Install one global subscriber and explicitly route its capture queue to the selected console.

A plugin added after subscriber initialization cannot attach another layer to that existing subscriber automatically. Offer a layer factory for applications that own their subscriber. Report unavailable capture without breaking ordinary command output.

Controlling source: [Bevy 0.19.1 logging implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_log/src/lib.rs). The template resolves `tracing-subscriber` 0.3.23; verify this dependency before implementation.

## RUST_LOG is a filter

`RUST_LOG=info,my_game=debug,wgpu=warn` selects event levels by target. It does not set timestamps, colors, field order, or the rendered text layout. Use [EnvFilter's documented directive grammar](https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/filter/struct.EnvFilter.html).

Bevy 0.19.1 starts with `LogPlugin.level` and `LogPlugin.filter`, then adds `RUST_LOG` directives. Those directives can selectively override matching defaults. It ignores empty comma-separated segments. If an environment directive is malformed, it reports the setup error to stderr and falls back to the configured defaults. Recheck this behavior on each supported Bevy line rather than claiming all versions behave identically.

The host's global subscriber filter limits every attached sink. A stricter console capture filter narrows that set. A display filter selects entries from already retained console output. Compile-time log filters can remove events before any runtime filter receives them.

Changing the console filter to `debug` cannot recover events rejected by a global `warn` filter. Changing a display filter cannot recover events previously rejected during capture or evicted from the buffer. A clear effective-filter summary must state those limits.

## Proposed configuration

Provide separate typed settings for capture and display. Capture accepts a validated EnvFilter directive string. Display supports level, target, and a text query over retained entries. Changing display selection should not discard retained output.

The default capture filter allows the records that the host subscriber emits. Games may choose a stricter initial filter. Browser builds receive configuration from application data or gallery controls; browser JavaScript does not supply a normal process `RUST_LOG` environment variable.

Add `log filter <directives>` and `log filter` in the advanced example. The first updates the console capture filter; the second prints the current filter and its scope. A malformed update keeps the old filter and reports the parse error. Candidate completion for common target names must not evaluate a filter or generate log events.

Use a reloadable layer-local filter, following [tracing-subscriber reload guidance](https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/reload/index.html). Update callsite interest through the established reload mechanism. Do not cache a rejected callsite forever or format records to decide whether to enqueue them.

Runtime changes to the host's global filter require an explicitly caller-owned reload handle. Keep that integration optional. A console filter command must not silently change stderr, tracing profiler, file, or browser-console capture policy. Never modify the process environment and imply that an installed subscriber rereads it.

## Formatting contract

Store an owned structured record because the tracing callback's borrowed data does not survive enqueue. Include severity, target, message, event fields, capture sequence, and elapsed capture time. Elapsed time uses `Duration` from capture initialization. Sequence establishes queue order; time does not establish causal order between threads.

Represent event fields with their name and typed boolean, signed integer, unsigned integer, floating-point, string, or diagnostic text value. Preserve diagnostic text as such rather than pretending to recover its original type. Add file, line, thread, and span context only when available. Missing metadata stays absent.

Use stable field ordering for presentation, proposed as lexical field-name order. Preserve the event message separately from other fields. Set limits on field count, field bytes, span depth, and total record size; document truncation and loss. Apply redaction before retaining or formatting sensitive fields. Direct command output and tracing records remain distinct entry categories.

Expose a `LogFormatter` trait that converts retained records into styled text spans or a typed row model. Offer a default readable formatter and a compact preset. Settings control timestamp visibility and precision, level labels, target labels, fields, source locations, wrapping, and severity styles. A custom formatter can produce a DOS-like line or a developer-oriented expanded row without modifying capture.

The default timestamp is elapsed time in seconds, displayed with three fractional digits. Do not imply that this is UTC. If an optional wall-clock formatter is added, store an established timestamp type and emit UTC with `Z` and millisecond precision. Test exact output and name the supported offset policy.

Theme colors and row layout are separate from text content. Reformat retained records when the formatter changes without recapturing them. Native stderr formatting is a host concern: `LogPlugin::fmt_layer` can customize it on supported native paths. The browser's ordinary sink uses Bevy's Wasm layer, so do not promise that `fmt_layer` changes browser developer-tools output.

Use [FormatEvent](https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/fmt/trait.FormatEvent.html) for a host-owned tracing text formatter when needed. Promptline's formatter consumes its own retained record model. An optional export formatter must escape text correctly; never interpret log text as commands or terminal control sequences.

## Required proof

Test target-specific overrides, level defaults, empty directives, malformed capture updates, and upstream rejection. Use an isolated process for global subscriber and environment tests so parallel tests cannot contaminate one another.

Test live reload at a callsite that was previously disabled. Keep another sink attached and assert that changing the console filter does not change that sink. Test browser configuration without environment access.

Test typed fields, diagnostic values, missing metadata, redaction, field ordering, and exact compact/custom output. Change format after capture and assert the retained event renders differently without duplicate capture. Snapshot representative and empty/failure output while retaining focused assertions for filtering and escaping.

Stall the app's drain, generate a bounded burst from several writers, and prove enqueue does not block waiting for drain. Verify overflow counts, truncation, disconnect behavior, and no recursion from overflow reporting. Resume drain and assert bounded work per frame and stable input progress.
