<script>
  import { Audio } from "../lib/audio";
  import { onDestroy, onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { isMidiLearnMode } from "../lib/stores";
  import { NOTES } from "../lib/data";

  // Constants
  const PRESETS = {
    "Classic Saw Lead": [
      { id: 0, value: 0.6 }, // Cutoff
      { id: 1, value: 0.3 }, // Res
      { id: 2, value: 0.01 }, // Attack
      { id: 3, value: 0.2 }, // Decay
      { id: 4, value: 0.7 }, // Sustain
      { id: 5, value: 0.3 }, // Release
      { id: 6, value: 0.8 }, // Vol
      { id: 7, value: 0.1 }  // Drive
    ],
    "Deep Moog Bass": [
      { id: 0, value: 0.25 },
      { id: 1, value: 0.6 },
      { id: 2, value: 0.01 },
      { id: 3, value: 0.3 },
      { id: 4, value: 0.5 },
      { id: 5, value: 0.1 },
      { id: 6, value: 0.9 },
      { id: 7, value: 0.4 }
    ],
    "Juno Cloud Pad": [
      { id: 0, value: 0.4 },
      { id: 1, value: 0.1 },
      { id: 2, value: 0.4 },
      { id: 3, value: 0.5 },
      { id: 4, value: 0.8 },
      { id: 5, value: 0.8 },
      { id: 6, value: 0.7 },
      { id: 7, value: 0.0 }
    ],
    "Acid Squelch": [
      { id: 0, value: 0.7 },
      { id: 1, value: 0.9 },
      { id: 2, value: 0.01 },
      { id: 3, value: 0.1 },
      { id: 4, value: 0.0 },
      { id: 5, value: 0.1 },
      { id: 6, value: 0.8 },
      { id: 7, value: 0.8 }
    ],
    "Stranger Arp": [
      { id: 0, value: 0.4 },
      { id: 1, value: 0.2 },
      { id: 2, value: 0.01 },
      { id: 3, value: 0.3 },
      { id: 4, value: 0.0 },
      { id: 5, value: 0.3 },
      { id: 6, value: 0.75 },
      { id: 7, value: 0.2 }
    ],
    "Fat 808 Bass": [
      { id: 0, value: 0.15 },
      { id: 1, value: 0.0 },
      { id: 2, value: 0.01 },
      { id: 3, value: 0.4 },
      { id: 4, value: 0.0 },
      { id: 5, value: 0.4 },
      { id: 6, value: 1.0 },
      { id: 7, value: 0.6 }
    ],
    "Soft Pluck": [
      { id: 0, value: 0.3 },
      { id: 1, value: 0.1 },
      { id: 2, value: 0.01 },
      { id: 3, value: 0.15 },
      { id: 4, value: 0.0 },
      { id: 5, value: 0.15 },
      { id: 6, value: 0.8 },
      { id: 7, value: 0.1 }
    ]
  };

  const PARAMS = [
    { id: 0, name: "Cutoff", min: 0, max: 1, step: 0.01, value: 0.5, unit: "Hz" },
    { id: 1, name: "Resonance", min: 0, max: 1, step: 0.01, value: 0.3, unit: "%" },
    { id: 2, name: "Attack", min: 0, max: 1, step: 0.01, value: 0.01, unit: "s" },
    { id: 3, name: "Decay", min: 0, max: 1, step: 0.01, value: 0.1, unit: "s" },
    { id: 4, name: "Sustain", min: 0, max: 1, step: 0.01, value: 0.7, unit: "%" },
    { id: 5, name: "Release", min: 0, max: 1, step: 0.01, value: 0.2, unit: "s" },
    { id: 6, name: "Volume", min: 0, max: 1, step: 0.01, value: 0.8, unit: "%" },
    { id: 7, name: "Drive", min: 0, max: 1, step: 0.01, value: 0.0, unit: "%" }
  ];

  const MODULE_STATE_KEY = "synth";
  const SAVED_PRESET_NAME = "Saved Preset";

  function createDefaultParams() {
    return PARAMS.map(param => ({ ...param }));
  }

  let activeParams = createDefaultParams();
  let heldNotes = new Set();
  let activePointers = new Map();
  let notePointers = new Map();
  let selectedPresetName = Object.keys(PRESETS)[0];
  let userPreset = null;
  let saveStatus = "";
  let isSavingPreset = false;
  let learnMode = false;

  $: presetNames = userPreset ? [...Object.keys(PRESETS), userPreset.name] : Object.keys(PRESETS);

  const unsubscribeLearnMode = isMidiLearnMode.subscribe(v => {
    const enteringLearnMode = v && !learnMode;
    learnMode = v;
    if (enteringLearnMode) stopAllNotes();
  });

  const BLACK_NOTES = new Set([1, 3, 6, 8, 10]);
  const KEYBOARD_NOTES = Array.from({ length: 24 }, (_, index) => {
    const midi = 60 + index;
    return {
      midi,
      name: `${NOTES[index % 12]}${4 + Math.floor(index / 12)}`,
      black: BLACK_NOTES.has(index % 12)
    };
  });
  const WHITE_KEYS = KEYBOARD_NOTES.filter(key => !key.black);
  const BLACK_KEYS = [
    { ...KEYBOARD_NOTES[1], afterWhite: 0 },
    { ...KEYBOARD_NOTES[3], afterWhite: 1 },
    { ...KEYBOARD_NOTES[6], afterWhite: 3 },
    { ...KEYBOARD_NOTES[8], afterWhite: 4 },
    { ...KEYBOARD_NOTES[10], afterWhite: 5 },
    { ...KEYBOARD_NOTES[13], afterWhite: 7 },
    { ...KEYBOARD_NOTES[15], afterWhite: 8 },
    { ...KEYBOARD_NOTES[18], afterWhite: 10 },
    { ...KEYBOARD_NOTES[20], afterWhite: 11 },
    { ...KEYBOARD_NOTES[22], afterWhite: 12 }
  ];

  onMount(() => {
    let mounted = true;
    let unlistenParamChange = () => {};
    let unlistenAllSoundsStopped = () => {};

    const releasePointerFromEvent = event => releasePointer(event.pointerId);
    const stopForPageExit = () => stopAllNotes();
    const stopWhenHidden = () => {
      if (document.hidden) stopAllNotes();
    };

    window.addEventListener("pointerup", releasePointerFromEvent, true);
    window.addEventListener("pointercancel", releasePointerFromEvent, true);
    window.addEventListener("blur", stopForPageExit);
    window.addEventListener("pagehide", stopForPageExit);
    document.addEventListener("visibilitychange", stopWhenHidden);

    listen("param-change", (event) => {
      const { id, value } = event.payload || {};
      if (Number.isInteger(Number(id))) {
        setLocalParam(Number(id), value);
      }
    })
      .then(unlisten => {
        if (mounted) unlistenParamChange = unlisten;
        else unlisten();
      })
      .catch(e => console.error("Failed to listen for synth parameter changes", e));

    listen("all-sounds-stopped", () => {
      stopAllNotes();
    })
      .then(unlisten => {
        if (mounted) unlistenAllSoundsStopped = unlisten;
        else unlisten();
      })
      .catch(e => console.error("Failed to listen for all sounds stopped", e));

    restorePersistedSynthState(() => mounted);

    return () => {
      mounted = false;
      unlistenParamChange();
      unlistenAllSoundsStopped();
      window.removeEventListener("pointerup", releasePointerFromEvent, true);
      window.removeEventListener("pointercancel", releasePointerFromEvent, true);
      window.removeEventListener("blur", stopForPageExit);
      window.removeEventListener("pagehide", stopForPageExit);
      document.removeEventListener("visibilitychange", stopWhenHidden);
      stopAllNotes();
    };
  });

  onDestroy(() => {
    unsubscribeLearnMode();
    stopAllNotes();
  });

  function normalizeParamValue(id, value) {
    const param = PARAMS.find(item => item.id === Number(id));
    const parsed = Number(value);
    const fallback = param?.value ?? 0;
    const min = param?.min ?? 0;
    const max = param?.max ?? 1;
    const next = Number.isFinite(parsed) ? parsed : fallback;
    return Math.max(min, Math.min(max, next));
  }

  function setLocalParam(id, value) {
    const numericId = Number(id);
    const nextValue = normalizeParamValue(numericId, value);
    activeParams = activeParams.map(param =>
      param.id === numericId ? { ...param, value: nextValue } : param
    );
    return nextValue;
  }

  function applyParamSnapshot(params, sendToAudio = true) {
    if (!Array.isArray(params)) return false;

    const valuesById = new Map();
    for (const item of params) {
      const id = Number(item?.id);
      if (!Number.isInteger(id) || !PARAMS.some(param => param.id === id)) continue;
      valuesById.set(id, item.value);
    }
    if (!valuesById.size) return false;

    activeParams = activeParams.map(param => {
      if (!valuesById.has(param.id)) return param;
      const value = normalizeParamValue(param.id, valuesById.get(param.id));
      if (sendToAudio) {
        Audio.setParam(param.id, value)
          .catch(e => console.error("Failed to apply synth parameter", e));
      }
      return { ...param, value };
    });

    return true;
  }

  function serializeParams() {
    return activeParams.map(param => ({
      id: param.id,
      value: normalizeParamValue(param.id, param.value)
    }));
  }

  function serializeSynthState() {
    return {
      schema_version: 1,
      selectedPresetName,
      savedPreset: userPreset,
      params: serializeParams()
    };
  }

  function normalizeSavedPreset(value) {
    if (!value || typeof value !== "object" || !Array.isArray(value.params)) return null;
    return {
      name: SAVED_PRESET_NAME,
      params: value.params,
      savedAt: value.savedAt || null
    };
  }

  function getPresetParams(name) {
    if (userPreset && name === userPreset.name) return userPreset.params;
    return PRESETS[name] || null;
  }

  function restoreSynthState(state) {
    if (!state || typeof state !== "object") return false;

    const savedPreset = normalizeSavedPreset(state.savedPreset);
    if (savedPreset) userPreset = savedPreset;

    const savedSelection = typeof state.selectedPresetName === "string" ? state.selectedPresetName : "";
    selectedPresetName = getPresetParams(savedSelection) ? savedSelection : (userPreset?.name || selectedPresetName);

    if (applyParamSnapshot(state.params)) return true;

    const preset = getPresetParams(selectedPresetName);
    return preset ? applyParamSnapshot(preset) : false;
  }

  async function restorePersistedSynthState(isCurrent) {
    try {
      const persisted = await Audio.loadPersistence();
      if (!isCurrent()) return;
      if (restoreSynthState(persisted?.module_state?.[MODULE_STATE_KEY])) return;
    } catch (e) {
      console.error("Failed to load Synth persistence", e);
    }

    if (isCurrent()) {
      applyParamSnapshot(getPresetParams(selectedPresetName));
    }
  }

  function handleParamChange(id, value) {
    const nextValue = setLocalParam(id, value);
    Audio.setParam(id, nextValue)
      .catch(e => console.error("Failed to update synth parameter", e));
  }

  function loadPreset(name) {
    const preset = getPresetParams(name);
    if (!preset) return;
    selectedPresetName = name;
    applyParamSnapshot(preset);
    saveStatus = name === userPreset?.name ? "Loaded saved preset" : "";
  }

  async function savePreset() {
    if (isSavingPreset) return;

    isSavingPreset = true;
    saveStatus = "Saving...";
    userPreset = {
      name: SAVED_PRESET_NAME,
      params: serializeParams(),
      savedAt: new Date().toISOString()
    };
    selectedPresetName = userPreset.name;

    try {
      await Audio.saveModuleState(MODULE_STATE_KEY, serializeSynthState());
      saveStatus = "Preset saved";
    } catch (e) {
      console.error("Failed to save Synth preset", e);
      saveStatus = "Save failed";
    } finally {
      isSavingPreset = false;
    }
  }

  function getRotation(val) {
    return -135 + (val * 270);
  }

  function startNote(midi) {
    if (learnMode || heldNotes.has(midi)) return;
    heldNotes = new Set([...heldNotes, midi]);
    Audio.noteOn(midi, 110).catch(e => {
      console.error("Failed to start synth note", e);
      clearNoteState(midi);
    });
  }

  function stopNote(midi) {
    if (!heldNotes.has(midi)) return;
    const nextHeldNotes = new Set(heldNotes);
    nextHeldNotes.delete(midi);
    heldNotes = nextHeldNotes;
    Audio.noteOff(midi).catch(e => console.error("Failed to stop synth note", e));
  }

  function stopAllNotes() {
    const notesToStop = new Set([...heldNotes, ...activePointers.values()]);
    activePointers.clear();
    notePointers.clear();
    heldNotes = new Set();

    for (const midi of notesToStop) {
      Audio.noteOff(midi).catch(() => {});
    }
  }

  function clearNoteState(midi) {
    for (const [pointerId, pointerMidi] of Array.from(activePointers.entries())) {
      if (pointerMidi === midi) activePointers.delete(pointerId);
    }
    notePointers.delete(midi);

    if (heldNotes.has(midi)) {
      const nextHeldNotes = new Set(heldNotes);
      nextHeldNotes.delete(midi);
      heldNotes = nextHeldNotes;
    }
  }

  function pressPointer(pointerId, midi) {
    if (learnMode || pointerId == null) return;
    if (activePointers.get(pointerId) === midi) return;
    if (activePointers.has(pointerId)) releasePointer(pointerId);

    activePointers.set(pointerId, midi);
    const pointerSet = notePointers.get(midi) || new Set();
    const shouldStartNote = pointerSet.size === 0;
    pointerSet.add(pointerId);
    notePointers.set(midi, pointerSet);

    if (shouldStartNote) startNote(midi);
  }

  function releasePointer(pointerId) {
    if (pointerId == null || !activePointers.has(pointerId)) return;

    const midi = activePointers.get(pointerId);
    activePointers.delete(pointerId);

    const pointerSet = notePointers.get(midi);
    if (!pointerSet) {
      stopNote(midi);
      return;
    }

    pointerSet.delete(pointerId);
    if (pointerSet.size === 0) {
      notePointers.delete(midi);
      stopNote(midi);
    }
  }

  function releaseNote(midi) {
    for (const [pointerId, pointerMidi] of Array.from(activePointers.entries())) {
      if (pointerMidi === midi) releasePointer(pointerId);
    }

    if (!notePointers.has(midi)) stopNote(midi);
  }

  function capturePointer(event) {
    try {
      event.currentTarget?.setPointerCapture?.(event.pointerId);
    } catch (_) {}
  }

  function releasePointerCapture(event) {
    try {
      if (event.currentTarget?.hasPointerCapture?.(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
    } catch (_) {}
  }

  function handleKeyPointerDown(event, midi) {
    if (event.pointerType === "mouse" && event.button !== 0) return;
    event.preventDefault();
    capturePointer(event);
    pressPointer(event.pointerId, midi);
  }

  function handleKeyPointerEnd(event) {
    event.preventDefault();
    releasePointer(event.pointerId);
    releasePointerCapture(event);
  }

  function keyboardPointerId(midi) {
    return `keyboard-${midi}`;
  }

  function isPlayableKeyboardEvent(event) {
    return event.key === " " || event.key === "Enter";
  }

  function handleKeyButtonKeyDown(event, midi) {
    if (!isPlayableKeyboardEvent(event)) return;
    event.preventDefault();
    if (event.repeat) return;
    pressPointer(keyboardPointerId(midi), midi);
  }

  function handleKeyButtonKeyUp(event, midi) {
    if (!isPlayableKeyboardEvent(event)) return;
    event.preventDefault();
    releasePointer(keyboardPointerId(midi));
  }
</script>

<div class="h-full flex flex-col gap-8">
  <div class="bg-stone-900 rounded-xl p-6 border border-stone-800 flex justify-between items-center">
    <div>
      <h2 class="text-xs font-bold text-stone-500 uppercase tracking-[0.2em] mb-1">Synthesizer</h2>
      <h1 class="text-2xl font-black text-white uppercase tracking-tight">Analog Engine v1</h1>
    </div>
    <div class="flex flex-wrap items-center justify-end gap-3">
      <select bind:value={selectedPresetName} on:change={(event) => loadPreset(event.currentTarget.value)} class="bg-stone-800 text-cyan-400 font-bold text-xs rounded-lg px-4 py-2 border border-stone-700 outline-none focus:border-cyan-500">
        {#each presetNames as name}
          <option value={name}>{name}</option>
        {/each}
      </select>
      <button
        type="button"
        on:click={savePreset}
        disabled={isSavingPreset}
        class="bg-stone-800 hover:bg-stone-700 text-stone-300 px-4 py-2 rounded-lg font-bold text-xs uppercase transition-all border border-stone-700 disabled:opacity-60 disabled:cursor-wait">
        {isSavingPreset ? "Saving" : "Save"}
      </button>
      {#if saveStatus}
        <span aria-live="polite" class="text-[10px] font-bold uppercase tracking-widest text-stone-500">{saveStatus}</span>
      {/if}
    </div>
  </div>

  <div class="flex-1 grid grid-cols-2 md:grid-cols-4 gap-6 content-center">
    {#each activeParams as param}
      <div class="flex flex-col items-center gap-3 group">
        <div
          class="relative w-24 h-24 rounded-full bg-stone-900 border-4 border-stone-800 shadow-2xl flex items-center justify-center group-hover:border-stone-700 transition-colors overflow-hidden">
          {#if learnMode}
            <button
                type="button"
                aria-label={`Map MIDI for ${param.name}`}
                on:click={() => Audio.learnMidi("param", param.id)}
                class="absolute inset-0 z-50 flex items-center justify-center bg-stone-900/80 cursor-crosshair border-2 border-orange-500 animate-pulse">
                <span class="text-xs font-black text-orange-500">MAP</span>
            </button>
          {/if}
          <div class="absolute inset-0 bg-cyan-500 opacity-5" style="transform: scale({0.5 + param.value * 0.5})"></div>
          <div class="absolute w-full h-full" style="transform: rotate({getRotation(param.value)}deg)">
            <div class="absolute top-2 left-1/2 -translate-x-1/2 w-1.5 h-4 bg-cyan-500 rounded-full shadow-[0_0_10px_rgba(34,211,238,0.8)]"></div>
          </div>
          <div class="z-10 text-[10px] font-mono font-bold text-stone-500 group-hover:text-cyan-400 transition-colors">
            {learnMode ? "MAP" : Math.round(param.value * 100) + "%"}
          </div>
          <input
            type="range"
            min={param.min}
            max={param.max}
            step={param.step}
            value={param.value}
            on:input={(event) => handleParamChange(param.id, event.currentTarget.value)}
            class="absolute inset-0 opacity-0 cursor-ns-resize"
            disabled={learnMode}
          />
        </div>
        <div class="text-center">
          <div class="text-[10px] font-black uppercase tracking-widest text-stone-500 group-hover:text-white transition-colors">{param.name}</div>
          <div class="text-[8px] font-mono text-stone-600 mt-0.5">CC {param.id + 1}</div>
        </div>
      </div>
    {/each}
  </div>

  <div class="relative h-32 bg-stone-900 rounded-xl border border-stone-800 px-3 pt-3 pb-3 overflow-hidden select-none">
    <div class="grid h-full gap-px" style="grid-template-columns: repeat({WHITE_KEYS.length}, minmax(0, 1fr));">
      {#each WHITE_KEYS as key}
        <button
          type="button"
          aria-label={`Play ${key.name}`}
          aria-pressed={heldNotes.has(key.midi)}
          on:pointerdown={(event) => handleKeyPointerDown(event, key.midi)}
          on:pointerup={handleKeyPointerEnd}
          on:pointercancel={handleKeyPointerEnd}
          on:lostpointercapture={(event) => releasePointer(event.pointerId)}
          on:blur={() => releaseNote(key.midi)}
          on:keydown={(event) => handleKeyButtonKeyDown(event, key.midi)}
          on:keyup={(event) => handleKeyButtonKeyUp(event, key.midi)}
          class="relative h-full rounded-b bg-stone-100 hover:bg-cyan-100 border border-stone-400 border-b-4 text-stone-900 transition-colors active:scale-[0.99] touch-none {heldNotes.has(key.midi) ? 'ring-2 ring-cyan-400 bg-cyan-300' : ''}">
          <span class="absolute bottom-2 left-1/2 -translate-x-1/2 text-[9px] font-black text-stone-700">{key.name}</span>
        </button>
      {/each}
    </div>

    {#each BLACK_KEYS as key}
      <button
        type="button"
        aria-label={`Play ${key.name}`}
        aria-pressed={heldNotes.has(key.midi)}
        on:pointerdown={(event) => handleKeyPointerDown(event, key.midi)}
        on:pointerup={handleKeyPointerEnd}
        on:pointercancel={handleKeyPointerEnd}
        on:lostpointercapture={(event) => releasePointer(event.pointerId)}
        on:blur={() => releaseNote(key.midi)}
        on:keydown={(event) => handleKeyButtonKeyDown(event, key.midi)}
        on:keyup={(event) => handleKeyButtonKeyUp(event, key.midi)}
        class="absolute top-3 z-10 h-20 w-8 rounded-b bg-stone-950 hover:bg-cyan-950 border border-stone-800 border-b-4 transition-colors active:scale-[0.98] touch-none {heldNotes.has(key.midi) ? 'ring-2 ring-cyan-400 bg-cyan-900' : ''}"
        style="left: calc({((key.afterWhite + 1) / WHITE_KEYS.length) * 100}% - 1rem);">
        <span class="absolute bottom-2 left-1/2 -translate-x-1/2 text-[9px] font-black text-stone-400">{key.name}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  input[type=range] {
    writing-mode: bt-lr;
  }
</style>
