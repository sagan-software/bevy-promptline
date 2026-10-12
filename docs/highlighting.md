# Shared input highlighting

`input::Analysis::highlights` returns `input::highlight::HighlightSpan` values.
Each span borrows its original UTF-8 source range and exposes a typed
`HighlightRole`. Quotes, escapes, and Unicode spelling remain unchanged in
the authoritative draft. Callers cannot construct unchecked spans.

The shared decoder emits tokens and presentation facts in one pass. Incomplete
quotes and escapes produce local `EditDiagnostic` values. These diagnostics do
not submit input, clear the draft, or emit lifecycle rejection messages.
The default help role displays their readable guidance. Forbidden NUL and
line-break input clears stale candidates and ghost text while displaying the
existing tokenizer error.

Command names and prefixes use permission-aware registry matching. Missing and
denied names receive identical error roles and diagnostics. Adapter metadata
can refine spans into subcommand, option, or value roles. Quoted values retain
their lexical role, and incomplete syntax takes precedence over metadata.

Clap completion and syntax use one immutable metadata cursor. The cursor handles
nested commands, aliases, assignments, separate values, declared value arity,
positional values, required equals syntax, and the end-of-options delimiter.
No value validator runs during analysis. Runtime parsing retains Clap's full
validation and the existing public error representation.

Analysis retains decoded tokens only while building its snapshot. Span storage
is proportional to token count; the snapshot owns the original draft.
Metadata traversal also depends on the active command's arguments and
subcommands. This does not claim complete validation of partial input or
measured frame performance.

`input::highlight::HighlightColors` configures command, subcommand, option,
value, quoted, incomplete, and error colors as optional app-resource overrides.
Absent overrides select defaults from the console background. Values use
`ConsoleTheme::input` by default. Input colors remain independent of output
severity overrides.

Painting reads the upstream editor's own shaped layout after Bevy UI layout.
It matches visual clusters to validated source spans, then changes only
ordinary extracted glyph colors. Glyph positions, atlas rectangles, transforms,
clips, cursor geometry, selection rectangles, and ghost placement retain
Bevy's rendering. Shadow glyphs are excluded from the tint pass.

When composition is active, painting preserves the editor's original colors.
When analysis is stale, editor layout is pending, or glyph counts differ,
painting preserves the original colors. Colored emoji and explicit
`TextCursorStyle::selected_text_color` overrides also retain Bevy's treatment.
A cluster spanning multiple roles retains its original foreground instead of
receiving a partial or incorrect token color.

The analysis snapshot moves into a private cache when its source or roles
change. Tint buffers reuse capacity.

Let C count clusters, G count glyphs,
T count source spans, and R count selection rectangles. Projection costs
`O(C log(T+1) + C + G(R+1))`; retained tint storage costs `O(G)`.
Extraction visits ordinary UI nodes and the input's glyphs each frame.
Upstream component change flags can
trigger projection during otherwise idle frames. Frame cost and allocation
counts remain unmeasured.

The host installs its renderer, camera, and window. Headless UI construction
does not create them. The tint hook installs at plugin finish when Bevy's UI
renderer exists. Default dependencies now include `bevy_ui_render`; Clap
remains optional. The lockfile resolves the feature change without new
package versions.

The implementation targets Bevy 0.20.0. Its controlling sources are
[editable layout](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui/src/widget/text_input_layout.rs)
and [UI extraction](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui_render/src/lib.rs).
Bevy 0.20 retains extracted UI nodes between frames. The tint pass changes
only inputs that Bevy rebuilt in the current frame. It skips the nodes already
present before ordinary text extraction, such as shadows. A refreshed tint
buffer marks the input's `TextColor` as changed, so Bevy rebuilds its glyphs.
Release verification on October 6, 2026 confirmed
[Bevy 0.19.1](https://github.com/bevyengine/bevy/releases/tag/v0.19.1)
and [Rust 1.99.0](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/).
Validation retains the existing Rust 1.98.1 snapshot; compiler-minimum and
other Bevy release checks remain unfinished.

Default plain output supports configurable severity colors through
`input::highlight::SeverityColors`. [Output presentation](scrollback.md)
describes its section storage and the selectable editor's remaining limitation.
