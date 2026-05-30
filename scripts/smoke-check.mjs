import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  BACKING_TRACKS,
  getChordNotes,
  getMidiNumber,
  getNoteIndex,
  getScaleNotes,
  NOTES
} from "../src/lib/data.js";
import {
  chordScaleOptions,
  chordToneNames,
  progressionScaleLinks
} from "../src/lib/modalMixture.js";
import {
  GUITAR_VOLUMES,
  MUSICIAN_CHALLENGE_CARDS,
  MUSICIAN_CHALLENGES,
  PRODUCER_CHALLENGE_CARDS,
  PRODUCER_CHALLENGES,
  PRODUCER_VOLUMES
} from "../src/lib/learningData.js";
import {
  LICK_LIBRARY,
  lickToJamTrack,
  stringFretToMidi
} from "../src/lib/lickLibraryData.js";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const logsDir = path.join(root, "logs");
mkdirSync(logsDir, { recursive: true });

const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const logPath = path.join(logsDir, `smoke-${timestamp}.log`);
const results = [];
const lines = [];

function log(line = "") {
  lines.push(line);
  console.log(line);
}

function read(relativePath) {
  return readFileSync(path.join(root, relativePath), "utf8");
}

function exists(relativePath) {
  try {
    statSync(path.join(root, relativePath));
    return true;
  } catch {
    return false;
  }
}

function sizeMb(relativePath) {
  const size = statSync(path.join(root, relativePath)).size / 1024 / 1024;
  return `${size.toFixed(2)} MB`;
}

function latestMtime(relativePaths) {
  let latest = 0;
  const visit = relativePath => {
    const absolutePath = path.join(root, relativePath);
    const info = statSync(absolutePath);
    if (info.isDirectory()) {
      for (const entry of readdirSync(absolutePath, { withFileTypes: true })) {
        visit(path.join(relativePath, entry.name));
      }
      return;
    }
    latest = Math.max(latest, info.mtimeMs);
  };

  relativePaths.forEach(visit);
  return latest;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function warn(name, message) {
  results.push({ status: "WARN", name, message });
  log(`[WARN] ${name}: ${message}`);
}

async function check(name, fn) {
  try {
    const details = await fn();
    results.push({ status: "PASS", name, message: details || "" });
    log(`[PASS] ${name}${details ? `: ${details}` : ""}`);
  } catch (error) {
    const message = error?.message || String(error);
    results.push({ status: "FAIL", name, message });
    log(`[FAIL] ${name}: ${message}`);
  }
}

function parseCargoVersion(cargoToml) {
  const match = cargoToml.match(/^\s*version\s*=\s*"([^"]+)"/m);
  return match?.[1];
}

function parseAndroidVersionName(properties) {
  const match = properties.match(/^tauri\.android\.versionName=(.+)$/m);
  return match?.[1]?.trim();
}

