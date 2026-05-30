// SHED POWER: Frontend <-> Rust Audio Bridge
// This file wraps the Tauri 'invoke' commands into a clean JS API.

import { invoke } from '@tauri-apps/api/core';

const LOOPER_SOURCE_PAYLOADS = {
    "input:1": { InputChannel: 1 },
    "input:2": { InputChannel: 2 },
    "input:3": { InputChannel: 3 },
    "input:4": { InputChannel: 4 },
    "input:5": { InputChannel: 5 },
    "input:6": { InputChannel: 6 },
    "input:7": { InputChannel: 7 },
    "input:8": { InputChannel: 8 },
    "mix": "InputMix",
    "input_mix": "InputMix",
    "InputMix": "InputMix",
    "synth": "Synth",
    "Synth": "Synth",
    "mpc": "Mpc",
    "MPC": "Mpc",
    "Mpc": "Mpc",
    "jam": "Jam",
    "Jam": "Jam",
    "instrument": "InstrumentMix",
    "instrument_mix": "InstrumentMix",
    "InstrumentMix": "InstrumentMix",
    "silent": "Silent",
    "Silent": "Silent"
};

function normalizeLooperSource(source) {
    if (typeof source === "number") {
        const channel = u8Number(source, "LooperSource InputChannel");
        return channel === 0 ? "InputMix" : { InputChannel: channel };
    }
    if (typeof source === "string") {
        return LOOPER_SOURCE_PAYLOADS[source] ?? "Silent";
    }
    if (source && typeof source === "object" && "InputChannel" in source) {
        return { InputChannel: u8Number(source.InputChannel, "LooperSource InputChannel") };
    }
    return source ?? "Silent";
}

function finiteNumber(value, label) {
    const number = Number(value);
    if (!Number.isFinite(number)) {
        throw new TypeError(`[Audio] ${label} must be a finite number, got ${String(value)}`);
    }
    return number;
}

function integerNumber(value, label) {
    const number = finiteNumber(value, label);
    if (!Number.isInteger(number)) {
        throw new TypeError(`[Audio] ${label} must be an integer, got ${String(value)}`);
    }
    return number;
}

function nonNegativeInteger(value, label) {
    const number = integerNumber(value, label);
    if (number < 0) {
        throw new RangeError(`[Audio] ${label} must be non-negative, got ${String(value)}`);
    }
    return number;
}

function u8Number(value, label) {
    const number = nonNegativeInteger(value, label);
    if (number > 255) {
        throw new RangeError(`[Audio] ${label} must be <= 255, got ${String(value)}`);
    }
    return number;
}

function normalizeNotes(notes, label) {
    if (!Array.isArray(notes)) {
        throw new TypeError(`[Audio] ${label} must be an array`);
    }
    return notes.map((note, index) => u8Number(note, `${label}[${index}]`));
}

function normalizeIntegerArray(values, label) {
    if (!Array.isArray(values)) {
        throw new TypeError(`[Audio] ${label} must be an array`);
    }
    return values.map((value, index) => nonNegativeInteger(value, `${label}[${index}]`));
}

function normalizeStringArray(values, label) {
    if (!Array.isArray(values)) {
        throw new TypeError(`[Audio] ${label} must be an array`);
    }
    return values.map(String);
}

function normalizeJamChords(chords) {
    if (!Array.isArray(chords)) {
        throw new TypeError("[Audio] PlayJamTrack chords must be an array");
    }
    return chords.map((chord, index) => {
        if (!chord || typeof chord !== "object") {
            throw new TypeError(`[Audio] PlayJamTrack chords[${index}] must be an object`);
        }
        const normalized = {
            notes: normalizeNotes(chord.notes, `PlayJamTrack chords[${index}].notes`),
            beats: nonNegativeInteger(chord.beats ?? 4, `PlayJamTrack chords[${index}].beats`)
        };
        if (chord.name != null) {
            normalized.name = String(chord.name);
        }
        return normalized;
    });
}

let browserAudioContext = null;
let browserJamState = {
    timers: [],
    voices: [],
    soundPreset: 0
};

