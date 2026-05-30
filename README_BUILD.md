# SHED POWER - Build Instructions

Professional harmony and bass engine for the native desktop app and Android.

VST3/Reaper plugin output is not currently implemented or releaseable from this repository.

## Native Desktop App Build

### Requirements

- Rust stable
- Node.js and npm
- Tauri v2 CLI from the project dependency (`npm run tauri -- ...`)
- Several GB of free space on `C:` for Rust release artifacts and temporary files

### Verification Commands

```powershell
cd "C:\Users\Sebastian\Desktop\GoogleApps\music magazine\Power\src-tauri"
cargo check

cd "C:\Users\Sebastian\Desktop\GoogleApps\music magazine\Power"
npm run build
npm run tauri build
```

In this workspace, `npm run build` and package builds should be run elevated. Sandboxed builds can fail with Windows `EPERM` under `C:\Users\Sebastian`.

### Release Checklist

- `npm run tauri build` was verified on 2026-04-17 after freeing disk space on `C:`.
- Desktop artifacts:
  - `src-tauri\target\release\bundle\msi\shed-power_1.0.0_x64_en-US.msi`
  - `src-tauri\target\release\bundle\nsis\shed-power_1.0.0_x64-setup.exe`
- Release versions are aligned at `1.0.0` across `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and `src-tauri/gen/android/app/tauri.properties`.
- Generated artifact hygiene is covered by `.gitignore` for `node_modules/`, `dist/`, Rust target output, Android Gradle/build output, release installers, and local app-state paths.
- Runtime smoke remains required before public release: desktop audio device, desktop MIDI controller, Android device install, and Android audio session.
- Current release blockers and caveats are tracked in [RELEASE_STATUS.md](RELEASE_STATUS.md).

## VST3 Status

VST3 is out of scope for the current release.

This repository does not define a `shed_power_vst3` Cargo bin target, a `vst3` Cargo feature, or VST3 bundle packaging. There is also no current `src-tauri/src/vst3_plugin.rs` wrapper file.

`src-tauri/build_vst3.ps1` is retained as a compatibility/status entry point only. It fails fast with an explicit message and does not attempt a broken build or install anything into VST3 plugin directories.

Do not advertise Reaper/VST3 installation, scanning, or MIDI testing as part of this release.

## Android APK Build

### Requirements

- Rust stable
- Node.js 16+ and npm
- Java Development Kit (JDK) 17+
- Android SDK with build tools
- `ANDROID_HOME` environment variable set, or the SDK installed at `%LOCALAPPDATA%\Android\Sdk`

### Setup Android Development

```powershell
# Install Android SDK if it is not already installed.
# Download from: https://developer.android.com/studio

$env:ANDROID_HOME = "C:\Users\$env:USERNAME\AppData\Local\Android\Sdk"

sdkmanager "platform-tools" "platforms;android-33" "build-tools;33.0.0"
```

### Build Commands

```powershell
cd "C:\Users\Sebastian\Desktop\GoogleApps\music magazine\Power"

.\build_android.ps1

# Direct Tauri Android build used for release verification.
$env:ANDROID_HOME = "C:\Users\$env:USERNAME\AppData\Local\Android\Sdk"
npm run tauri -- android build
```

The 2026-04-17 release-hardening pass produced:

- `src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-unsigned.apk`
- `src-tauri\gen\android\app\build\outputs\bundle\universalRelease\app-universal-release.aab`

The APK is unsigned and must be signed or installed through an appropriate debug/release workflow before distribution.

### Installing APK

```bash
adb install "src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-unsigned.apk"
```

### Android Notes

- Touch-optimized UI: Svelte components adapted for mobile.
- Core audio engine: uses the same harmony and bass algorithms as the desktop app.
- MIDI caveat: Android native MIDI is currently disabled in the Rust backend because the desktop `midir` backend does not support Android. USB/controller MIDI on Android needs a dedicated Android MIDI backend.
- Runtime smoke remains required on a physical Android device before distribution.

## Engine Features

### Harmony Engine

- Close Position: traditional close harmony voicing.
- Open Position: contemporary open voicing with wider intervals.
- Drop-2: jazz voicing with dropped second voice.
- Quartal: modern quartal harmony using fourth intervals.
- Extensions: advanced harmony with 9ths, 11ths, and 13ths.

### Bass Engine

- Root Pattern: simple root notes on downbeats.
- Octave Pattern: alternating root and octave on beats 1 and 3.
- Walking: quarter-note bass lines with chord-tone movement.
- Rhythmic: syncopated patterns with 16th-note subdivisions.

### Audio Behavior

- Voice leading: smooth transitions between chords.
- Sample-accurate timing: precise synchronization.
- Anti-aliasing: PolyBLEP oscillators for clean sound.
- Humanization: subtle timing variations for natural feel.

## Troubleshooting

### VST3

- VST3 is not implemented. `src-tauri/build_vst3.ps1` exits immediately with the current status and should not produce a `.vst3` bundle.

### Android

- Build fails: ensure `ANDROID_HOME` is set correctly.
- APK will not install: verify signing/install workflow and Android install permissions.
- No audio: check Android audio permissions and run a physical-device smoke test.

### Common Solutions

```powershell
rustup update
cargo clean
```

## Project Structure

```text
Power/
+-- src/                     # Svelte frontend
|   +-- components/          # UI components
|   +-- lib/                 # Data and utilities
+-- src-tauri/               # Rust backend
|   +-- src/audio/           # Audio engine modules
|   +-- src/audio_engine.rs  # Main audio engine
|   +-- src/lib.rs           # Tauri command/backend entry points
|   +-- Cargo.toml           # Rust package manifest
+-- build_android.ps1        # Android build script
+-- src-tauri/build_vst3.ps1 # VST3 status/fail-fast stub
+-- README_BUILD.md          # Build instructions
+-- RELEASE_STATUS.md        # Current release blockers and caveats
```

Build the native desktop app and Android artifacts from the documented paths above. VST3 is not part of the current release.
