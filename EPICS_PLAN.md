# SHED POWER Epics Plan

## Context Clear

The original HTML app is the feature reference. The `Power` app is the shipping target. Work starts with platform stability, then feature parity, then the missing learning surfaces.

## Epic 1: Platform Baseline

Goal:

- make the native app compile cleanly
- remove immediate runtime breakages
- establish trustworthy save/load and command flow

Scope:

- Rust compile blockers
- broken shell shortcuts
- shell command/event stability
- baseline build verification

Initial slice:

- fix `cargo check`
- fix the broken metronome shortcut import path
- verify frontend and backend builds

Status: complete

Closeout note, 2026-04-16:

- `cargo check` passes in `src-tauri`.
- Elevated `npm run build` passes in the project root.
- The previous active status was stale; no platform-level compile or shell build blocker remains for the non-Epic-2/Epic-6 work.

## Epic 2: Looper Parity

Goal:

- match the original looper workflow with a stable native implementation

Scope:

- record/play/overdub/undo/clear behavior
- song sequence playback
- waveform reliability
- metronome behavior
- per-part gain and monitoring polish

## Epic 3: MPC Parity

Goal:

- bring MPC behavior closer to the original HTML workflow

Scope:

- proper kit loading behavior
- sample loading and editing
- sequencer state ownership
- transport linkage
- visual feedback parity

Status: complete for current parity slice

Progress note, 2026-04-16:

- Kit loading now updates both UI and Rust engine state, resets pad sample labels, and keeps MIDI kit changes in the same flow.
- Sample-backed pads now take priority over procedural drum fallback, so imported WAVs do not double-trigger kit drums.
- MPC sequencer playback moved from a Svelte `setInterval` mock into the Rust audio engine with BPM/swing-aware timing and UI step/transport events.
- Sample loading now emits waveform metadata to the UI.
- Sample editing now includes start/end trim, volume, and pitch controls wired to Rust sampler playback state.
- Pad trigger visual feedback is driven by native trigger events, and sequenced hits no longer steal selected-pad editor focus.
- `cargo check` passes after the latest MPC sampler/sequencer changes.
- Frontend build passed after the earlier MPC slices; the final `npm run build` after adding sample volume/pitch still needs re-run because the required elevated build was interrupted.

Closeout note, 2026-04-16:

- Re-ran elevated `npm run build`; it passes after the latest sample editor changes.
- Inspected drag/drop sample loading, waveform display, trim boundaries, volume, pitch, sequenced playback, MIDI pad triggering, and kit switching. The paths are wired through the Svelte component, frontend bridge, Tauri commands/events, and Rust sampler/sequencer.
- Stereo pan remains backlog. The current Rust sampler and final mix path are mono, so adding pan is a broader mix-engine feature rather than a small MPC closeout blocker.

Follow-up backlog:

- Add stereo pan if the sampler/mixer path becomes stereo-aware.
- Optional legacy parity backlog: file-picker import and record-to-pad workflow. Current native sample loading is drag/drop path-based.

## Epic 4: Jam Station Completion

Goal:

- complete the jam engine and theory workflow

Scope:

- backing track flow
- bassline and harmonics behavior
- custom song playback in Rust
- visualizer consistency
- Spark handoff into Jam Station

Status: complete for current parity slice

Progress note, 2026-04-16:

- Backing-track playback moved from Svelte `setTimeout` chord scheduling into the Rust audio engine via `PlayJamTrack`.
- Preset backing tracks now send structured chord data to Rust, including MIDI notes, beat duration, and chord label.
- Rust Jam sequencing now honors each chord's `beats` value instead of assuming fixed four-beat changes.
- Rust emits `jam-chord-step` events back to Svelte so the active progression display follows the engine-owned sequencer instead of a parallel frontend timer.
- `PlayChord` bridge shape was fixed so frontend calls include `tempo`, allowing the existing Rust bassline and harmony engines to receive the correct BPM.
- Shared note/chord helpers now normalize common flats and enharmonics such as `Eb`, `Bb`, `Ab`, `Cb`, and `Db`, fixing wrong-pitch Jam chords from flat-heavy standards.
- Custom song playback now has a Rust-side sequencer instead of a log-only TODO. Text parts such as `Am F C G` are parsed into chords and played through the same Jam chord trigger path.
- Custom song UI now mirrors the generated progression while Rust owns playback timing.
- Spark Generator handoff now creates a temporary Spark backing track, switches into Jam Station, and starts playback there.
- Jam chord-step events now include chord index, MIDI notes, and label. Jam Station uses that payload to show the current chord label and highlight active chord tones separately from scale tones on guitar and piano visualizers.
- Track switching now awaits `StopChord` before starting the next Jam playback command, reducing the chance of an async stop killing a newly started track.
- `cargo check` passes after the latest Jam event/visualizer changes.
- `npm run build` passed after the earlier Jam sequencer and Spark handoff work. The final `npm run build` after adding richer chord-step payloads and chord-tone visualizer highlighting still needs re-run because the elevated build was interrupted.

