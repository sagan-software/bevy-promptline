# Product requirements

The user requested planning and local repository initialization. Implementation is pending review.

## Users and purpose

Game developers need a reusable Bevy plugin they can add to multiple projects. Players and developers need to discover and execute custom commands without knowing their names or argument syntax in advance.

## Reason for building

The user chose an independent implementation to control its visual design, interaction, and architecture. Existing plugins remain useful alternatives and research references. The project does not require a claim that those alternatives are defective or lack every proposed feature.

A mature result requires consistent typography, deliberate spacing, readable help, accurate caret and ghost alignment, restrained default controls, complete error states, and working browser demonstrations. These are acceptance criteria for future review, not properties already established by this repository.

## Required capabilities

- Render the console entirely with Bevy UI.
- Support traits, custom command systems, and command lifecycle notifications.
- Provide Tab completion, ghost text, contextual help while typing, and syntax highlighting.
- Integrate existing Clap commands without requiring Clap for other users.
- Allow applications to replace layout, styling, controls, and animation.
- Display game output and optionally capture Bevy logs.
- Target the upcoming Bevy release, the current release, and the previous release.
- Provide six independent browser examples and a gallery similar to Bevy's examples page.
- Demonstrate a Quake-style animated drawer and a Half-Life-style movable window.
- Render the Bevy UI console onto a CRT monitor at a desk, with a screen shader and DOS-inspired example commands.
- Expose log capture filters and formatting customization, including live console filtering.
- Capture an actual working CRT example as a README GIF and publish runnable online examples.

## Platforms

The plugin runs inside native and browser Bevy applications. The browser gallery is a static website whose examples are separate Wasm builds. Touch access and a visible open control are proposed additions because tilde requires a keyboard.

## Planning assumptions

Use a lightweight command interface rather than implementing a shell, operating-system terminal, or scripting language. Start with one interactive console per app. Retain enough console identity in notifications to permit future multi-console support without pretending it exists today.

Use Bevy 0.19 as the first implementation target, then port to 0.18 and validate against a pinned 0.20 development revision. Publish only after release validation. Treat Clap as an optional feature, disabled by default.

## Open review decisions

The working name, license choice, version policy, and feature boundaries are recommendations. The user can revise them before implementation. The future Bevy 0.20 API and the text-editing adapter for 0.18 require implementation evidence.