function validateMidiNote(note, context) {
  const match = String(note).match(/^([A-G](?:#|b)?)(-?\d+)$/);
  assert(match, `${context}: invalid note token "${note}"`);
  assert(getNoteIndex(match[1]) !== -1, `${context}: unknown note name "${note}"`);
  const midi = getMidiNumber(note);
  assert(Number.isFinite(midi) && midi >= 0 && midi <= 127, `${context}: MIDI out of range for "${note}"`);
  return midi;
}

function assertContains(source, needle, context) {
  assert(source.includes(needle), `${context} missing "${needle}"`);
}

function assertUnique(values, label) {
  const seen = new Map();
  values.forEach((value, index) => {
    const key = String(value);
    assert(!seen.has(key), `${label} duplicate "${key}" at indexes ${seen.get(key)} and ${index}`);
    seen.set(key, index);
  });
}

function getLearningLessons(volumes) {
  return volumes.flatMap(volume =>
    (volume.musicians || []).flatMap(musician =>
      (musician.lessons || []).map(lesson => ({ volume, musician, lesson }))
    )
  );
}

log("SHED POWER smoke check");
log(`Workspace: ${root}`);
log(`Started: ${new Date().toLocaleString()}`);
log("");

await check("release metadata is aligned", () => {
  const pkg = JSON.parse(read("package.json"));
  const tauri = JSON.parse(read("src-tauri/tauri.conf.json"));
  const cargoVersion = parseCargoVersion(read("src-tauri/Cargo.toml"));
  const androidVersion = parseAndroidVersionName(read("src-tauri/gen/android/app/tauri.properties"));
  assert(pkg.version === "1.0.0", `package.json version is ${pkg.version}`);
  assert(tauri.version === pkg.version, `tauri.conf version ${tauri.version} != package ${pkg.version}`);
  assert(cargoVersion === pkg.version, `Cargo version ${cargoVersion} != package ${pkg.version}`);
  assert(androidVersion === pkg.version, `Android versionName ${androidVersion} != package ${pkg.version}`);
  assert(tauri.identifier === "com.shed.power", `unexpected Tauri identifier ${tauri.identifier}`);
  return `version ${pkg.version}, identifier ${tauri.identifier}`;
});

await check("desktop/mobile build config is present", () => {
  const tauri = JSON.parse(read("src-tauri/tauri.conf.json"));
  const vite = read("vite.config.js");
  const cargo = read("src-tauri/Cargo.toml");
  assert(tauri.bundle?.active === true, "Tauri bundle is not active");
  assert(exists("src-tauri/icons/icon.ico"), "desktop icon.ico missing");
  assert(exists("src-tauri/icons/icon.png"), "Android icon.png missing");
  assertContains(vite, "port: 1420", "Vite dev server");
  assertContains(vite, "strictPort: true", "Vite dev server");
  assertContains(cargo, 'name = "shed_power_lib"', "mobile library target");
  assertContains(cargo, "cfg(not(target_os = \"android\"))", "Android MIDI gating");
  return "Tauri bundle, icons, Vite port, and Android lib target found";
});

await check("Tauri permissions are narrow", () => {
  const capability = JSON.parse(read("src-tauri/capabilities/default.json"));
  const permissions = capability.permissions || [];
  assert(permissions.includes("core:default"), "core default permission missing");
  assert(permissions.includes("shell:allow-open"), "shell open permission missing");
  assert(!permissions.includes("shell:allow-execute"), "shell execute should not be enabled");
  assert(!permissions.includes("shell:allow-spawn"), "shell spawn should not be enabled");
  return permissions.join(", ");
});

await check("all main app routes are wired", () => {
  const app = read("src/App.svelte");
  const routes = {
    home: "HomeShell",
    looper: "Looper",
    synth: "Synth",
    mpc: "MPC",
    jam: "JamStation",
    lickLibrary: "LickLibrary",
    tuner: "Tuner",
    visualizer: "Visualizer",
    magazine: "Magazine",
    practice: "PracticeTimer",
    challenge: "ChallengeDeck",
    myshed: "MyShed",
    spark: "SparkGenerator",
    scales: "ScaleExplorer"
  };

  for (const [route, component] of Object.entries(routes)) {
    assertContains(app, `import ${component}`, `${component} import`);
    assertContains(app, `switchPage("${route}")`, `${route} navigation`);
    assertContains(app, `activeModule === "${route}"`, `${route} render branch`);
    assert(exists(`src/components/${component}.svelte`), `${component}.svelte missing`);
  }

  return `${Object.keys(routes).length} routes/components`;
});

await check("modal-mixture progression practice path is wired", () => {
  const jam = read("src/components/JamStation.svelte");
  const modalMixture = read("src/lib/modalMixture.js");

  assertContains(modalMixture, "export function chordScaleOptions", "local chord-scale discovery export");
  assertContains(modalMixture, "export function progressionScaleLinks", "whole-progression scale discovery export");
  assertContains(jam, "chordScaleOptions(currentChord, rootKey, 8)", "Jam current-chord scale options");
  assertContains(jam, "progressionScaleLinks(activeTrack, rootKey, 8)", "Jam progression-link scale options");
  assertContains(jam, "function addBuilderChord()", "custom chord insertion function");
  assertContains(jam, "tokens.splice(insertAt, 0, token)", "custom chord insertion placement");
  assertContains(jam, "function applyChordScaleOption", "per-chord scale selection");
  assertContains(jam, "scaleChoice", "per-chord scale choice state");
  assertContains(jam, "value.scaleChoices = trackScaleChoices(track)", "preset track scale-choice persistence");
  assertContains(jam, "track: JSON.parse(JSON.stringify(track))", "custom track scale-choice persistence");
  assertContains(jam, "currentVisualizerScale = effectiveScaleForChord", "effective visualizer scale selection");
  assertContains(jam, "visualizerRootKey = currentVisualizerScale.root", "visualizer root follows effective scale");
  assertContains(jam, "visualizerScaleType = currentVisualizerScale.type", "visualizer type follows effective scale");

  const dMinorOptions = chordScaleOptions({ name: "Dm", notes: ["D", "F", "A"] }, "A", 12);
  assert(
    dMinorOptions.some(option => option.root === "A" && option.type === "harmonic_major" && option.exact),
    "Dm in A should include A Harmonic Major as a local scale option"
  );

  const mixolydianLinks = progressionScaleLinks({
    progression: [
      { name: "A", notes: ["A", "C#", "E"] },
      { name: "D", notes: ["D", "F#", "A"] },
      { name: "G", notes: ["G", "B", "D"] }
    ]
  }, "A", 12);
  assert(
    mixolydianLinks.some(option => option.root === "A" && option.type === "mixolydian" && option.exact),
    "A-D-G should include A Mixolydian as an exact progression link"
  );

  const bridgeLinks = progressionScaleLinks({
    progression: [
      { name: "A", notes: ["A", "C#", "E"] },
      { name: "Bb", notes: ["A#", "D", "F"] },
      { name: "E", notes: ["E", "G#", "B"] }
    ]
  }, "A", 12);
  assert(
    bridgeLinks.some(option => !option.exact && option.coverage < 1 && option.missing.length),
    "non-exact progressions should return bridge scales with coverage and missing notes"
  );

  return "custom progression, per-chord choices, visualizer switching, and bridge discovery";
});

await check("recent UI regressions are covered in source", () => {
  const app = read("src/App.svelte");
  const bridge = read("src/lib/audio.js");
  const tuner = read("src/components/Tuner.svelte");
  const mpc = read("src/components/MPC.svelte");
  const jam = read("src/components/JamStation.svelte");
  const theoryVisualizer = read("src/components/TheoryVisualizer.svelte");
  const engine = read("src-tauri/src/audio_engine.rs");
  const sampler = read("src-tauri/src/audio/sampler.rs");

  assertContains(app, "notificationTimeoutMs", "toast auto-dismiss");
  assertContains(app, "pointer-events-none", "toast click-through");
  assertContains(app, "fixed top-4 right-4", "toast position");
  assertContains(tuner, "enumerateDevices", "tuner input device list");
  assertContains(tuner, "createChannelSplitter", "tuner input channel selector");
  assertContains(tuner, "selectedChannel", "tuner selected channel");
  assertContains(mpc, "Laser", "MPC laser pad label");
  assertContains(engine, "mpc_voices", "dedicated MPC voice pool");
  assertContains(engine, "stop_mpc_output", "MPC stop output cleanup");
  assertContains(sampler, "pub fn stop_all", "sampler stop_all");
  assertContains(engine, "v.env.sustain = 0.25", "Jam sustain fix");
  assertContains(engine, "jam_song_advances_and_wraps_by_beat_lengths", "Jam scheduler test");
  assertContains(jam, "beats: playbackBeatLength(chord.beats)", "Jam chord beat payload");
  assertContains(bridge, "browser-web-audio", "browser Jam fallback");
  assertContains(jam, "shed-power:jam-chord-step", "browser Jam visualizer step event");
  assertContains(jam, "jamPaceMode", "Jam practice/original pace mode");
  assertContains(jam, "Math.min(length, 4)", "Jam practice pace caps long chord holds");
  assertContains(jam, "visualizerRevision", "Jam visualizer refresh on chord steps");
  assertContains(theoryVisualizer, "chordAccentClass", "Theory visualizer chord-tone accent");
  return "toast, tuner, MPC stop, and Jam sustain checks found";
});

await check("backing-track music data is valid", () => {
  assert(Array.isArray(NOTES) && NOTES.length === 12, "NOTES must have 12 pitch classes");
  assert(BACKING_TRACKS.length >= 10, `expected at least 10 backing tracks, found ${BACKING_TRACKS.length}`);
  let chordCount = 0;

  for (const track of BACKING_TRACKS) {
    assert(track.id && track.title, "backing track missing id/title");
    assert(Number.isFinite(track.bpm) && track.bpm >= 40 && track.bpm <= 240, `${track.id}: invalid bpm ${track.bpm}`);
    assert(Array.isArray(track.progression) && track.progression.length > 0, `${track.id}: empty progression`);

    for (const [index, chord] of track.progression.entries()) {
      assert(chord.name, `${track.id}[${index}]: missing chord name`);
      assert(Number(chord.beats) > 0, `${track.id}[${index}]: invalid beats ${chord.beats}`);
      assert(Array.isArray(chord.notes) && chord.notes.length >= 3, `${track.id}[${index}]: chord needs at least 3 notes`);
      chord.notes.forEach(note => validateMidiNote(note, `${track.id}[${index}] ${chord.name}`));
      chordCount += 1;
    }
  }

  assert(getScaleNotes("Eb", "minor").length === 7, "flat scale normalization failed");
  assert(getChordNotes("F#m7b5").length >= 4, "m7b5 chord parsing failed");
  return `${BACKING_TRACKS.length} tracks, ${chordCount} chords`;
});

await check("modal mixture practice engine is wired", () => {
  const chord = name => ({ name, notes: getChordNotes(name) });
  const hasScale = (options, root, type) => options.some(option => option.root === root && option.type === type);

  const dmInA = chordScaleOptions(chord("Dm"), "A", 24);
  assert(dmInA.length > 1, "Dm in A should expose multiple compatible chord scales");
  assert(dmInA.every(option => option.exact), "current chord options should fully cover chord tones");
  assert(dmInA.every((option, index) => option.suggested === index + 1), "current chord options missing suggested order");
  assertUnique(dmInA.map(option => option.id), "Dm modal option id");
  assert(hasScale(dmInA, "A", "harmonic_major"), "Dm in A should offer A Harmonic Major");
  assert(hasScale(dmInA, "A", "mixolydian_b6"), "Dm in A should offer A Mixolydian b6");

  const gInA = chordScaleOptions(chord("G"), "A", 24);
  assert(hasScale(gInA, "A", "mixolydian"), "G in A should offer A Mixolydian");
  assert(hasScale(gInA, "A", "dorian"), "G in A should offer A Dorian");
  assert(hasScale(gInA, "A", "minor"), "G in A should offer A Aeolian");

  const cToBb7 = progressionScaleLinks({ progression: [chord("C"), chord("Bb7")] }, "C", 12);
  assert(hasScale(cToBb7, "C", "mixolydian_b6"), "C to Bb7 should link with C Mixolydian b6");
  assert(cToBb7.find(option => option.root === "C" && option.type === "mixolydian_b6")?.exact, "C Mixolydian b6 should fully cover C to Bb7");

  const mixedProgression = progressionScaleLinks({ progression: ["A", "G", "D", "E"].map(chord) }, "A", 8);
  assert(mixedProgression.length > 0, "mixed progression should return bridge scale options");
  assert(mixedProgression.some(option => !option.exact && option.coverage >= 0.85), "mixed progression should show high-coverage bridge scales when no exact link exists");

  const bb7Tones = chordToneNames(chord("Bb7"));
  ["A#", "D", "F", "G#"].forEach(tone => {
    assert(bb7Tones.includes(tone), `Bb7 chord tones should include ${tone}`);
  });

  const jam = read("src/components/JamStation.svelte");
  assertContains(jam, "progressionScaleLinks", "Jam progression link UI");
  assertContains(jam, "applyChordScaleOption", "Jam per-chord scale selection");
  assertContains(jam, "scaleChoice", "Jam per-chord scale persistence");
  assertContains(jam, "jam-chord-step", "Jam playback visualizer step sync");

  return "local chord scales, full-progression links, bridge scales, and Jam UI hooks found";
});

await check("learning and challenge content is loaded", () => {
  assert(PRODUCER_VOLUMES.length === 11, `expected 11 producer volumes, found ${PRODUCER_VOLUMES.length}`);
  assert(GUITAR_VOLUMES.length === 14, `expected 14 musician volumes, found ${GUITAR_VOLUMES.length}`);
  assert(MUSICIAN_CHALLENGES.length === 17, `expected 17 static musician challenges, found ${MUSICIAN_CHALLENGES.length}`);
  assert(PRODUCER_CHALLENGES.length === 13, `expected 13 static producer challenges, found ${PRODUCER_CHALLENGES.length}`);
  assert(MUSICIAN_CHALLENGE_CARDS.length === 36, `expected 36 musician challenge cards, found ${MUSICIAN_CHALLENGE_CARDS.length}`);
  assert(PRODUCER_CHALLENGE_CARDS.length === 17, `expected 17 producer challenge cards, found ${PRODUCER_CHALLENGE_CARDS.length}`);
  assert(PRODUCER_VOLUMES.some(volume => volume.id === "prod_vol5"), "Producer Volume V is missing");
  assert(PRODUCER_VOLUMES.some(volume => volume.id === "prod_utility_belt"), "Producer Utility Belt is missing");

  const producerLessons = getLearningLessons(PRODUCER_VOLUMES);
  const musicianLessons = getLearningLessons(GUITAR_VOLUMES);
  assert(producerLessons.length === 63, `expected 63 producer lessons, found ${producerLessons.length}`);
  assert(musicianLessons.length === 98, `expected 98 musician lessons, found ${musicianLessons.length}`);

  assertUnique(PRODUCER_VOLUMES.map(volume => volume.id), "producer volume id");
  assertUnique(GUITAR_VOLUMES.map(volume => volume.id), "musician volume id");
  assertUnique(producerLessons.map(({ volume, musician, lesson }) => `producers:${volume.id}:${musician.id}:${lesson.id}`), "producer live lesson id");
  assertUnique(musicianLessons.map(({ volume, musician, lesson }) => `musicians:${volume.id}:${musician.id}:${lesson.id}`), "musician live lesson id");
  assertUnique(PRODUCER_CHALLENGE_CARDS.map(challenge => challenge.id), "producer challenge id");
  assertUnique(MUSICIAN_CHALLENGE_CARDS.map(challenge => challenge.id), "musician challenge id");

  return `${producerLessons.length} producer lessons, ${musicianLessons.length} musician lessons, ${PRODUCER_CHALLENGE_CARDS.length + MUSICIAN_CHALLENGE_CARDS.length} challenge cards`;
});

await check("lick library content is loaded", () => {
  assert(LICK_LIBRARY.length === 11, `expected 11 licks, found ${LICK_LIBRARY.length}`);
  assertUnique(LICK_LIBRARY.map(lick => lick.id), "lick id");

  for (const lick of LICK_LIBRARY) {
    assert(lick.name && lick.artist && lick.style && lick.key, `${lick.id}: missing identity fields`);
    assert(Number.isFinite(lick.bpm) && lick.bpm >= 40 && lick.bpm <= 240, `${lick.id}: invalid bpm ${lick.bpm}`);
    assert(Array.isArray(lick.progression) && lick.progression.length > 0, `${lick.id}: empty progression`);
    assert(Array.isArray(lick.sequence) && lick.sequence.length > 0, `${lick.id}: empty note sequence`);
    for (const event of lick.sequence) {
      const midi = stringFretToMidi(event);
      assert(midi === null || (midi >= 0 && midi <= 127), `${lick.id}: event MIDI out of range`);
    }

    const track = lickToJamTrack(lick);
    assert(track?.source === "lick-library", `${lick.id}: missing Jam handoff source`);
    assert(track.progression.length === lick.progression.length, `${lick.id}: handoff progression length mismatch`);
  }

  return `${LICK_LIBRARY.length} licks with Jam handoff data`;
});

await check("native audio command bridge covers major modules", () => {
  const bridge = read("src/lib/audio.js");
  const commands = read("src-tauri/src/audio/commands.rs");
  const engine = read("src-tauri/src/audio_engine.rs");
  const requiredCommands = [
    "Play",
    "Stop",
    "Record",
    "Overdub",
    "Undo",
    "ClearPart",
    "SetMpcKit",
    "StartMpcSequencer",
    "StopMpcSequencer",
    "PlayJamTrack",
    "PlayCustomSong",
    "StopChord",
    "SaveProject",
    "LoadProject",
    "SetInputMonitoring"
  ];

  for (const command of requiredCommands) {
    assertContains(commands, command, `AudioCommand ${command}`);
    assertContains(engine, `AudioCommand::${command}`, `engine handler ${command}`);
  }

  ["playJamTrack", "playCustomSong", "stopMpcSequencer", "loadSample", "saveProject", "loadProject"].forEach(method => {
    assertContains(bridge, `${method}: async`, `frontend bridge ${method}`);
  });

  return `${requiredCommands.length} command handlers plus frontend bridge methods`;
});

await check("release artifacts exist", () => {
  const artifacts = [
    "src-tauri/target/release/bundle/msi/shed-power_1.0.0_x64_en-US.msi",
    "src-tauri/target/release/bundle/nsis/shed-power_1.0.0_x64-setup.exe",
    "src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk",
    "src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab"
  ];

  const sharedSourceTime = latestMtime([
    "package.json",
    "package-lock.json",
    "vite.config.js",
    "src",
    "src-tauri/Cargo.toml",
    "src-tauri/tauri.conf.json",
    "src-tauri/capabilities",
    "src-tauri/src"
  ]);
  const androidSourceTime = latestMtime([
    "package.json",
    "package-lock.json",
    "vite.config.js",
    "src",
    "src-tauri/Cargo.toml",
    "src-tauri/tauri.conf.json",
    "src-tauri/capabilities",
    "src-tauri/src",
    "src-tauri/gen/android/app/tauri.properties",
    "src-tauri/gen/android/app/build.gradle.kts",
    "src-tauri/gen/android/app/src/main/AndroidManifest.xml"
  ]);
  const details = [];
  for (const artifact of artifacts) {
    assert(exists(artifact), `${artifact} missing`);
    const info = statSync(path.join(root, artifact));
    assert(info.size > 1024 * 1024, `${artifact} is unexpectedly small`);
    details.push(`${artifact} (${sizeMb(artifact)})`);
    const freshnessCutoff = artifact.includes("gen/android") ? androidSourceTime : sharedSourceTime;
    if (info.mtimeMs < freshnessCutoff) {
      warn("release artifact freshness", `${artifact} is older than current app/native source; rebuild it before publishing`);
    }
  }

  return details.join("; ");
});

await check("known release caveats are documented", () => {
  const readme = read("README_BUILD.md");
  const epics = read("EPICS_PLAN.md");
  assertContains(readme, "unsigned", "Android unsigned release note");
  assertContains(readme, "Android native MIDI is currently disabled", "Android MIDI caveat");
  assertContains(epics, "Desktop real audio-device/MIDI-controller smoke test", "desktop hardware caveat");
  assertContains(epics, "Android device install/signing smoke test", "Android device caveat");
  return "manual hardware/signing caveats present";
});

const passCount = results.filter(result => result.status === "PASS").length;
const warnCount = results.filter(result => result.status === "WARN").length;
const failCount = results.filter(result => result.status === "FAIL").length;

log("");
log(`Summary: ${passCount} passed, ${warnCount} warnings, ${failCount} failed`);
log(`Log file: ${logPath}`);

writeFileSync(logPath, `${lines.join("\n")}\n`, "utf8");

if (failCount > 0) {
  process.exitCode = 1;
}
