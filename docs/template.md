# Template baseline

The template was cloned locally into the Sagan Software checkout directory. This project copies its tracked files at `da85b65ffbad26f78d23c953d379a5b0d624e1ed` into a fresh repository, excluding its Git history and generated links.

Source: [Sagan-Software/template-bevy](https://github.com/sagan-software/template-bevy). Inspected October 5, 2026.

## Preserved files

All template source, tests, benchmarks, manifests, lockfiles, Nix configuration, formatting configuration, AI configuration, and license remain byte-for-byte unchanged. The root README now describes Promptline. The original README is preserved in `docs/template-readme.md`.

The baseline package remains `template-bevy` at 0.1.0. It pins Bevy 0.19.1 and Clap 4.6.6. It includes Avian, Lightyear, Hanabi, enhanced input, an egui inspector, and remote inspection tooling. These are template dependencies, not approved Promptline dependencies.

## Planned conversion

The first implementation milestone will replace the template game package with a library and focused examples. Remove networking, physics, particles, and inspector dependencies from the library. An example may use optional demonstration dependencies when they prove a requested integration.

Keep the template's Nix development workflow, locked dependencies, strict lint policy, Wasm profile, and lowest-sufficient-layer test approach. Update package names and Nix outputs together. Remove baseline game code only in the explicitly authorized conversion milestone.

The template's native entry point and two-world networking demonstration are not a browser console example. The current Rust files do not implement the proposed APIs.

## Instructions and checks

Tracked files contain `ai/AGENTS.md`, which requires current dependency verification and joint manifest/lockfile updates. No tracked `.gitlab/duo/mr-review-instructions.yaml` or other reviewer instruction file was found in the template. The copied instructions remain unchanged.

No Rust or Nix behavior changed during planning. Planning validation checks documentation formatting, local links, template preservation, repository state, and commit identity. Compilation, runtime interaction, screenshots, and compatibility remain unverified.

Future implementation must run the template's applicable checks after conversion. Do not copy the template's 50% coverage floor as a target for new command behavior. Aim for complete changed-path coverage and record every uncovered branch.

## Local state

`VERSION` is 0.1.1 after the CRT and logging planning update. It is not a published crate version. This repository has no Git remote. Future GitHub creation, pushes, package publication, and gallery deployment require a later instruction.
