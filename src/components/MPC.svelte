<script>
  import { Audio } from "../lib/audio";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { isMidiLearnMode } from "../lib/stores";

  let currentStep = 0;
  let isPlaying = false;
  let activePad = 0;
  let learnMode = false;
  let swing = 0.0;
  let bpm = 120;
  let restoringState = true;

  isMidiLearnMode.subscribe(v => learnMode = v);

  // Kits
  const KITS = [
    { id: 0, name: "TR-808", padNames: ["Kick 1", "Kick 2", "Sub", "Tom", "Snare 1", "Snare 2", "Clap", "Rim", "Hat Cl", "Hat Op", "Cymbal", "Shaker", "Cowbell", "Clave", "Zap", "Laser"] },
    { id: 1, name: "TR-909", padNames: ["Kick Main", "Kick Hard", "Kick Low", "Tom", "Snare Main", "Snare Br", "Clap", "Rim", "CH", "OH", "Ride", "Crash", "Tamb", "Agogo", "FX 1", "FX 2"] },
    { id: 2, name: "Acoustic", padNames: ["Kick", "Kick Damp", "Tom Low", "Tom Mid", "Snare", "Side Stick", "Clap", "Rim", "Hat Cl", "Hat Pedal", "Hat Op", "Ride", "Crash", "Shaker", "Perc 1", "Perc 2"] },
    { id: 3, name: "Lo-Fi", padNames: ["Dust Kick", "Tape Kick", "Sub Hit", "Box Tom", "Dust Snare", "Snap", "Clap", "Rim", "Hat Tight", "Hat Open", "Noise Hat", "Vinyl", "Crate Hit", "Knock", "Zap", "Texture"] }
  ];
  let activeKit = KITS[0];
  let hoveredPad = null;

  onMount(async () => {
    const persisted = await Audio.loadPersistence().catch(e => {
        console.error("Failed to load MPC persistence:", e);
        return null;
    });
    const savedMpc = persisted?.module_state?.mpc;
    if (savedMpc) {
        restoreMpcState(savedMpc);
    } else {
        loadKit(activeKit.id, false);
    }
    restoringState = false;

    const unlisten = await listen("mpc-param-change", (event) => {
        const { id, value } = event.payload;
        if (id === 0) {
            swing = value;
            persistMpcState();
        } else if (id === 1) {
            const idx = normalizedKitIndex(value);
            loadKit(KITS[idx].id, false);
            persistMpcState();
        }
    });

    const unlistenDrum = await listen("drum-trigger", (event) => {
        const note = event.payload;
        const padId = note - 36;
        if (padId >= 0 && padId < 16) {
            triggerVisual(padId);
        }
    });

    const unlistenStep = await listen("mpc-step", (event) => {
        currentStep = event.payload;
    });

    const unlistenTransport = await listen("mpc-transport", (event) => {
        isPlaying = event.payload;
        if (!isPlaying) {
            currentStep = 0;
        }
    });

    const unlistenSampleLoaded = await listen("mpc-sample-loaded", (event) => {
        const { pad_id, sample_rate, samples, waveform } = event.payload;
        if (pad_id >= 0 && pad_id < pads.length) {
            const previous = pads[pad_id];
            pads[pad_id].waveform = waveform || [];
            pads[pad_id].sampleRate = sample_rate || 0;
            pads[pad_id].sampleFrames = samples || 0;
            pads[pad_id].trimStart = previous.trimStart ?? 0;
            pads[pad_id].trimEnd = previous.trimEnd ?? 1;
            pads[pad_id].volume = previous.volume ?? 1;
            pads[pad_id].pitch = previous.pitch ?? 0;
            pads = [...pads];
            Audio.setMpcSampleTrim(pad_id, pads[pad_id].trimStart, pads[pad_id].trimEnd, pads[pad_id].volume, pads[pad_id].pitch);
            persistMpcState();
        }
    });

    const unlistenDrop = await listen("tauri://file-drop", (event) => {
        if (hoveredPad !== null && event.payload && event.payload.length > 0) {
            const path = event.payload[0];
            Audio.loadSample(hoveredPad, path);
            
            // Update Name
            const filename = path.split(/[\\/]/).pop();
            pads[hoveredPad].name = filename.length > 10 ? filename.substring(0, 10) + "..." : filename;
            pads[hoveredPad].sample = path;
            pads = [...pads];
            persistMpcState();
        }
    });

    return () => {
        unlisten();
        unlistenDrum();
        unlistenStep();
        unlistenTransport();
        unlistenSampleLoaded();
        unlistenDrop();
        Audio.stopMpcSequencer();
    };
  });

  // 16 Pads Data
  let pads = Array(16).fill(0).map((_, i) => ({
    id: i,
    name: `PAD ${i + 1}`,
    sample: null,
    waveform: [],
    sampleRate: 0,
    sampleFrames: 0,
    trimStart: 0,
    trimEnd: 1,
    volume: 1,
    pitch: 0,
    steps: Array(16).fill(false),
    triggered: false
  }));

  function triggerVisual(id) {
    pads[id].triggered = true;
    setTimeout(() => {
        pads[id].triggered = false;
        pads = [...pads];
    }, 100);
    pads = [...pads];
  }

  function normalizedKitIndex(value) {
    return Math.min(KITS.length - 1, Math.floor(Number(value || 0) * KITS.length));
  }

  function loadKit(kitId, notifyBackend = true) {
    const kit = KITS.find(k => k.id === kitId) || KITS[0];
    activeKit = kit;
    pads = pads.map((pad, i) => ({
        ...pad,
        name: kit.padNames[i] || `PAD ${i + 1}`,
        sample: null,
        waveform: [],
        sampleRate: 0,
        sampleFrames: 0,
        trimStart: 0,
        trimEnd: 1,
        volume: 1,
        pitch: 0,
        triggered: false
    }));

    if (notifyBackend) {
        Audio.setMpcKit(kit.id);
    }
    persistMpcState();
  }

  function serializeMpcState() {
    return {
        schema_version: 1,
        activeKitId: activeKit.id,
        activePad,
        swing: Number(swing),
        bpm: Number(bpm),
        pads: pads.map(pad => ({
            id: pad.id,
            name: pad.name,
            sample: pad.sample,
            trimStart: pad.trimStart,
            trimEnd: pad.trimEnd,
            volume: pad.volume,
            pitch: pad.pitch,
            steps: pad.steps
        }))
    };
  }

  function persistMpcState() {
    if (restoringState) return;
    Audio.saveModuleState("mpc", serializeMpcState())
        .catch(e => console.error("Failed to persist MPC state:", e));
  }

  function restoreMpcState(state) {
    const kit = KITS.find(k => k.id === state.activeKitId) || KITS[0];
    activeKit = kit;
    activePad = Number.isInteger(state.activePad) ? Math.min(Math.max(state.activePad, 0), 15) : 0;
    swing = Number.isFinite(Number(state.swing)) ? Number(state.swing) : 0;
    bpm = Number.isFinite(Number(state.bpm)) ? Number(state.bpm) : 120;

    const savedPads = Array.isArray(state.pads) ? state.pads : [];
    pads = pads.map((pad, i) => {
        const saved = savedPads.find(item => item?.id === i) || {};
        return {
            ...pad,
            name: saved.name || kit.padNames[i] || `PAD ${i + 1}`,
            sample: saved.sample || null,
            waveform: [],
            sampleRate: 0,
            sampleFrames: 0,
            trimStart: Number.isFinite(Number(saved.trimStart)) ? Number(saved.trimStart) : 0,
            trimEnd: Number.isFinite(Number(saved.trimEnd)) ? Number(saved.trimEnd) : 1,
            volume: Number.isFinite(Number(saved.volume)) ? Number(saved.volume) : 1,
            pitch: Number.isFinite(Number(saved.pitch)) ? Number(saved.pitch) : 0,
            steps: Array.isArray(saved.steps) && saved.steps.length === 16 ? saved.steps.map(Boolean) : Array(16).fill(false),
            triggered: false
        };
    });

    Audio.setMpcKit(kit.id);
    Audio.setMpcParam(0, Number(swing));
    for (const pad of pads) {
        pad.steps.forEach((active, stepIdx) => {
            if (active) Audio.setMpcStep(pad.id, stepIdx, true);
        });
        if (pad.sample) {
            Audio.loadSample(pad.id, pad.sample);
            Audio.setMpcSampleTrim(pad.id, pad.trimStart, pad.trimEnd, pad.volume, pad.pitch);
        }
    }
  }

  // Sequencer transport is owned by the Rust audio engine.
  function toggleSequencer() {
    if (learnMode) {
        Audio.learnMidi("transport", 0);
        return;
    }

    if (isPlaying) {
      Audio.stopMpcSequencer();
    } else {
      Audio.startMpcSequencer(Number(bpm), Number(swing));
    }
  }

  function triggerPad(id) {
    if (learnMode) {
        Audio.learnMidi("note", 36 + id);
        return;
    }
    activePad = id;
    Audio.noteOn(36 + id, 100);
    setTimeout(() => Audio.noteOff(36 + id), 50);
  }

  function toggleStep(padId, stepIdx) {
    const active = !pads[padId].steps[stepIdx];
    pads[padId].steps[stepIdx] = active;
    pads = [...pads];
    Audio.setMpcStep(padId, stepIdx, active);
    persistMpcState();
  }

  function setTrim(padId, field, value) {
    const next = Number(value);
    if (!Number.isFinite(next)) return;

    const pad = pads[padId];
    if (!pad) return;

    if (field === "start") {
        pad.trimStart = Math.min(Math.max(next, 0), Math.max(0, pad.trimEnd - 0.01));
    } else {
        pad.trimEnd = Math.max(Math.min(next, 1), Math.min(1, pad.trimStart + 0.01));
    }

    pads = [...pads];
    Audio.setMpcSampleTrim(padId, pad.trimStart, pad.trimEnd, pad.volume, pad.pitch);
    persistMpcState();
  }

  function setSampleParam(padId, field, value) {
    const next = Number(value);
    if (!Number.isFinite(next)) return;

    const pad = pads[padId];
    if (!pad) return;

    if (field === "volume") {
        pad.volume = Math.min(Math.max(next, 0), 2);
    } else if (field === "pitch") {
        pad.pitch = Math.min(Math.max(next, -24), 24);
    }

    pads = [...pads];
    Audio.setMpcSampleTrim(padId, pad.trimStart, pad.trimEnd, pad.volume, pad.pitch);
    persistMpcState();
  }

  function sampleDuration(pad) {
    if (!pad.sampleFrames || !pad.sampleRate) return "0.00s";
    return `${(pad.sampleFrames / pad.sampleRate).toFixed(2)}s`;
  }

  function trimTime(pad, value) {
    if (!pad.sampleFrames || !pad.sampleRate) return "0.00s";
    return `${((pad.sampleFrames / pad.sampleRate) * value).toFixed(2)}s`;
  }
