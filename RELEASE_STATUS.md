# SHED POWER - Release Status

Last updated: 2026-05-13

This document tracks current release blockers and caveats only. Historical planning documents may still contain aspirational or stale references.

## Current Scope

- Releaseable targets: native Tauri desktop build and Android build artifacts, after the required signing and smoke testing.
- Out of scope: VST3/Reaper plugin output. No `shed_power_vst3` Cargo bin target, `vst3` Cargo feature, or `.vst3` bundle packaging is present.

## Blockers and Caveats

- Manual hardware smoke is still required before public release: desktop audio device, desktop MIDI controller, Android device install, and Android audio session.
- Android artifacts from 2026-04-17 were unsigned. Distribution requires signing and the appropriate release/install workflow.
- Android native MIDI/controller support is not releaseable yet. The Rust backend disables the desktop `midir` path on Android; USB/controller MIDI needs a dedicated Android MIDI backend.
- CDN/CSP hardening may be handled by another worker. This cleanup does not claim that work is complete.

## Related Files

- `README_BUILD.md` contains current build commands and release caveats.
- `src-tauri/build_vst3.ps1` is a fail-fast status stub, not a VST3 build script.
