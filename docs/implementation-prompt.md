# Implementation-session prompt

Implement Bevy Promptline end to end in `/run/media/deck/Emulation/Code/github.com/sagan-software/bevy-promptline`.

Set an active goal with the goal tool: finish repository initialization, implement the reusable plugin and six examples, publish the example gallery, and validate the README media and live examples. Continue until that goal is achieved or a genuine external blocker requires my input. Mark it complete only after its required evidence exists.

The local repository already contains the unchanged Sagan-Software/template-bevy baseline and committed planning documents. Inspect its status and instructions first. Preserve existing history and unrelated work. Complete initialization by converting the baseline into the planned library and examples. If the directory is missing, initialize it from the recorded template baseline and recover the planning packet before implementing.

Read README.md, PRODUCT.md, ai/AGENTS.md, and docs/plan.md, architecture.md, compatibility.md, examples.md, logging.md, crt-workstation.md, research.md, and template.md. These documents define the scope and acceptance checks. The decision to build an independent plugin is settled. I want control over its visual design, interaction, and architecture. Keep alternatives linked neutrally in the README.

Implement Bevy UI rendering, public traits and system integration, optional Clap derive/builder support, Tab completion, ghost text, live contextual help, syntax highlighting, history, typed command delivery, lifecycle notifications, focus gating, bounded output, and optional Bevy log capture. Implement separately configurable capture filters, display filters, and log formatting. Support live capture-filter changes while documenting global and compile-time filtering limits.

Build six independent native/Wasm examples: minimal, quake_drawer, source_window, clap_commands, crt_workstation, and kitchen_sink. Keep compact and light-theme modes in kitchen_sink. The CRT example must show a desk and old monitor in a dark void. Render the actual Bevy UI console to its screen texture. Add a WebGL2-compatible WGSL CRT effect, accurate pointer mapping, a close-up editing view, and DOS-inspired commands over a virtual filesystem. Keep 3D and shader dependencies outside the default library.

Resolve APIs against current primary sources. Target Bevy 0.18 and 0.19 with matching release branches, plus a pinned 0.20 preview. Refresh these facts at kickoff. Record preview limitations honestly. Use focused failing public tests before production implementation, documented module skeletons, coherent behavior slices, internal tests, complete changed-path coverage, and a final separate documentation edit. Follow relevant installed Rust, lint, and frontend skills.

Run every required gate in docs/plan.md after the final changes. Build and interactively test all six Wasm examples. Validate command side effects, Clap errors/help, Unicode/IME, focus, completion, logs, shader controls, and curved-screen selection. Review rendered desktop, Steam Deck, and narrow views. Fix observed defects and confirm them with bounded checks. Record unavailable platform checks explicitly.

Capture a real interaction sequence from the completed CRT browser example. Produce and inspect a looping README GIF, a static poster, and optional higher-quality video. Keep a reproducible capture script. Write the README with compiled usage examples, supported versions, customization, alternatives, the GIF, and direct live-example links.

You are authorized to create the public `Sagan-Software/bevy-promptline` GitHub repository, push validated commits without force, configure GitHub Pages, and deploy the gallery using existing credentials and free services. If that repository exists, inspect it before attaching the local history. Preserve the configured user Git identity and omit AI attribution. Commit coherent validated milestones and maintain VERSION. Crates.io publication, paid services, and destructive replacement of existing remote content are outside this authorization.

Validate the deployed gallery and every direct example URL through real browser interaction after deployment. Check project-subpath asset loading, fetch/error states, actual command effects, CRT input, and README GIF loading. Save test results, screenshots, coverage evidence, media provenance, deployed URLs, and exact unresolved gaps in docs/validation.md. Report completion only for behavior and deployments actually verified.