</script>

<div class="h-full flex flex-col gap-6">
  <div class="bg-stone-900 rounded-xl p-6 border border-stone-800 flex justify-between items-center">
    <div class="flex items-center gap-6">
      <button on:click={toggleSequencer} aria-label={isPlaying ? "Stop sequencer" : "Start sequencer"}
        class="w-12 h-12 rounded-full flex items-center justify-center transition-all {isPlaying ? "bg-red-600 shadow-[0_0_15px_rgba(220,38,38,0.5)]" : "bg-stone-800 text-stone-400 hover:text-white"} {learnMode ? "ring-2 ring-orange-500" : ""}">
        <div class={isPlaying ? "w-4 h-4 bg-white rounded-sm" : "w-0 h-0 border-t-[8px] border-t-transparent border-l-[14px] border-l-current border-b-[8px] border-b-transparent ml-1"}></div>
      </button>
      <div>
        <h2 class="text-xs font-bold text-stone-500 uppercase tracking-widest mb-1">Kit Selection</h2>
        <div class="flex gap-2">
          {#each KITS as kit}
            <button
              on:click={() => { if(learnMode) { Audio.learnMidi("mpc_param", 1); } else { loadKit(kit.id); } }}
              class="px-3 py-1 rounded text-[10px] font-bold uppercase transition-all {activeKit.id === kit.id ? "bg-cyan-600 text-white shadow-lg" : "bg-stone-800 text-stone-500 hover:bg-stone-700"} {learnMode ? "border border-orange-500" : ""}">
              {kit.name}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <div class="text-right">
      <div class="flex items-end gap-4">
        <label class="block">
          <span class="block text-stone-500 text-[10px] font-bold uppercase tracking-widest mb-1">BPM</span>
          <input
            type="number" min="40" max="240" bind:value={bpm}
            on:change={persistMpcState}
            class="w-20 bg-stone-800 border border-stone-700 rounded px-2 py-1 text-right text-xs font-mono text-stone-200 outline-none focus:border-cyan-500">
        </label>
        <label class="block">
          <span class="block text-stone-500 text-[10px] font-bold uppercase tracking-widest mb-1">Swing</span>
          <input
            type="range" min="0" max="0.5" step="0.05" bind:value={swing}
            on:mousedown={() => { if(learnMode) Audio.learnMidi("mpc_param", 0); }}
            on:change={() => { if(!learnMode) { Audio.setMpcParam(0, Number(swing)); persistMpcState(); } }}
            class="w-32 h-1 bg-stone-800 appearance-none rounded-full accent-cyan-500 {learnMode ? "ring-1 ring-orange-500" : ""}">
        </label>
      </div>
    </div>
  </div>

  <div class="flex-1 grid grid-cols-1 lg:grid-cols-2 gap-6">
    <div class="grid grid-cols-4 gap-3 p-4 bg-stone-900 rounded-2xl border border-stone-800 shadow-2xl">
      {#each pads as pad}
        <div class="relative aspect-square">
            {#if learnMode}
                <button
                    type="button"
                    aria-label={`Map MIDI note for pad ${pad.id + 1}`}
                    on:click={() => Audio.learnMidi("note", 36 + pad.id)}
                    class="absolute inset-0 z-50 bg-stone-900/80 flex items-center justify-center border-2 border-orange-500 rounded-lg cursor-crosshair animate-pulse">
                    <span class="text-[10px] font-black text-orange-500">MAP NOTE</span>
                </button>
            {/if}
            <button
            on:mousedown={() => triggerPad(pad.id)}
            on:mouseenter={() => hoveredPad = pad.id}
            on:mouseleave={() => hoveredPad = null}
            class="w-full h-full rounded-lg bg-stone-800 border-b-4 border-stone-950 flex flex-col items-center justify-center transition-all active:scale-95 active:border-b-0 active:translate-y-1 {activePad === pad.id || pad.triggered ? "bg-cyan-600 border-cyan-800 text-white shadow-[0_0_15px_rgba(34,211,238,0.6)] scale-95 border-b-0 translate-y-1" : "text-stone-500 hover:bg-stone-700 hover:text-stone-300"}">
            <span class="text-xl font-black">{pad.id + 1}</span>
            <span class="text-[8px] font-bold uppercase tracking-tighter opacity-50">{pad.name}</span>
            </button>
        </div>
      {/each}
    </div>

    <div class="flex flex-col gap-6">
      <div class="flex-1 bg-stone-950 rounded-xl border border-stone-800 p-4 relative overflow-hidden flex flex-col justify-center items-center">
        <div class="absolute top-4 left-4 right-4 flex justify-between items-start z-10">
          <div>
            <div class="text-[9px] font-bold text-stone-600 uppercase tracking-widest">Selected Pad</div>
            <div class="text-lg font-black text-stone-200 truncate max-w-64">{pads[activePad].name}</div>
          </div>
          <div class="text-right text-[10px] font-mono text-stone-500">
            {#if pads[activePad].sample}
              <div class="text-cyan-500 font-bold">Sample</div>
              <div>{sampleDuration(pads[activePad])}</div>
            {:else}
              <div>Synth Kit</div>
            {/if}
          </div>
        </div>

        <div class="w-full h-32 border-y border-stone-900 flex items-center justify-center px-2">
          {#if pads[activePad].waveform.length > 0}
            <div class="relative w-full h-24 flex items-center gap-px">
              <div
                class="absolute top-0 bottom-0 bg-cyan-500/15 border-x border-cyan-400 pointer-events-none"
                style={`left: ${pads[activePad].trimStart * 100}%; width: ${(pads[activePad].trimEnd - pads[activePad].trimStart) * 100}%;`}>
              </div>
              {#each pads[activePad].waveform as peak}
                <div class="flex-1 bg-cyan-500/70 min-w-px" style={`height: ${Math.max(4, peak * 100)}%;`}></div>
              {/each}
            </div>
          {:else}
            <div class="w-full h-px bg-cyan-500 opacity-20"></div>
          {/if}
        </div>

        <div class="absolute bottom-4 left-4 right-4">
          <div class="flex justify-between text-[10px] font-mono text-stone-600 uppercase mb-3">
            <span>Start: {trimTime(pads[activePad], pads[activePad].trimStart)}</span>
            <span class="text-cyan-500 font-bold">{pads[activePad].sample ? "Trim Active" : "Drop WAV on a pad"}</span>
            <span>End: {trimTime(pads[activePad], pads[activePad].trimEnd)}</span>
          </div>
          <div class="grid grid-cols-4 gap-4">
            <label class="block">
              <span class="block text-[9px] font-bold uppercase tracking-widest text-stone-600 mb-1">Start Trim</span>
              <input
                type="range" min="0" max="0.99" step="0.001"
                value={pads[activePad].trimStart}
                disabled={!pads[activePad].sample}
                on:input={(e) => setTrim(activePad, "start", e.currentTarget.value)}
                class="w-full h-1 bg-stone-800 appearance-none rounded-full accent-orange-500 disabled:opacity-30">
            </label>
            <label class="block">
              <span class="block text-[9px] font-bold uppercase tracking-widest text-stone-600 mb-1">End Trim</span>
              <input
                type="range" min="0.01" max="1" step="0.001"
                value={pads[activePad].trimEnd}
                disabled={!pads[activePad].sample}
                on:input={(e) => setTrim(activePad, "end", e.currentTarget.value)}
                class="w-full h-1 bg-stone-800 appearance-none rounded-full accent-orange-500 disabled:opacity-30">
            </label>
            <label class="block">
              <span class="block text-[9px] font-bold uppercase tracking-widest text-stone-600 mb-1">Vol {pads[activePad].volume.toFixed(2)}</span>
              <input
                type="range" min="0" max="2" step="0.01"
                value={pads[activePad].volume}
                disabled={!pads[activePad].sample}
                on:input={(e) => setSampleParam(activePad, "volume", e.currentTarget.value)}
                class="w-full h-1 bg-stone-800 appearance-none rounded-full accent-cyan-500 disabled:opacity-30">
            </label>
            <label class="block">
              <span class="block text-[9px] font-bold uppercase tracking-widest text-stone-600 mb-1">Pitch {pads[activePad].pitch}</span>
              <input
                type="range" min="-24" max="24" step="1"
                value={pads[activePad].pitch}
                disabled={!pads[activePad].sample}
                on:input={(e) => setSampleParam(activePad, "pitch", e.currentTarget.value)}
                class="w-full h-1 bg-stone-800 appearance-none rounded-full accent-cyan-500 disabled:opacity-30">
            </label>
          </div>
        </div>
      </div>

      <div class="bg-stone-900 rounded-xl p-6 border border-stone-800">
        <h3 class="text-[10px] font-black text-stone-500 uppercase tracking-[0.2em] mb-4">Sequencer: {pads[activePad].name}</h3>
        <div class="grid grid-cols-8 gap-2">
          {#each pads[activePad].steps as step, i}
            <button
              on:click={() => toggleStep(activePad, i)}
              class="h-10 rounded border transition-all relative {step ? "bg-cyan-500 border-cyan-400 shadow-[0_0_10px_rgba(34,211,238,0.4)]" : "bg-stone-800 border-stone-700 hover:bg-stone-700"} {currentStep === i ? "ring-2 ring-white z-10" : ""}">
              {#if i % 4 === 0}
                <div class="absolute top-0.5 left-1 text-[8px] font-black opacity-30">{Math.floor(i/4) + 1}</div>
              {/if}
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>