Known technical notes:

- Sandboxed `npm run build` fails with Node `EPERM` on `C:\Users\Sebastian`; the successful build requires the approved elevated `npm run build` path.
- `rustfmt` over the touched Rust files is currently blocked by pre-existing trailing whitespace in `src-tauri/src/audio/midi.rs`. `cargo check` is not blocked by this.
- Jam Station is still named Epic 4 in this file, while recent working sessions referred to it as the active Epic 3 slice because the delivery order places Jam Station before MPC.

Closeout note, 2026-04-16:

- Re-ran elevated `npm run build`; it passes after the latest Jam chord payload and visualizer updates.
- Inspected preset track play/stop/next/prev, MIDI Jam controls, Spark handoff, custom song playback, bassline toggles/styles, harmonics toggles/presets, and guitar/piano visualizer highlighting. The paths are wired through Jam Station, the frontend bridge, event listeners, and Rust `PlayJamTrack`/`PlayCustomSong` sequencing.
- Added UI feedback for invalid custom song chord tokens and sanitized custom playback so the displayed custom progression matches the Rust playback list.
- Custom song parsing remains intentionally simple for this slice: whitespace/comma/pipe-separated chord tokens at four beats each. Explicit beat lengths and section syntax are backlog rather than release-critical.
- Compared against the legacy Jam Station/Jam Mixer once more. Remaining legacy-only practice controls are mixer/drum volume faders, manual root controls, drone/metronome practice helpers, and additional bass patterns such as arpeggio/jazz walk. These are polish/backlog unless the release scope explicitly requires full legacy Jam Mixer parity.

Follow-up backlog:

- Add explicit beat lengths and section syntax for custom songs.
- Restore legacy Jam Mixer practice controls if they are needed for the release scope.
- Extend Jam chord events with richer musical metadata if needed for full parity, such as chord quality, active bass note, harmony voicing, and suggested scale/mode.
- Consider connecting the separate Advanced Visualizer view directly to the live `jam-chord-step` stream as a later polish pass.

## Epic 5: Native Persistence

Goal:

- replace browser-era in-memory and `localStorage` assumptions

Scope:

- MIDI mapping persistence
- saved sparks
- saved lessons and practice state
- app settings

Status: complete

Progress note, 2026-04-16:

- Added native app-state persistence at `%APPDATA%\shed-power\app_state.json`, with cross-platform fallbacks through the Rust persistence layer.
- Persisted MIDI maps and reload them at startup; MIDI learn writes native mappings instead of relying on in-memory state.
- Added native persistence commands and frontend bridge methods for saved sparks, saved lessons, active/completed practice sessions, app settings, recent projects, and module-owned state.
- Replaced Magazine lesson `localStorage` behavior with native saved lessons and practice session state.
- Saved Sparks now persist natively, can be deleted, reloaded into the generator, and sent back into Jam Station.
- Added one-time migration from legacy browser keys: `shed_midi_map`, `theshed_saved`, `theshed_sparks`, and `theshed_active_mixer_spark`.
- Project saves now write a `shed_project.json` manifest beside exported looper WAV/layer files.
- Project loads validate `shed_project.json` before loading audio or recording the path as a recent project.
- Added app-state JSON export/import commands and a shell Projects panel for recent project loading plus backup/restore.
- Added schema validation for app-state imports and recent project records.
- Persisted UI-owned MPC state, including kit, BPM, swing, pad samples, trim/volume/pitch edits, and sequencer steps.
- Persisted UI-owned Jam Station state, including current key/scale/view, sound, enhancement settings, custom chord parts, and the last selected backing/custom/spark track.
- `cargo check` passes after the latest persistence changes.
- `npm run build` passes after the latest persistence changes when run through the approved elevated build path.

Validation note, 2026-04-16:

- Re-inspected native app-state JSON, MIDI maps, saved sparks, saved lessons/practice state, recent projects, and project manifest validation.
- Kept Epic 5 complete. Release-hardening items remain in Epic 8/backlog.

Known technical notes:

- Sandboxed `npm run build` still fails with Node `EPERM` on `C:\Users\Sebastian`; the elevated `npm run build` path is required in this workspace.
- Runtime smoke testing with real MIDI hardware, drag/drop sample paths, and real project save/load folders is still recommended before release hardening.

Follow-up backlog:

- Add native file dialogs instead of prompt-based import/export and project path entry.
- Add a schema migration path when `app_state.json` moves beyond schema version 1.
- Add finer user-facing recovery for missing sample files referenced by persisted MPC pads.

## Epic 6: Magazine and Learning Migration