function hasTauriRuntime() {
    return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

function getBrowserAudioContext() {
    if (typeof window === "undefined") return null;
    const AudioContextCtor = window.AudioContext || window.webkitAudioContext;
    if (!AudioContextCtor) return null;
    if (!browserAudioContext) {
        browserAudioContext = new AudioContextCtor();
    }
    return browserAudioContext;
}

function stopBrowserVoices() {
    for (const timer of browserJamState.timers) {
        clearTimeout(timer);
    }
    browserJamState.timers = [];

    for (const voice of browserJamState.voices) {
        try {
            voice.gain.gain.cancelScheduledValues(0);
            voice.gain.gain.setTargetAtTime(0.0001, browserAudioContext?.currentTime || 0, 0.015);
            voice.oscillator.stop((browserAudioContext?.currentTime || 0) + 0.05);
        } catch {
            // Voice may have already stopped naturally.
        }
    }
    browserJamState.voices = [];
}

function midiToFrequency(midi) {
    return 440 * Math.pow(2, (midi - 69) / 12);
}

function browserWaveform() {
    return ["sine", "triangle", "sawtooth", "square"][browserJamState.soundPreset % 4] || "triangle";
}

function emitBrowserJamStep(index, chord) {
    if (typeof window === "undefined") return;
    window.dispatchEvent(new CustomEvent("shed-power:jam-chord-step", {
        detail: {
            index,
            notes: chord.notes.map(note => note % 12),
            label: chord.name || ""
        }
    }));
}

async function playBrowserJamTrack(chords, tempo) {
    const context = getBrowserAudioContext();
    if (!context) {
        throw new Error("Browser audio is not available in this environment.");
    }

    await context.resume();
    stopBrowserVoices();

    const beatMs = 60000 / Math.max(1, tempo);
    const waveform = browserWaveform();

    const scheduleChord = index => {
        const chord = chords[index % chords.length];
        const durationMs = Math.max(250, chord.beats * beatMs);
        const now = context.currentTime;
        const stopAt = now + durationMs / 1000;

        emitBrowserJamStep(index % chords.length, chord);

        chord.notes.forEach((midi, noteIndex) => {
            const oscillator = context.createOscillator();
            const gain = context.createGain();
            oscillator.type = waveform;
            oscillator.frequency.setValueAtTime(midiToFrequency(midi), now);
            gain.gain.setValueAtTime(0.0001, now);
            gain.gain.exponentialRampToValueAtTime(noteIndex === 0 ? 0.09 : 0.045, now + 0.02);
            gain.gain.setTargetAtTime(0.018, Math.max(now + 0.06, stopAt - 0.18), 0.06);
            gain.gain.setTargetAtTime(0.0001, Math.max(now + 0.08, stopAt - 0.06), 0.025);
            oscillator.connect(gain);
            gain.connect(context.destination);
            oscillator.start(now);
            oscillator.stop(stopAt + 0.08);
            browserJamState.voices.push({ oscillator, gain });
        });

        const timer = setTimeout(() => scheduleChord(index + 1), durationMs);
        browserJamState.timers.push(timer);
    };

    scheduleChord(0);
    return { ok: true, runtime: "browser-web-audio" };
}

async function browserAudioCommand(command) {
    const [name, payload] = Object.entries(command || {})[0] || [];
    switch (name) {
        case "PlayJamTrack":
            return playBrowserJamTrack(payload.chords, payload.tempo);
        case "PlayChord":
            return playBrowserJamTrack([{ notes: payload.notes, beats: 1, name: "" }], payload.tempo);
        case "StopChord":
        case "Stop":
        case "StopAllSounds":
            stopBrowserVoices();
            return { ok: true, runtime: "browser-web-audio" };
        case "SetJamSound":
            browserJamState.soundPreset = payload.preset_id;
            return { ok: true, runtime: "browser-web-audio" };
        default:
            return { ok: true, runtime: "browser-noop" };
    }
}

async function sendCommand(command) {
    if (!hasTauriRuntime()) {
        return browserAudioCommand(command);
    }

    try {
        return await invoke('send_audio_command', { command: JSON.stringify(command) });
    } catch (error) {
        console.error("[Audio] Command failed:", command, error);
        throw error;
    }
}

export const Audio = {
    // --- TRANSPORT ---
    play: async () => {
        console.log("[Audio] Play");
        return sendCommand({ Play: null });
    },

    stop: async () => {
        console.log("[Audio] Stop");
        return sendCommand({ Stop: null });
    },

    stopAllSounds: async () => {
        console.log("[Audio] Stop All Sounds");
        return sendCommand({ StopAllSounds: null });
    },

    // --- LOOPER ---
    // partId: 0 (A), 1 (B), 2 (C), 3 (D), 4 (E)
    record: async (partId) => {
        console.log(`[Audio] Record Part ${partId}`);
        return sendCommand({ Record: { part_id: nonNegativeInteger(partId, "Record part_id") } });
    },

    overdub: async (partId) => {
        console.log(`[Audio] Overdub Part ${partId}`);
        return sendCommand({ Overdub: { part_id: nonNegativeInteger(partId, "Overdub part_id") } });
    },

    undo: async (partId) => {
        console.log(`[Audio] Undo Part ${partId}`);
        return sendCommand({ Undo: { part_id: nonNegativeInteger(partId, "Undo part_id") } });
    },

    clearPart: async (partId) => {
        console.log(`[Audio] Clear Part ${partId}`);
        return sendCommand({ ClearPart: { part_id: nonNegativeInteger(partId, "ClearPart part_id") } });
    },

    selectPart: async (partId) => {
        return sendCommand({ SelectPart: { part_id: nonNegativeInteger(partId, "SelectPart part_id") } });
    },

    toggleLooper: async (partId) => {
        console.log(`[Audio] Toggle Looper Part ${partId}`);
        return sendCommand({ ToggleLooper: { part_id: nonNegativeInteger(partId, "ToggleLooper part_id") } });
    },

    // --- SYNTH ---
    noteOn: async (note, velocity = 110) => {
        return sendCommand({
            NoteOn: {
                note: u8Number(note, "NoteOn note"),
                velocity: u8Number(velocity, "NoteOn velocity")
            }
        });
    },

    noteOff: async (note) => {
        return sendCommand({ NoteOff: { note: u8Number(note, "NoteOff note") } });
    },

    synthNoteOn: async (note, velocity = 110) => {
        return Audio.noteOn(note, velocity);
    },

    synthNoteOff: async (note) => {
        return Audio.noteOff(note);
    },

    // paramId: 0-7 (The 8 Knobs)
    setParam: async (paramId, value) => {
        return sendCommand({
            SetParam: {
                id: u8Number(paramId, "SetParam id"),
                value: finiteNumber(value, "SetParam value")
            }
        });
    },

    // --- MPC ---
    setMpcKit: async (kit) => {
        return sendCommand({ SetMpcKit: { kit: u8Number(kit, "SetMpcKit kit") } });
    },

    setMpcParam: async (paramId, value) => {
        return sendCommand({
            SetMpcParam: {
                id: u8Number(paramId, "SetMpcParam id"),
                value: finiteNumber(value, "SetMpcParam value")
            }
        });
    },

    setMpcStep: async (padId, step, active) => {
        return sendCommand({
            SetMpcStep: {
                pad_id: nonNegativeInteger(padId, "SetMpcStep pad_id"),
                step: nonNegativeInteger(step, "SetMpcStep step"),
                active: Boolean(active)
            }
        });
    },

    setMpcSampleTrim: async (padId, start, end, volume = 1, pitch = 0) => {
        return sendCommand({
            SetMpcSampleTrim: {
                pad_id: nonNegativeInteger(padId, "SetMpcSampleTrim pad_id"),
                start: finiteNumber(start, "SetMpcSampleTrim start"),
                end: finiteNumber(end, "SetMpcSampleTrim end"),
                volume: finiteNumber(volume, "SetMpcSampleTrim volume"),
                pitch: finiteNumber(pitch, "SetMpcSampleTrim pitch")
            }
        });
    },

    startMpcSequencer: async (bpm, swing) => {
        return sendCommand({
            StartMpcSequencer: {
                bpm: finiteNumber(bpm, "StartMpcSequencer bpm"),
                swing: finiteNumber(swing, "StartMpcSequencer swing")
            }
        });
    },

    stopMpcSequencer: async () => {
        return sendCommand({ StopMpcSequencer: null });
    },

    mpcPadOn: async (padId, velocity = 100) => {
        return Audio.noteOn(36 + nonNegativeInteger(padId, "mpcPadOn pad_id"), velocity);
    },

    mpcPadOff: async (padId) => {
        return Audio.noteOff(36 + nonNegativeInteger(padId, "mpcPadOff pad_id"));
    },

    // --- JAM STATION ---
    playChord: async (notes, tempo = 120) => {
        // Convert note names (C3) to MIDI numbers if needed, but Rust expects u8
        // Assuming frontend sends MIDI numbers [60, 64, 67]
        return sendCommand({
            PlayChord: {
                notes: normalizeNotes(notes, "PlayChord notes"),
                tempo: finiteNumber(tempo, "PlayChord tempo")
            }
        });
    },

    stopChord: async () => {
        return sendCommand({ StopChord: null });
    },

    setJamSound: async (id) => {
        return sendCommand({ SetJamSound: { preset_id: u8Number(id, "SetJamSound preset_id") } });
    },

    playJamTrack: async (chords, tempo) => {
        return sendCommand({
            PlayJamTrack: {
                chords: normalizeJamChords(chords),
                tempo: finiteNumber(tempo, "PlayJamTrack tempo")
            }
        });
    },

    // Separate parameter control for jam station
    setJamParam: async (paramId, value) => {
        return sendCommand({
            SetJamParam: {
                id: u8Number(paramId, "SetJamParam id"),
                value: finiteNumber(value, "SetJamParam value")
            }
        });
    },

    // Enhanced Jam Station Features
    setBasslineEnabled: async (enabled) => {
        return sendCommand({ SetBasslineEnabled: { enabled: Boolean(enabled) } });
    },

    setHarmonicsEnabled: async (enabled) => {
        return sendCommand({ SetHarmonicsEnabled: { enabled: Boolean(enabled) } });
    },

    setBasslineStyle: async (style) => {
        // 0=Root, 1=Octave, 2=Walking, 3=Rhythmic
        return sendCommand({ SetBasslineStyle: { style: u8Number(style, "SetBasslineStyle style") } });
    },

    setBasslinePreset: async (presetId) => {
        // 0=Upright, 1=Electric, 2=Synth
        return sendCommand({ SetBasslinePreset: { preset_id: u8Number(presetId, "SetBasslinePreset preset_id") } });
    },

    setHarmonicsPreset: async (presetId) => {
        // 0=Fifth, 1=Octave, 2=Major Third, 3=Minor Third
        return sendCommand({ SetHarmonicsPreset: { preset_id: u8Number(presetId, "SetHarmonicsPreset preset_id") } });
    },

    playCustomSong: async (parts, tempo) => {
        return sendCommand({
            PlayCustomSong: {
                parts: normalizeStringArray(parts, "PlayCustomSong parts"),
                tempo: finiteNumber(tempo, "PlayCustomSong tempo")
            }
        });
    },

    // --- SYSTEM ---
    setMicGain: async (gain) => {
        return sendCommand({ SetMicGain: { gain: finiteNumber(gain, "SetMicGain gain") } });
    },

    setNativeInputChannel: async (channel) => {
        return sendCommand({ SetNativeInputChannel: { channel: u8Number(channel, "SetNativeInputChannel channel") } });
    },

    setLooperPartInput: async (partId, channel) => {
        return sendCommand({
            SetLooperPartInput: {
                part_id: nonNegativeInteger(partId, "SetLooperPartInput part_id"),
                channel: u8Number(channel, "SetLooperPartInput channel")
            }
        });
    },

    setLooperPartInputChannel: async (partId, channel) => {
        return Audio.setLooperPartInput(partId, channel);
    },

    setLooperPartSource: async (partId, source) => {
        return sendCommand({
            SetLooperPartSource: {
                part_id: nonNegativeInteger(partId, "SetLooperPartSource part_id"),
                source: normalizeLooperSource(source)
            }
        });
    },

    setLooperFx: async (partId, effectId, value) => {
        return sendCommand({
            SetLooperFx: {
                part_id: nonNegativeInteger(partId, "SetLooperFx part_id"),
                effect_id: u8Number(effectId, "SetLooperFx effect_id"),
                value: finiteNumber(value, "SetLooperFx value")
            }
        });
    },

    toggleLooperPartActive: async (partId) => {
        return sendCommand({ ToggleLooperPartActive: { part_id: nonNegativeInteger(partId, "ToggleLooperPartActive part_id") } });
    },

    setInputState: async (micActive, midiActive) => {
        await sendCommand({ SetMicActive: { active: Boolean(micActive) } });
        return await invoke('set_midi_active', { active: Boolean(midiActive) });
    },

    setMetronome: async (enabled, bpm) => {
        return sendCommand({
            SetMetronome: {
                enabled: Boolean(enabled),
                bpm: finiteNumber(bpm, "SetMetronome bpm")
            }
        });
    },

    // --- SONG SEQUENCE ---
    playSequence: async (parts) => {
        console.log(`[Audio] Play Sequence: ${JSON.stringify(parts)}`);
        return sendCommand({
            PlaySequence: {
                parts: normalizeIntegerArray(parts, "PlaySequence parts")
            }
        });
    },

    stopSequence: async () => {
        console.log("[Audio] Stop Sequence");
        return sendCommand({ StopSequence: null });
    },

    // --- INPUT MONITORING ---
    setInputMonitoring: async (active) => {
        console.log(`[Audio] Input Monitoring: ${active}`);
        return sendCommand({ SetInputMonitoring: { active: Boolean(active) } });
    },

    // --- FILE IO ---
    saveProject: async (path) => {
        console.log(`[Audio] Saving to ${path}...`);
        return sendCommand({ SaveProject: { path: String(path) } });
    },

    loadProject: async (path) => {
        console.log(`[Audio] Loading from ${path}...`);
        await invoke('load_project', { path });
    },

    loadSample: async (padId, path) => {
        console.log(`[Audio] Loading Sample to Pad ${padId}: ${path}`);
        await invoke('load_sample', { padId, path });
    },

    // --- MIDI ---
    learnMidi: async (type, id) => {
        console.log(`[Audio] Learning MIDI for ${type} ${id}`);
        await invoke('start_midi_learn', { targetType: type, id: id });
    },

    setContext: async (name) => {
        await invoke('set_midi_context', { ctx: name });
    },

    // --- DEVICE MANAGEMENT ---
    scanDevices: async () => {
        console.log("[Audio] Scanning devices...");
        try {
            const devices = await invoke('scan_devices');
            console.log("[Audio] Found devices:", devices);
            return devices;
        } catch (e) {
            console.error("[Audio] Device scan failed:", e);
            return [];
        }
    },

    refreshMidi: async () => {
        console.log("[Audio] Refreshing MIDI...");
        try {
            const result = await invoke('refresh_midi');
            console.log("[Audio] MIDI refresh:", result);
            return result;
        } catch (e) {
            console.error("[Audio] MIDI refresh failed:", e);
            throw e;
        }
    },

    refreshAudio: async () => {
        console.log("[Audio] Refreshing audio...");
        try {
            const result = await invoke('refresh_audio');
            console.log("[Audio] Audio refresh:", result);
            return result;
        } catch (e) {
            console.error("[Audio] Audio refresh failed:", e);
            throw e;
        }
    },

    // --- NATIVE PERSISTENCE ---
    loadPersistence: async () => {
        return await invoke('get_persistence');
    },

    getPersistencePath: async () => {
        return await invoke('get_persistence_path');
    },

    exportPersistence: async (path) => {
        return await invoke('export_persistence', { path });
    },

    importPersistenceFile: async (path) => {
        return await invoke('import_persistence_file', { path });
    },

    saveAppSettings: async (settings) => {
        return await invoke('save_app_settings', { settings });
    },

    saveModuleState: async (module, state) => {
        return await invoke('save_module_state', { module, state });
    },

    saveSpark: async (spark) => {
        return await invoke('save_spark', { spark });
    },

    deleteSpark: async (id) => {
        return await invoke('delete_spark', { id });
    },

    saveLesson: async (lesson) => {
        return await invoke('save_lesson', { lesson });
    },

    deleteLesson: async (id) => {
        return await invoke('delete_lesson', { id });
    },

    startPracticeSession: async (lesson, startedAt) => {
        return await invoke('start_practice_session', { lesson, startedAt });
    },

    finishPracticeSession: async (lessonId, completedAt) => {
        return await invoke('finish_practice_session', { lessonId, completedAt });
    },

    importLegacyBrowserState: async (payload) => {
        return await invoke('import_legacy_browser_state', { payload });
    }
};
