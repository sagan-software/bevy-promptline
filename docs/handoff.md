# Handoff, 2026-10-07

This is an implementation checkpoint for transfer to another machine. The
original completion goal remains unfinished. Resume from `main` and inspect
[validation evidence](validation.md) and [the implementation prompt](implementation-prompt.md).

The checkout contains the typed command runtime, optional Clap adapter,
completion and help analysis, token colors, bounded selectable output,
pending completion and cancellation, optional structured log capture, and
six example targets. It pins Bevy 0.19.1, Rust 1.98.1, and wasm-bindgen 0.2.129.

The latest changes preserve keyboard event order for console editors, release
logical modifiers after window focus loss, and retain callback-generated
keyboard messages for the next frame. The kitchen-sink log viewport exposes
all retained matching rows, follows arrivals until manual review, and avoids
reformatting on idle frames. The capture drain preserves `LogStore` change
detection when no record arrives.

All ten checks in the final Rust validation runner passed. Results are
recorded in `evidence/input-logs-*.txt`.
The all-feature personal lint still reports nine public API policy conflicts;
its implementation diagnostics are resolved.

The last configured coverage
run reported 96.29% lines and failed the 100% floor. That report predates the
latest keyboard and log-viewport changes. Additional logging coverage was
95.10% overall lines and included the initial failing idle-drain regression.
Current changed-branch coverage needs another run.

The browser evidence includes live filter reload, malformed-filter rejection,
independent display selection and formatting, queue overflow, retention
eviction, and clear counters. Those captures use the earlier kitchen-sink
Wasm artifact. Final keyboard and log-viewport changes still require a fresh
Wasm build and visual confirmation.

## Remaining work

- Close production coverage gaps and resolve the documented API/lint conflicts.
- Run remaining Nix, feature, MSRV, and Bevy compatibility gates.
- Build and interactively validate all six native and Wasm examples.
- Verify real IME, clipboard, focus, resized layouts, and CRT pointer mapping.
- Finish configurable log presentation and remaining acceptance cases in the plan.
- Capture the real CRT README GIF and poster with a reproducible script.
- Finish the README tutorial, version support, and media.
- Configure, deploy, and interactively validate the GitHub Pages gallery.

## Local artifacts

`target/`, `site/`, and `local/` are ignored generated or machine-local data.
They are not transferred by Git. Rebuild the examples on the next machine.
The gallery builder requires a wasm-bindgen CLI matching 0.2.129:

```sh
nix develop --command python3 scripts/build-gallery.py --bindgen /path/to/wasm-bindgen
```

The personal lint checkout is also machine-local. Preserve the recorded
failed gates instead of treating absent tooling as a passing check.