Goal:

- restore the learning product area from the original HTML app

Status: completed

Scope:

- complete content migration
- richer lesson rendering
- producer and musician volume parity
- practice flows
- challenge flows
- My Shed equivalents

## Epic 7: Missing Legacy Views

Goal:

- reintroduce the remaining product surfaces intentionally, not by re-copying the old monolith

Scope:

- home shell
- tuner
- advanced visualizer
- practice timer
- challenge screens

Status: complete

Progress note, 2026-04-16:

- Home shell restored as a native dashboard view with workstation, learning, creation, saved-state, active-practice, and recent-project entry points.
- Practice and practice timer restored as native Svelte focus surfaces backed by the migrated lesson data.
- My Shed restored as a standalone native view using app persistence for saved lessons, saved sparks, active practice, and completed practice sessions.
- Challenge restored as a native musician/producer random drill deck.
- Tuner restored as a native Svelte view using browser microphone input and Web Audio pitch detection.
- Advanced Visualizer restored as a native theory view with guitar/piano layouts, selectable key/scale, backing-track chord context, chord-tone overlays, and common-tone highlighting.
- Spark-to-Jam and saved Spark handoff flows remain compatible with the restored surfaces.
- `npm run build` passed after the latest Epic 7 slices.

Validation note, 2026-04-16:

- Re-inspected Home, Practice, My Shed, Challenge, Tuner microphone permission/error path, Advanced Visualizer, and Spark/Jam handoff compatibility.
- Kept Epic 7 complete. Live Advanced Visualizer/Jam integration remains Epic 8/polish, not release-critical.

Follow-up backlog:

- Runtime smoke test each restored view in the Tauri shell with real microphone permission, saved persistence data, and Jam handoff.
- Consider connecting the advanced Visualizer to live Jam Station chord-step events in a later polish pass.

## Epic 8: Packaging and Release Hardening

Goal:

- make the app shippable

Scope:

- desktop packaging sanity
- Android path verification
- asset cleanup
- generated-artifact hygiene
- accessibility and UI polish

Status: complete for packaging and release-hardening pass

Closeout note, 2026-04-17:

- Re-inspected `package.json`, `vite.config.js`, `README_BUILD.md`, `build_android.ps1`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `src-tauri/capabilities/default.json`.
- Confirmed elevated free disk space on `C:` at about 125 GB before package builds.
- Aligned release metadata to `1.0.0` across npm, Cargo, Tauri, and Android `tauri.properties`.
- Added generated-artifact hygiene via `.gitignore` for `node_modules/`, `dist/`, Rust target output, Android Gradle/build output, release installers, and local app-state paths.
- Added desktop bundle icon config and verified `npm run tauri build` produces:
  - `src-tauri/target/release/bundle/msi/shed-power_1.0.0_x64_en-US.msi`
  - `src-tauri/target/release/bundle/nsis/shed-power_1.0.0_x64-setup.exe`
- Split the Tauri Rust entrypoint into a mobile-compatible library plus desktop `main.rs`, and renamed the lib target to avoid desktop release PDB name collisions.
- Gated the `midir` backend to non-Android targets and made Android native MIDI initialize as a no-op handler until a true Android MIDI backend is added.
- Updated the Android build path so the helper script can discover the default SDK under `%LOCALAPPDATA%\Android\Sdk` when `ANDROID_HOME` is not already set.
- Verified Android SDK/platform assumptions locally: SDK path exists, Rust Android targets are installed, Android platforms/build-tools are present, and `npm run tauri -- android build` produces:
  - `src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`
  - `src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab`
- Cleaned stale generated JNI symlinks from the earlier Android lib name and verified the generated JNI output now only references `libshed_power_lib.so`.
- Build warning pass: fixed the desktop Cargo PDB collision warning, Android Rust dead-code warnings, generated WebView deprecation warning, and obsolete Java 8 compile target warnings. The Android build still reports Gradle deprecation notices from the generated Gradle/Tauri stack; these do not block the current release artifacts.

Remaining release/runtime checks:

- Desktop real audio-device/MIDI-controller smoke test in the running Tauri app.
- Android device install/signing smoke test. The produced APK is the unsigned universal release artifact.
- Android native MIDI controller support remains backlog because the desktop `midir` backend does not support Android.

## Delivery Order

1. Epic 1: Platform Baseline
2. Epic 2: Looper Parity
3. Epic 4: Jam Station Completion
4. Epic 3: MPC Parity
5. Epic 5: Native Persistence
6. Epic 6: Magazine and Learning Migration
7. Epic 7: Missing Legacy Views
8. Epic 8: Packaging and Release Hardening

## Definition of Done for the Current Sprint

- architecture and epics are documented in-repo
- backend compiles
- frontend still builds
- no known shell-level blocker remains for continued feature work
