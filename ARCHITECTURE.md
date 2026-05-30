# SHED POWER Architecture

## Context Clear

SHED POWER is no longer a single browser app. It is now a native desktop application with:

- Svelte UI in `src/`
- Tauri command and event bridge in `src-tauri/src/main.rs`
- Rust audio engine and device layer in `src-tauri/src/`
- Shared content and theory data in `src/lib/data.js`

The original HTML app in `C:\Users\Sebastian\Desktop\GoogleApps\music magazine\index.html` still matters because it remains the source of truth for:

- feature parity
- missing workflows
- educational content
- view inventory outside the workstation shell

The current codebase already split the workstation into focused modules, but migration is incomplete. The architecture plan below treats the current Svelte/Tauri app as the production target and the original HTML app as the parity reference.

## System Layers

### 1. UI Shell

Files:

- `src/App.svelte`
- `src/components/*.svelte`
- `src/lib/stores.js`

Responsibilities:

- route between workstation modules
- collect user intent
- display Rust engine state
- manage short-lived UI state
- avoid direct DSP logic

Rules:

- components talk to the backend only through `src/lib/audio.js`
- global state stays minimal
- workstation pages own their local UI state

### 2. Frontend Bridge

Files:

- `src/lib/audio.js`

Responsibilities:

- expose a stable JS API for the UI
- translate UI actions into Tauri `invoke` calls
- hide backend command shapes from components

Rules:

- no component should build raw command JSON itself
- bridge names should stay task-oriented, not backend-oriented

### 3. Tauri App Layer

Files:

- `src-tauri/src/main.rs`
- `src-tauri/src/events.rs`

Responsibilities:

- own app lifecycle
- register Tauri commands
- translate UI requests into audio-thread commands
- emit UI-facing events
- perform file I/O and device scans off the audio thread

Rules:

- no blocking file or device work on the audio callback
- commands should be thin and deterministic
- event payloads should be UI-friendly

### 4. Audio Domain Layer

Files:

- `src-tauri/src/audio_engine.rs`
- `src-tauri/src/audio/*`

Responsibilities:

- real-time DSP
- looper state and playback
- synth voices and modulation
- drum and sampler playback
- jam backing logic
- MIDI handling and device routing

Rules:

- real-time safe code path
- no unnecessary allocation in the audio loop
- explicit command and event boundaries

### 5. Content Layer

Files:

- `src/lib/data.js`
- legacy content sources in the parent `music magazine` directory

Responsibilities:

- backing tracks
- scales and note helpers
- magazine volumes and lessons
- spark generator data

Rules:

- content must be source-controlled and structured
- legacy HTML remains the migration reference until parity is complete

## Runtime Data Flow

1. User interacts with a Svelte component.
2. Component calls a method from `src/lib/audio.js`.
3. Bridge sends a Tauri command or command JSON.
4. `main.rs` validates and forwards work to the audio engine or background worker.
5. Rust emits state updates back to the UI as Tauri events.
6. Components listen for those events and update the view.

This keeps DSP and native I/O out of the browser layer while preserving a reactive UI.

## Module Ownership

### App Shell

- top navigation
- device status
- global shortcuts
- save/load entry points
- MIDI learn mode shell state

### Looper

- part selection
- transport
- waveform display
- song sequence UI
- metronome and looper mix controls

### Synth

- macro parameter control
- preset selection
- MIDI learn overlays

### MPC

- pad triggering
- kit selection
- sequencer grid
- sample import workflow

### Jam Station

- backing track playback
- harmonic overlays
- bassline and harmony options
- custom song workflow

### Magazine

- educational content browser
- volume and lesson navigation
- future saved practice hooks

### Spark Generator

- idea generation
- handoff into Jam Station
- future save-to-library support

## Current Gaps

### Stable but Incomplete

- workstation shell exists
- core audio bridge exists
- content browser exists in reduced form

### Missing or Partial

- non-workstation views from the original HTML app
- persisted user state such as saved lessons, saved sparks, and MIDI maps
- full Spark to Jam handoff
- custom song playback in Rust
- parity for tuner and advanced visualizer views

## Target Architecture Direction

The app should evolve into three clear product areas under one shell:

### 1. Workstation

- looper
- synth
- MPC
- jam station

### 2. Learning

- magazine
- practice sessions
- saved drills
- challenge flows

### 3. Creation Support

- spark generator
- saved sparks
- track handoff into jam and workstation modules

The original HTML app mixed all three concerns into one giant file. The native app should keep them isolated while sharing common navigation, persistence, and event contracts.

## Persistence Strategy

Native persistence should replace browser `localStorage` patterns.

Recommended targets:

- project audio data: filesystem WAV snapshots
- user settings and saved content: JSON or TOML app data files
- MIDI mappings: native config file, not in-memory only

## Immediate Architectural Priorities

1. Make the current native app compile and run cleanly.
2. Stabilize the workstation shell and core commands.
3. Finish parity for the current visible modules before restoring missing legacy views.
4. Add native persistence for user state that was previously stored in `localStorage`.
5. Only then port the remaining HTML-only screens and workflows.
