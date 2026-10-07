# Output scrolling

The default output viewport follows the latest retained output. A wheel action
above the bottom starts review. New output preserves the reviewed position and
shows an arrival count with a `Latest` control. Clicking `Latest` resumes
following without changing the command draft.

`command::output::OutputView` exposes `pause_following`, `follow_latest`,
`is_following`, `unread_entries`, and `synchronize` for replacement presentations.
Unread counts describe appended entries, including entries later evicted.
One multiline entry still counts once. Clearing output or replacing it with an
earlier arrival sequence resets review; arrival sequence is scoped to its Output
buffer and is not a global identity.

Arrival identities remain positive. When the arrival counter is `u64::MAX`,
the next write panics before truncation, eviction, or accounting changes.
Clearing output preserves the counter and does not reset exhaustion.

The viewport uses Bevy `ScrollPosition` in logical pixels. Physical content and
viewport sizes are multiplied by `ComputedNode::inverse_scale_factor` to obtain
the logical scroll range. Line wheel units use Bevy 0.19.1's
`MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR`; pixel units retain their input
value. Nonfinite deltas have no effect.

Review anchors use retained entry identity and the UTF-8 byte range of the first
visible shaped line. Parley's line metrics control placement after arrival,
wrapping, font changes, or resizing. The anchor retains its logical offset
within that line. Evicting the anchored entry clamps review to retained history
instead of silently returning to latest. No character-width estimate controls
scroll position.

The default source and entry-index buffers reuse capacity. Transcript assembly
runs on output changes or role creation and is linear in retained text bytes.
Bevy shapes the retained transcript and clips glyphs outside the viewport.
This does not claim constant frame cost, zero allocation, or a measured memory
bound beyond the configured retained text and entry limits; glyph layout and
atlas storage add overhead. Rendering cost still needs representative native
and browser measurement.

Sources are the tagged [scroll widget](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ui_widgets/src/scrollarea.rs),
[UI layout](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ui/src/layout/mod.rs),
[text layout](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_text/src/text.rs), and
[glyph clipping](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ui_render/src/lib.rs).

Replacement output roles can use Bevy `EditableText` instead of `Text`. Both
presentations receive every retained entry and share the source ranges used by
review anchors. Adding an editor to an existing role refreshes its transcript;
idle frames leave the editor unchanged. The selectable adapter reuses a source
buffer before copying text into the upstream editor's owned buffer.

Default plain output styles each retained entry by severity. Configure
`input::highlight::SeverityColors` as an app resource to override debug,
warning, and error colors. Trace uses the debug color. Informational output
uses `ConsoleTheme::output`. Absent overrides select defaults from the
console background; explicit overrides survive theme changes.

The root `Text` contains the first retained entry. Following entries use
`TextSpan` children with their original separators and text. Use Bevy's
`TextUiReader` to read the complete rendered transcript, or `Output::entries`
to read authoritative entries. Section entities and string capacity are reused
at each retained display position. Clearing output or replacing plain output
with an editor removes those sections. The selectable editor still uses one
text color; per-range severity colors for that adapter remain pending.

The Source-window example uses this adapter instead of a fixed 24-entry tail.
The CRT router forwards wheel units, deltas, and phases at the shader-mapped
image position. A missed glass hit cannot acquire scroll ownership, including
during a clamped drag. Unit tests verify conversion; current browser builds of
both examples still need interaction checks. Keyboard paging and touch scroll
gestures remain unverified. These gaps block the complete interaction claim.
