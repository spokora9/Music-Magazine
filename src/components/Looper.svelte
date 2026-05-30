<script>
  import { Audio } from "../lib/audio";
  import { onDestroy, onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { isMidiLearnMode, isMetronomeEnabled, metronomeBpm } from "../lib/stores";
  import { NOTES } from "../lib/data";

  const PART_DEFS = [
    { id: 0, key: "A", label: "PART A", color: "text-red-500", defaultSource: "input:1" },
    { id: 1, key: "B", label: "PART B", color: "text-green-500", defaultSource: "input:2" },
    { id: 2, key: "C", label: "PART C", color: "text-blue-500", defaultSource: "mix" },
    { id: 3, key: "D", label: "PART D", color: "text-yellow-400", defaultSource: "silent" },
    { id: 4, key: "E", label: "PART E", color: "text-cyan-400", defaultSource: "silent" }
  ];

  const SOURCE_OPTIONS = [
    { value: "input:1", label: "Input 1" },
    { value: "input:2", label: "Input 2" },
    { value: "input:3", label: "Input 3" },
    { value: "input:4", label: "Input 4" },
    { value: "input:5", label: "Input 5" },
    { value: "input:6", label: "Input 6" },
    { value: "input:7", label: "Input 7" },
    { value: "input:8", label: "Input 8" },
    { value: "mix", label: "Input Mix" },
    { value: "synth", label: "Synth" },
    { value: "mpc", label: "MPC" },
    { value: "jam", label: "Jam" },
    { value: "instrument", label: "Instrument Mix" },
    { value: "silent", label: "Silent" }
  ];

  const looperFxControls = [
    { id: 0, label: "FX Level", accent: "accent-stone-300", valueText: value => `${Math.round(value * 200)}%` },
    { id: 1, label: "Drive", accent: "accent-red-500", valueText: value => `${Math.round(value * 100)}%` },
    { id: 2, label: "Lowpass", accent: "accent-emerald-400", valueText: value => value > 0.98 ? "Open" : `${Math.round(value * 100)}%` },
    { id: 3, label: "Highpass", accent: "accent-lime-400", valueText: value => `${Math.round(value * 100)}%` },
    { id: 4, label: "Delay Time", accent: "accent-cyan-500", valueText: value => `${Math.round(10 + value * 1200)}ms` },
    { id: 5, label: "Feedback", accent: "accent-cyan-500", valueText: value => `${Math.round(value * 92)}%` },
    { id: 6, label: "Delay Mix", accent: "accent-cyan-400", valueText: value => `${Math.round(value * 100)}%` },
    { id: 7, label: "Reverb", accent: "accent-fuchsia-500", valueText: value => `${Math.round(value * 100)}%` },
    { id: 8, label: "Room Size", accent: "accent-fuchsia-400", valueText: value => `${Math.round(value * 100)}%` },
    { id: 9, label: "Chorus", accent: "accent-sky-400", valueText: value => `${Math.round(value * 100)}%` },
    { id: 10, label: "Tremolo", accent: "accent-yellow-400", valueText: value => `${Math.round(value * 100)}%` },
    { id: 11, label: "Slicer", accent: "accent-orange-500", valueText: value => `${Math.round(value * 100)}%` },
    { id: 12, label: "Crush", accent: "accent-pink-500", valueText: value => `${Math.round(value * 100)}%` },
    { id: 13, label: "Punch", accent: "accent-green-500", valueText: value => `${Math.round(value * 100)}%` }
  ];

  const defaultLooperFxValues = [0.5, 0, 1, 0, 0.24, 0.38, 0, 0, 0.55, 0, 0, 0, 0, 0];
  const looperFxPresets = [
    { name: "Clean", values: [0.5, 0, 1, 0, 0.24, 0.35, 0, 0, 0.55, 0, 0, 0, 0, 0] },
    { name: "Vocal Space", values: [0.56, 0.08, 0.9, 0.08, 0.32, 0.34, 0.16, 0.38, 0.74, 0.08, 0, 0, 0, 0.2] },
    { name: "Guitar Wide", values: [0.58, 0.14, 0.82, 0.04, 0.26, 0.28, 0.13, 0.22, 0.6, 0.34, 0, 0, 0, 0.16] },
    { name: "Ambient", values: [0.52, 0.02, 0.75, 0.02, 0.56, 0.58, 0.34, 0.66, 0.92, 0.18, 0, 0, 0, 0.08] },
    { name: "Beat Repeat", values: [0.5, 0.12, 0.88, 0.02, 0.12, 0.72, 0.48, 0.12, 0.45, 0, 0, 0.72, 0.1, 0.25] },
    { name: "Lo-Fi", values: [0.52, 0.24, 0.44, 0.12, 0.18, 0.28, 0.1, 0.2, 0.52, 0.16, 0, 0.12, 0.48, 0.14] }
  ];

  const SYNTH_PRESETS = {
    "Classic Saw": [
      { id: 0, value: 0.6 },
      { id: 1, value: 0.3 },
      { id: 2, value: 0.01 },
      { id: 3, value: 0.2 },
      { id: 4, value: 0.7 },
      { id: 5, value: 0.3 },
      { id: 6, value: 0.8 },
      { id: 7, value: 0.1 }
    ],
    "Moog Bass": [
      { id: 0, value: 0.25 },
      { id: 1, value: 0.6 },
      { id: 2, value: 0.01 },
      { id: 3, value: 0.3 },
      { id: 4, value: 0.5 },
      { id: 5, value: 0.1 },
      { id: 6, value: 0.9 },
      { id: 7, value: 0.4 }
    ],
    "Cloud Pad": [
      { id: 0, value: 0.4 },
      { id: 1, value: 0.1 },
      { id: 2, value: 0.4 },
      { id: 3, value: 0.5 },
      { id: 4, value: 0.8 },
      { id: 5, value: 0.8 },
      { id: 6, value: 0.7 },
      { id: 7, value: 0.0 }
    ],
    "Acid": [
      { id: 0, value: 0.7 },
      { id: 1, value: 0.9 },
      { id: 2, value: 0.01 },
      { id: 3, value: 0.1 },
      { id: 4, value: 0.0 },
      { id: 5, value: 0.1 },
      { id: 6, value: 0.8 },
      { id: 7, value: 0.8 }
    ]
  };

  const SYNTH_PARAMS = [
    { id: 0, name: "Cutoff", value: 0.5 },
    { id: 1, name: "Res", value: 0.3 },
    { id: 2, name: "Attack", value: 0.01 },
    { id: 3, name: "Decay", value: 0.1 },
    { id: 4, name: "Sustain", value: 0.7 },
    { id: 5, name: "Release", value: 0.2 },
    { id: 6, name: "Volume", value: 0.8 },
    { id: 7, name: "Drive", value: 0.0 }
  ];

  const BLACK_NOTES = new Set([1, 3, 6, 8, 10]);
  const COMPACT_KEYS = Array.from({ length: 17 }, (_, index) => {
    const midi = 48 + index;
    return {
      midi,
      name: `${NOTES[midi % 12]}${Math.floor(midi / 12) - 1}`,
      black: BLACK_NOTES.has(midi % 12)
    };
  });

  const MPC_KITS = [
    { id: 0, name: "TR-808", padNames: ["Kick 1", "Kick 2", "Sub", "Tom", "Snare 1", "Snare 2", "Clap", "Rim", "Hat Cl", "Hat Op", "Cymbal", "Shaker", "Cowbell", "Clave", "Zap", "Laser"] },
    { id: 1, name: "TR-909", padNames: ["Kick Main", "Kick Hard", "Kick Low", "Tom", "Snare Main", "Snare Br", "Clap", "Rim", "CH", "OH", "Ride", "Crash", "Tamb", "Agogo", "FX 1", "FX 2"] },
    { id: 2, name: "Acoustic", padNames: ["Kick", "Kick Damp", "Tom Low", "Tom Mid", "Snare", "Side Stick", "Clap", "Rim", "Hat Cl", "Hat Pedal", "Hat Op", "Ride", "Crash", "Shaker", "Perc 1", "Perc 2"] },
    { id: 3, name: "Lo-Fi", padNames: ["Dust Kick", "Tape Kick", "Sub Hit", "Box Tom", "Dust Snare", "Snap", "Clap", "Rim", "Hat Tight", "Hat Open", "Noise Hat", "Vinyl", "Crate Hit", "Knock", "Zap", "Texture"] }
  ];

  const JAM_CHORDS = [
    { label: "C", notes: [60, 64, 67] },
    { label: "F", notes: [65, 69, 72] },
    { label: "G", notes: [67, 71, 74] },
    { label: "Am", notes: [69, 72, 76] }
  ];

  function createPart(def) {
    return {
      ...def,
      state: "empty",
      source: def.defaultSource,
      layers: 0,
      waveform: [],
      notice: "",
      fxValues: [...defaultLooperFxValues],
      volume: 1
    };
  }

  let parts = PART_DEFS.map(createPart);
  let activePart = 0;
  let transportState = "stopped";
  let micGain = 1.5;
  let learnMode = false;
  let metroEnabled = false;
  let metroBpm = 120;
  let inputMonitoring = false;
  let activeSequenceStep = -1;
  let loopDurationSamples = 0;
  let sampleRate = 44100;
  let playbackStartTime = 0;
  let animationFrame;
  let songSequence = [];
  let isPlayingSequence = false;
  let canvas;

  let activeInstrumentTab = "synth";
  let synthParams = SYNTH_PARAMS.map(param => ({ ...param }));
  let heldNotes = new Set();
  let activeMpcKit = MPC_KITS[0];
  let activeMpcPad = 0;
  let mpcBpm = 120;
  let mpcSwing = 0;
  let mpcPlaying = false;
  let mpcCurrentStep = 0;
  let compactMpcSteps = Array.from({ length: 16 }, () => Array(16).fill(false));

  $: activePartData = parts[activePart] || parts[0];
  $: activePartHasUndoableMaterial = activePartData?.layers > 0 || activePartData?.state === "recording" || activePartData?.state === "overdubbing";
  $: hasAnyRecordedPart = parts.some(part => part.layers > 0);

  isMidiLearnMode.subscribe(v => learnMode = v);
  let firstMetroInit = true;
  isMetronomeEnabled.subscribe(v => {
    metroEnabled = v;
    if (!firstMetroInit) {
      Audio.setMetronome(v, metroBpm);
    }
    firstMetroInit = false;
  });
  metronomeBpm.subscribe(v => metroBpm = v);

  function normalizedMetroBpm(value = metroBpm) {
    const bpm = Number(value);
    if (!Number.isFinite(bpm)) return 120;
    return Math.max(40, Math.min(240, bpm));
  }

  function persistMetronomeState(enabled = metroEnabled, bpm = metroBpm) {
    const nextBpm = normalizedMetroBpm(bpm);
    Audio.saveAppSettings({
      metronome_enabled: enabled,
      metronome_bpm: nextBpm
    }).catch(e => console.error("Failed to persist metronome settings", e));
  }

  function updatePart(partId, updater) {
    parts = parts.map(part => part.id === partId ? { ...part, ...updater(part) } : part);
  }

  function getSourceLabel(value) {
    return SOURCE_OPTIONS.find(option => option.value === value)?.label || "Silent";
  }

  function sourceMetadataToValue(source) {
    if (!source) return null;
    if (typeof source === "string") {
      if (source === "InputMix") return "mix";
      if (source === "Synth") return "synth";
      if (source === "Mpc" || source === "MPC") return "mpc";
      if (source === "Jam") return "jam";
      if (source === "InstrumentMix") return "instrument";
      if (source === "Silent") return "silent";
      return source.toLowerCase();
    }
    if (typeof source === "object" && "InputChannel" in source) {
      const channel = Number(source.InputChannel);
      return channel === 0 ? "mix" : `input:${channel}`;
    }
    return null;
  }

  async function restorePersistedSources() {
    const persisted = await Audio.loadPersistence().catch(e => {
      console.error("Failed to load looper source persistence:", e);
      return null;
    });
    const savedSources = persisted?.settings?.looper_sources;
    if (!Array.isArray(savedSources)) return;

    parts = parts.map(part => ({
      ...part,
      source: sourceMetadataToValue(savedSources[part.id]) || part.source
    }));
  }

  function startAnimation() {
    if (animationFrame) cancelAnimationFrame(animationFrame);
    if (playbackStartTime === 0) playbackStartTime = performance.now();

    function loop() {
      if (transportState !== "playing" && transportState !== "recording" && transportState !== "overdubbing") {
        drawCurrentState();
        return;
      }

      drawCurrentState();
      animationFrame = requestAnimationFrame(loop);
    }
    animationFrame = requestAnimationFrame(loop);
  }

  function drawCurrentState() {
    if (!canvas || !activePartData) return;
    drawWaveformOnly(activePartData.waveform, activePartData.color);

    if (loopDurationSamples > 0 && (transportState === "playing" || transportState === "overdubbing")) {
      const now = performance.now();
      const elapsedSec = (now - playbackStartTime) / 1000;
      const elapsedSamples = elapsedSec * sampleRate;
      const progress = (elapsedSamples % loopDurationSamples) / loopDurationSamples;
      const ctx = canvas.getContext("2d");
      const x = progress * canvas.width;

      ctx.beginPath();
      ctx.strokeStyle = "white";
      ctx.lineWidth = 2;
      ctx.moveTo(x, 0);
      ctx.lineTo(x, canvas.height);
      ctx.stroke();
    }
  }

  function toggleMonitoring() {
    if (learnMode) {
      Audio.learnMidi("mic_gain", 0);
      return;
    }
    inputMonitoring = !inputMonitoring;
    Audio.setInputMonitoring(inputMonitoring);
  }

  onMount(async () => {
    await restorePersistedSources();
    Audio.setMicGain(micGain);
    Audio.setNativeInputChannel(0);
    parts.forEach(part => Audio.setLooperPartSource(part.id, part.source));
    Audio.setInputMonitoring(inputMonitoring);

    const unlistenInfo = await listen("engine-info", (event) => {
      sampleRate = event.payload.sample_rate;
    });

    const unlistenDur = await listen("loop-duration", (event) => {
      loopDurationSamples = event.payload;
    });

    const unlistenWave = await listen("waveform-ready", (event) => {
      const { part_id, data } = event.payload;
      updatePart(part_id, () => ({
        waveform: data || []
      }));
      if (activePart === part_id) drawCurrentState();
    });

    const unlistenActive = await listen("part-active", (event) => {
      activePart = event.payload;
      drawCurrentState();
    });

    const unlistenState = await listen("looper-state", (event) => {
      const { part_id, state } = event.payload;

      if (state === "stopped" || state === "playing" || state === "paused") {
        transportState = state === "stopped" ? "stopped" : "playing";
        if (state === "playing" || state === "paused") {
          playbackStartTime = performance.now();
          startAnimation();
        } else {
          cancelAnimationFrame(animationFrame);
          drawCurrentState();
          if (isPlayingSequence) {
            isPlayingSequence = false;
            activeSequenceStep = -1;
          }
        }
      } else if (part_id === activePart && (state === "recording" || state === "overdubbing")) {
        transportState = state;
      } else if (part_id === activePart && state === "empty" && (transportState === "recording" || transportState === "overdubbing")) {
        transportState = "stopped";
      }

      if (state === "empty" || state === "recorded" || state === "recording" || state === "overdubbing" || state === "playing" || state === "paused") {
        updatePart(part_id, (part) => {
          const captureEndedEmpty = state === "empty" && (part.state === "recording" || part.state === "overdubbing");
          const notice = captureEndedEmpty ? `No material captured from ${getSourceLabel(part.source)}` : "";
          return { state, notice };
        });
      }
    });

    const unlistenLayers = await listen("looper-layers", (event) => {
      const { part_id, layers } = event.payload;
      updatePart(part_id, () => ({ layers }));
    });

    const unlistenSeqStep = await listen("sequence-step", (event) => {
      const { step, part_id } = event.payload;
      activeSequenceStep = step;
      activePart = part_id;
      drawCurrentState();
    });

    const unlistenSeqFinished = await listen("sequence-finished", () => {
      isPlayingSequence = false;
      activeSequenceStep = -1;
    });

    const unlistenSynthParam = await listen("param-change", (event) => {
      const { id, value } = event.payload;
      if (synthParams[id]) {
        synthParams[id].value = value;
        synthParams = [...synthParams];
      }
    });

    const unlistenMpcStep = await listen("mpc-step", (event) => {
      mpcCurrentStep = event.payload;
    });

    const unlistenMpcTransport = await listen("mpc-transport", (event) => {
      mpcPlaying = event.payload;
      if (!mpcPlaying) mpcCurrentStep = 0;
    });

    const unlistenDrum = await listen("drum-trigger", (event) => {
      const padId = event.payload - 36;
      if (padId >= 0 && padId < 16) activeMpcPad = padId;
    });

    return () => {
      unlistenWave();
      unlistenActive();
      unlistenState();
      unlistenLayers();
      unlistenInfo();
      unlistenDur();
      unlistenSeqStep();
      unlistenSeqFinished();
      unlistenSynthParam();
      unlistenMpcStep();
      unlistenMpcTransport();
      unlistenDrum();
      cancelAnimationFrame(animationFrame);
    };
  });

  function addToSequence(id) {
    if (songSequence.length < 16 && parts[id]?.layers > 0) {
      songSequence = [...songSequence, id];
    }
  }

  function removeFromSequence(index) {
    songSequence = songSequence.filter((_, i) => i !== index);
  }

  function toggleSequencePlay() {
    if (learnMode) {
      Audio.learnMidi("transport", 0);
      return;
    }
    if (!isPlayingSequence) {
      const playableSequence = songSequence.filter(id => parts[id]?.layers > 0);
      if (playableSequence.length > 0) {
        isPlayingSequence = true;
        songSequence = playableSequence;
        Audio.playSequence(playableSequence);
      }
    } else {
      isPlayingSequence = false;
      Audio.stopSequence();
      activeSequenceStep = -1;
    }
  }

  function selectPart(id) {
    if (learnMode) {
      Audio.learnMidi("looper_rec", id);
      return;
    }
    activePart = id;
    Audio.selectPart(id);
    drawCurrentState();
  }

  function toggleRecord() {
    if (learnMode) {
      Audio.learnMidi("transport", 2);
      return;
    }
    Audio.toggleLooper(activePart);
  }

  function togglePartActive(partId) {
    if (learnMode || parts[partId]?.layers === 0) return;
    Audio.toggleLooperPartActive(partId);
  }

  function togglePlay() {
    if (learnMode) {
      Audio.learnMidi("transport", 0);
      return;
    }
    if (transportState === "playing" || transportState === "recording" || transportState === "overdubbing" || isPlayingSequence) {
      Audio.stop();
      transportState = "stopped";
      isPlayingSequence = false;
      activeSequenceStep = -1;
    } else {
      if (!hasAnyRecordedPart) return;
      Audio.play();
      transportState = "playing";
    }
  }

  function updateMicGain() {
    Audio.setMicGain(micGain);
  }

  function updatePartSource(partId, value) {
    updatePart(partId, () => ({ source: value, notice: "" }));
    Audio.setLooperPartSource(partId, value);
  }

  function handlePartKeydown(event, partId) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      selectPart(partId);
    }
  }

  function toggleMetronome() {
    const bpm = normalizedMetroBpm();
    isMetronomeEnabled.update(v => {
      const next = !v;
      metroEnabled = next;
      persistMetronomeState(next, bpm);
      return next;
    });
  }

  function updateMetronomeBpm() {
    metroBpm = normalizedMetroBpm();
    metronomeBpm.set(metroBpm);
    Audio.setMetronome(metroEnabled, metroBpm);
    persistMetronomeState(metroEnabled, metroBpm);
  }

  function updatePartVolume(partId, value) {
    updatePart(partId, () => ({ volume: value }));
    Audio.setParam(12 + partId, value);
  }

  function updateLooperFx(effectId, value) {
    const next = Number(value);
    updatePart(activePart, part => {
      const fxValues = [...part.fxValues];
      fxValues[effectId] = next;
      return { fxValues };
    });
    Audio.setLooperFx(activePart, effectId, next);
  }

  function applyLooperFxPreset(preset) {
    updatePart(activePart, () => ({ fxValues: [...preset.values] }));
    preset.values.forEach((value, effectId) => {
      Audio.setLooperFx(activePart, effectId, value);
    });
  }

  function loadSynthPreset(name) {
    const preset = SYNTH_PRESETS[name];
    if (!preset) return;
    preset.forEach(param => {
      Audio.setParam(param.id, param.value);
      synthParams[param.id].value = param.value;
    });
    synthParams = [...synthParams];
  }

  function updateSynthParam(id, value) {
    const next = Number(value);
    synthParams[id].value = next;
    synthParams = [...synthParams];
    Audio.setParam(id, next);
  }

  function startSynthNote(midi) {
    if (learnMode || heldNotes.has(midi)) return;
    heldNotes.add(midi);
    heldNotes = new Set(heldNotes);
    Audio.synthNoteOn(midi, 110).catch(e => console.error("Failed to start synth note", e));
  }

  function stopSynthNote(midi) {
    if (!heldNotes.has(midi)) return;
    heldNotes.delete(midi);
    heldNotes = new Set(heldNotes);
    Audio.synthNoteOff(midi).catch(e => console.error("Failed to stop synth note", e));
  }

  function stopAllSynthNotes() {
    for (const midi of heldNotes) {
      Audio.synthNoteOff(midi).catch(() => {});
    }
    heldNotes.clear();
    heldNotes = new Set();
  }

  function loadMpcKit(kitId) {
    const kit = MPC_KITS.find(item => item.id === Number(kitId)) || MPC_KITS[0];
    activeMpcKit = kit;
    Audio.setMpcKit(kit.id);
  }

  function triggerMpcPad(padId) {
    if (learnMode) {
      Audio.learnMidi("note", 36 + padId);
      return;
    }
    activeMpcPad = padId;
    Audio.mpcPadOn(padId, 100);
    setTimeout(() => Audio.mpcPadOff(padId), 50);
  }

  function toggleMpcStep(step) {
    const next = !compactMpcSteps[activeMpcPad][step];
    compactMpcSteps[activeMpcPad][step] = next;
    compactMpcSteps = [...compactMpcSteps];
    Audio.setMpcStep(activeMpcPad, step, next);
  }

  function toggleMpcSequencer() {
    if (mpcPlaying) {
      Audio.stopMpcSequencer();
    } else {
      Audio.startMpcSequencer(Number(mpcBpm), Number(mpcSwing));
    }
  }

  function playJamChord(chord) {
    Audio.playChord(chord.notes, metroBpm);
  }

  function drawWaveformOnly(data, colorClass) {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    const width = canvas.width;
    const height = canvas.height;
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = "#1c1917";
    ctx.fillRect(0, 0, width, height);

    if (!data || data.length === 0) {
      ctx.fillStyle = "#44403c";
      ctx.font = "12px monospace";
      ctx.textAlign = "center";
      ctx.fillText("NO DATA", width / 2, height / 2);
      return;
    }

    let strokeColor = "#22d3ee";
    if (colorClass.includes("red")) strokeColor = "#ef4444";
    if (colorClass.includes("green")) strokeColor = "#4ade80";
    if (colorClass.includes("blue")) strokeColor = "#60a5fa";
    if (colorClass.includes("yellow")) strokeColor = "#facc15";
    if (colorClass.includes("cyan")) strokeColor = "#22d3ee";
    if (colorClass.includes("fuchsia")) strokeColor = "#e879f9";

    ctx.beginPath();
    ctx.strokeStyle = strokeColor;
    ctx.lineWidth = 2;
    const step = width / data.length;
    const amp = height / 2;
    for (let i = 0; i < data.length; i += 1) {
      const x = i * step;
      const val = data[i];
      const y = val * amp * 0.9;
      ctx.moveTo(x, amp - y);
      ctx.lineTo(x, amp + y);
    }
    ctx.stroke();
    ctx.beginPath();
    ctx.strokeStyle = "#44403c";
    ctx.lineWidth = 1;
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();
  }

  onDestroy(stopAllSynthNotes);
</script>

<div class="h-full flex flex-col gap-6">
  <div class="flex-1 min-h-48 bg-stone-900 rounded-xl border border-stone-800 relative overflow-hidden flex items-center justify-center">
    <canvas bind:this={canvas} width="800" height="200" class="w-full h-full object-cover"></canvas>
    <div class="absolute top-4 left-4 text-stone-600 font-mono text-xs uppercase tracking-widest pointer-events-none">
      DISPLAY: {activePartData.label}
    </div>
    <div class="absolute top-4 right-4 text-right pointer-events-none">
      <div class="text-[10px] font-black uppercase tracking-widest {activePartData.color}">{activePartData.label}</div>
      <div class="text-[10px] font-mono text-stone-500 uppercase">{getSourceLabel(activePartData.source)}</div>
    </div>
  </div>

  <div class="bg-stone-900 border border-stone-800 rounded-xl p-4 flex flex-col gap-4">
    <div class="flex flex-col md:flex-row md:justify-between md:items-center gap-3 px-2">
      <div class="flex items-center gap-3">
        <h3 class="text-[10px] font-black text-stone-500 uppercase tracking-widest">Song Sequencer</h3>
        <button on:click={toggleSequencePlay} class="px-4 py-1.5 rounded-full text-[10px] font-black uppercase tracking-tighter transition-all {isPlayingSequence ? 'bg-green-600 text-white animate-pulse' : 'bg-stone-800 text-stone-400 hover:text-white'} {learnMode ? 'ring-2 ring-orange-500' : ''}">
          {isPlayingSequence ? "Stop Seq" : "Play Seq"}
        </button>
      </div>
      <div class="flex flex-wrap gap-2">
        {#each parts as part}
          <button on:click={() => addToSequence(part.id)} disabled={part.layers === 0} class="px-2 py-1 rounded bg-stone-800 text-[9px] font-bold {part.color} border border-stone-700 hover:bg-stone-700 disabled:opacity-40 disabled:cursor-not-allowed">
            + {part.key}
          </button>
        {/each}
      </div>
    </div>
    <div class="flex gap-2 overflow-x-auto min-h-[40px] bg-stone-950 p-2 rounded-lg border border-stone-800 shadow-inner">
      {#if songSequence.length === 0}
        <span class="text-[10px] text-stone-700 font-bold uppercase m-auto">Sequence Empty</span>
      {/if}
      {#each songSequence as sid, i}
        <button on:click={() => removeFromSequence(i)} class="shrink-0 px-3 py-1 rounded border text-[10px] font-black {parts[sid]?.color} hover:bg-red-900/30 hover:border-red-500 hover:text-white transition-all group relative {activeSequenceStep === i ? 'border-orange-500 bg-orange-900/30 ring-1 ring-orange-500' : 'border-stone-700 bg-stone-800'}">
          Part {parts[sid]?.key}
          <span class="absolute -top-1 -right-1 opacity-0 group-hover:opacity-100 text-[8px] bg-red-600 text-white rounded-full w-3 h-3 flex items-center justify-center">x</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-5 gap-4">
    {#each parts as part}
      <div
        role="button"
        tabindex="0"
        on:click={() => selectPart(part.id)}
        on:keydown={(event) => handlePartKeydown(event, part.id)}
        class="min-h-36 rounded-xl border-2 flex flex-col items-center justify-center transition-all relative cursor-pointer px-3 py-4 {activePart === part.id ? 'border-orange-500 bg-stone-800' : 'border-stone-800 bg-stone-900 opacity-70 hover:opacity-100'} {learnMode ? 'ring-2 ring-orange-500' : ''}"
      >
        {#if part.state === "recorded" || part.state === "playing" || part.state === "overdubbing"}
          <div class="absolute top-2 right-2 w-2.5 h-2.5 rounded-full bg-green-500 {transportState === 'playing' && part.waveform?.length > 0 ? 'animate-pulse' : ''}"></div>
        {:else if part.state === "paused"}
          <div class="absolute top-2 right-2 w-2.5 h-2.5 rounded-full bg-yellow-500"></div>
        {:else if part.state === "recording"}
          <div class="absolute top-2 right-2 w-2.5 h-2.5 rounded-full bg-red-500 animate-pulse"></div>
        {/if}
        <button
          type="button"
          on:click|stopPropagation={() => togglePartActive(part.id)}
          disabled={part.layers === 0}
          class="absolute top-2 left-2 px-2 py-1 rounded border text-[9px] font-black uppercase transition-colors {part.state === 'paused' ? 'border-yellow-600 bg-yellow-900/30 text-yellow-400' : 'border-green-700 bg-green-900/20 text-green-400'} disabled:opacity-30 disabled:cursor-not-allowed"
        >
          {part.state === "paused" ? "Paused" : "On"}
        </button>
        <span class="font-black text-2xl {part.color}">{part.label}</span>
        <span class="text-xs font-bold text-stone-500 uppercase mt-1">{learnMode ? "CLICK TO MAP" : part.state}</span>
        <span class="text-[10px] font-mono text-stone-600 uppercase mt-1">{part.layers} {part.layers === 1 ? "layer" : "layers"}</span>
        {#if part.notice}
          <span class="text-[10px] font-bold text-yellow-500 uppercase mt-1 text-center leading-tight">{part.notice}</span>
        {/if}
        <label class="mt-3 w-full flex flex-col gap-1 text-[9px] font-black uppercase text-stone-500">
          Source
          <select
            aria-label={`${part.label} recording source`}
            value={part.source}
            on:click|stopPropagation
            on:keydown|stopPropagation
            on:change={(event) => updatePartSource(part.id, event.currentTarget.value)}
            class="w-full bg-stone-950 border border-stone-700 rounded px-2 py-1 text-[10px] font-bold text-cyan-400 outline-none"
          >
            {#each SOURCE_OPTIONS as option}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </label>
      </div>
    {/each}
  </div>

  <div class="bg-stone-900 rounded-xl p-4 border border-stone-800 flex flex-col gap-4">
    <div class="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-3">
      <div>
        <h3 class="text-[10px] font-black text-stone-500 uppercase tracking-widest">Instrument Deck</h3>
        <p class="text-[10px] text-stone-600 font-bold uppercase mt-1">Choose Synth, MPC, Jam, or Instrument Mix as any part source, then record that part.</p>
      </div>
      <div class="grid grid-cols-4 gap-2">
        {#each ["input", "synth", "mpc", "jam"] as tab}
          <button
            type="button"
            on:click={() => activeInstrumentTab = tab}
            class="px-3 py-2 rounded border text-[10px] font-black uppercase transition-colors {activeInstrumentTab === tab ? 'bg-orange-500 text-stone-950 border-orange-400' : 'bg-stone-800 border-stone-700 text-stone-400 hover:text-white'}"
          >
            {tab}
          </button>
        {/each}
      </div>
    </div>

    {#if activeInstrumentTab === "input"}
      <div class="grid grid-cols-1 lg:grid-cols-[1fr_2fr] gap-4">
        <div class="flex flex-col gap-3">
          <div class="flex justify-between mb-1">
            <label for="looper-mic-gain" class="text-[10px] font-bold text-stone-500 uppercase">Monitor Gain</label>
            <span class="text-[10px] font-mono text-orange-500">{micGain}x</span>
          </div>
          <input id="looper-mic-gain" type="range" min="0" max="5" step="0.1" bind:value={micGain} on:input={updateMicGain} class="w-full h-1 bg-stone-700 rounded-lg appearance-none accent-orange-500 cursor-pointer">
          <button on:click={toggleMonitoring} class="px-3 py-2 rounded border text-xs font-bold uppercase transition-all {inputMonitoring ? 'bg-green-900/30 text-green-500 border-green-700' : 'border-stone-700 text-stone-400 hover:text-white hover:bg-stone-800'} {learnMode ? 'border-orange-500 text-orange-500' : ''}">{inputMonitoring ? "MON ON" : "Monitor"}</button>
        </div>
        <div class="grid grid-cols-1 sm:grid-cols-5 gap-2">
          {#each parts as part}
            <button type="button" on:click={() => selectPart(part.id)} class="rounded border border-stone-800 bg-stone-950 p-3 text-left hover:border-orange-500 transition-colors">
              <span class="block text-[10px] font-black uppercase {part.color}">{part.label}</span>
              <span class="block text-[10px] font-mono uppercase text-stone-400 mt-1">{getSourceLabel(part.source)}</span>
            </button>
          {/each}
        </div>
      </div>
    {:else if activeInstrumentTab === "synth"}
      <div class="flex flex-col gap-4">
        <div class="flex flex-col lg:flex-row lg:items-center gap-3">
          <select on:change={(event) => loadSynthPreset(event.currentTarget.value)} class="bg-stone-950 text-cyan-400 font-bold text-xs rounded px-3 py-2 border border-stone-700 outline-none focus:border-cyan-500">
            {#each Object.keys(SYNTH_PRESETS) as name}
              <option value={name}>{name}</option>
            {/each}
          </select>
          <div class="grid grid-cols-2 md:grid-cols-4 xl:grid-cols-8 gap-2 flex-1">
            {#each synthParams as param}
              <label class="flex flex-col gap-1 rounded border border-stone-800 bg-stone-950 p-2">
                <span class="text-[9px] font-black uppercase text-stone-500">{param.name} {Math.round(param.value * 100)}%</span>
                <input type="range" min="0" max="1" step="0.01" value={param.value} on:input={(event) => updateSynthParam(param.id, event.currentTarget.value)} class="w-full h-1 bg-stone-700 rounded-lg appearance-none accent-cyan-500 cursor-pointer">
              </label>
            {/each}
          </div>
        </div>
        <div class="grid h-24 gap-px select-none" style="grid-template-columns: repeat({COMPACT_KEYS.length}, minmax(0, 1fr));">
          {#each COMPACT_KEYS as key}
            <button
              type="button"
              aria-label={`Play ${key.name}`}
              on:pointerdown={() => startSynthNote(key.midi)}
              on:pointerup={() => stopSynthNote(key.midi)}
              on:pointercancel={() => stopSynthNote(key.midi)}
              on:pointerleave={() => stopSynthNote(key.midi)}
              on:blur={() => stopSynthNote(key.midi)}
              class="relative h-full rounded-b border border-stone-700 border-b-4 transition-colors active:scale-[0.99] {key.black ? 'bg-stone-950 text-stone-400 hover:bg-cyan-950' : 'bg-stone-100 text-stone-900 hover:bg-cyan-100'} {heldNotes.has(key.midi) ? 'ring-2 ring-cyan-400 bg-cyan-300 text-stone-950' : ''}"
            >
              <span class="absolute bottom-2 left-1/2 -translate-x-1/2 text-[8px] font-black">{key.name}</span>
            </button>
          {/each}
        </div>
      </div>
    {:else if activeInstrumentTab === "mpc"}
      <div class="grid grid-cols-1 xl:grid-cols-[1fr_1fr] gap-4">
        <div class="flex flex-col gap-3">
          <div class="flex flex-col sm:flex-row sm:items-end sm:justify-between gap-3">
            <div class="flex flex-wrap gap-2">
              {#each MPC_KITS as kit}
                <button type="button" on:click={() => loadMpcKit(kit.id)} class="px-3 py-1.5 rounded text-[10px] font-bold uppercase transition-all {activeMpcKit.id === kit.id ? 'bg-cyan-600 text-white' : 'bg-stone-800 text-stone-500 hover:text-white'}">
                  {kit.name}
                </button>
              {/each}
            </div>
            <button type="button" on:click={toggleMpcSequencer} class="px-4 py-2 rounded border text-[10px] font-black uppercase {mpcPlaying ? 'bg-red-600 text-white border-red-500' : 'bg-stone-800 text-stone-300 border-stone-700 hover:text-white'}">
              {mpcPlaying ? "Stop MPC" : "Play MPC"}
            </button>
          </div>
          <div class="grid grid-cols-4 gap-2">
            {#each activeMpcKit.padNames as padName, padId}
              <button
                type="button"
                on:mousedown={() => triggerMpcPad(padId)}
                class="aspect-square rounded-lg border-b-4 border-stone-950 bg-stone-800 text-stone-500 flex flex-col items-center justify-center transition-all active:translate-y-1 active:border-b-0 {activeMpcPad === padId ? 'bg-cyan-600 text-white border-cyan-800 shadow-[0_0_12px_rgba(34,211,238,0.45)]' : 'hover:bg-stone-700 hover:text-stone-200'}"
              >
                <span class="text-lg font-black">{padId + 1}</span>
                <span class="text-[8px] font-bold uppercase opacity-60 truncate max-w-full px-1">{padName}</span>
              </button>
            {/each}
          </div>
        </div>
        <div class="flex flex-col gap-4 bg-stone-950 rounded-xl border border-stone-800 p-4">
          <div class="flex flex-wrap items-end justify-between gap-3">
            <div>
              <div class="text-[9px] font-bold text-stone-600 uppercase tracking-widest">Selected Pad</div>
              <div class="text-lg font-black text-stone-200">{activeMpcKit.padNames[activeMpcPad]}</div>
            </div>
            <label class="block">
              <span class="block text-stone-500 text-[10px] font-bold uppercase tracking-widest mb-1">BPM</span>
              <input type="number" min="40" max="240" bind:value={mpcBpm} class="w-20 bg-stone-800 border border-stone-700 rounded px-2 py-1 text-right text-xs font-mono text-stone-200 outline-none focus:border-cyan-500">
            </label>
            <label class="block">
              <span class="block text-stone-500 text-[10px] font-bold uppercase tracking-widest mb-1">Swing</span>
              <input type="range" min="0" max="0.5" step="0.05" bind:value={mpcSwing} on:change={() => Audio.setMpcParam(0, Number(mpcSwing))} class="w-28 h-1 bg-stone-800 appearance-none rounded-full accent-cyan-500">
            </label>
          </div>
          <div class="grid grid-cols-8 gap-2">
            {#each compactMpcSteps[activeMpcPad] as stepActive, step}
              <button
                type="button"
                on:click={() => toggleMpcStep(step)}
                class="h-9 rounded border transition-all relative {stepActive ? 'bg-cyan-500 border-cyan-400 shadow-[0_0_10px_rgba(34,211,238,0.4)]' : 'bg-stone-800 border-stone-700 hover:bg-stone-700'} {mpcCurrentStep === step ? 'ring-2 ring-white z-10' : ''}"
              >
                {#if step % 4 === 0}
                  <span class="absolute top-0.5 left-1 text-[8px] font-black opacity-30">{Math.floor(step / 4) + 1}</span>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      </div>
    {:else}
      <div class="grid grid-cols-1 lg:grid-cols-[1fr_2fr] gap-4">
        <div class="flex flex-col gap-2">
          <label class="block">
            <span class="block text-stone-500 text-[10px] font-bold uppercase tracking-widest mb-1">Jam Sound</span>
            <select on:change={(event) => Audio.setJamSound(event.currentTarget.value)} class="w-full bg-stone-950 border border-stone-700 rounded px-3 py-2 text-xs font-bold text-cyan-400 outline-none">
              <option value="0">Piano</option>
              <option value="1">E-Piano</option>
              <option value="2">Organ</option>
            </select>
          </label>
          <button type="button" on:click={() => Audio.stopChord()} class="px-3 py-2 rounded border border-stone-700 bg-stone-800 text-xs font-bold uppercase text-stone-300 hover:text-white">Stop Jam</button>
        </div>
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
          {#each JAM_CHORDS as chord}
            <button type="button" on:mousedown={() => playJamChord(chord)} on:mouseup={() => Audio.stopChord()} on:mouseleave={() => Audio.stopChord()} class="h-24 rounded-lg border border-stone-700 bg-stone-800 text-stone-200 font-black text-2xl hover:border-orange-500 hover:text-orange-400 active:scale-[0.98] transition-all">
              {chord.label}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <div class="bg-stone-900 rounded-xl p-6 border border-stone-800 flex flex-col lg:flex-row justify-between items-center gap-6">
    <div class="flex items-center gap-4">
      <div class="flex flex-col items-center gap-1">
        <button on:click={toggleMetronome} class="px-3 py-2 rounded-lg text-xs font-bold uppercase transition-all {metroEnabled ? 'bg-yellow-600 text-white' : 'bg-stone-800 text-stone-400 hover:text-white'}">
          Metro
        </button>
        <div class="flex items-center gap-1">
          <input type="number" min="40" max="240" bind:value={metroBpm} on:input={updateMetronomeBpm} class="w-12 bg-stone-800 text-center text-yellow-500 font-mono text-xs rounded border border-stone-700">
          <span class="text-[10px] text-stone-500 font-bold">BPM</span>
        </div>
      </div>

      <div class="relative">
        {#if learnMode}
          <button type="button" aria-label="Map MIDI record control" on:click={() => Audio.learnMidi("transport", 2)} class="absolute inset-0 z-50 bg-stone-900/80 rounded-full flex items-center justify-center border-2 border-orange-500 cursor-crosshair animate-pulse">
            <span class="text-[8px] font-black text-orange-500">MAP</span>
          </button>
        {/if}
        <button on:click={toggleRecord} aria-label="Toggle looper record" class="w-16 h-16 rounded-full flex items-center justify-center shadow-lg transition-all active:scale-95 {transportState === 'recording' ? 'bg-red-600 animate-pulse' : (transportState === 'overdubbing' ? 'bg-yellow-500' : 'bg-stone-800 border border-stone-700 hover:bg-red-900/30 hover:border-red-500 hover:text-red-500')}">
          <div class="w-4 h-4 rounded-full {transportState === 'recording' ? 'bg-white' : 'bg-current'}"></div>
        </button>
      </div>

      <div class="relative">
        {#if learnMode}
          <button type="button" aria-label="Map MIDI play control" on:click={() => Audio.learnMidi("transport", 0)} class="absolute inset-0 z-50 bg-stone-900/80 rounded-full flex items-center justify-center border-2 border-orange-500 cursor-crosshair animate-pulse">
            <span class="text-[10px] font-black text-orange-500">MAP</span>
          </button>
        {/if}
        <button on:click={togglePlay} aria-label="Toggle looper playback" class="w-20 h-20 rounded-full bg-stone-200 text-stone-950 flex items-center justify-center font-black shadow-xl hover:scale-105 active:scale-95 transition-transform">
          {#if transportState === "playing" || transportState === "recording" || transportState === "overdubbing" || isPlayingSequence}
            <div class="w-6 h-6 bg-black rounded-sm"></div>
          {:else}
            <div class="w-0 h-0 border-t-[12px] border-t-transparent border-l-[20px] border-l-black border-b-[12px] border-b-transparent ml-1"></div>
          {/if}
        </button>
      </div>
    </div>

    <div class="flex justify-end gap-2">
      <div class="relative">
        {#if learnMode}
          <button type="button" aria-label="Map MIDI undo control" on:click={() => Audio.learnMidi("looper_undo", activePart)} class="absolute inset-0 z-50 bg-stone-900/80 rounded-lg flex items-center justify-center border border-orange-500 cursor-crosshair">
            <span class="text-[8px] font-black text-orange-500">MAP</span>
          </button>
        {/if}
        <button
          on:click={() => Audio.undo(activePart)}
          disabled={!activePartHasUndoableMaterial}
          class="p-3 rounded-lg bg-stone-800 text-stone-400 hover:text-white border border-stone-700 font-bold text-xs uppercase transition-all active:scale-95 disabled:opacity-40 disabled:hover:text-stone-400 disabled:cursor-not-allowed"
        >Undo</button>
      </div>

      <div class="relative">
        {#if learnMode}
          <button type="button" aria-label="Map MIDI clear control" on:click={() => Audio.learnMidi("looper_clear", activePart)} class="absolute inset-0 z-50 bg-stone-900/80 rounded-lg flex items-center justify-center border border-orange-500 cursor-crosshair">
            <span class="text-[8px] font-black text-orange-500">MAP</span>
          </button>
        {/if}
        <button
          on:click={() => Audio.clearPart(activePart)}
          disabled={!activePartHasUndoableMaterial}
          class="p-3 rounded-lg bg-stone-800 text-stone-400 hover:text-red-500 border border-stone-700 font-bold text-xs uppercase hover:border-red-500 transition-all active:scale-95 disabled:opacity-40 disabled:hover:text-stone-400 disabled:hover:border-stone-700 disabled:cursor-not-allowed"
        >Clear</button>
      </div>
    </div>
  </div>

  <div class="bg-stone-900 rounded-xl p-4 border border-stone-800 flex flex-wrap justify-center gap-6">
    {#each parts as part}
      <div class="flex flex-col items-center gap-2 w-16">
        <span class="text-[10px] font-black {part.color} uppercase">{part.key}</span>
        <div class="relative h-24 w-6 bg-stone-800 rounded-full overflow-hidden border border-stone-700">
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={part.volume}
            on:input={(event) => updatePartVolume(part.id, parseFloat(event.currentTarget.value))}
            class="absolute w-full h-24 opacity-0 cursor-pointer"
            style="writing-mode: vertical-lr; direction: rtl;"
          >
          <div
            class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-stone-600 to-stone-500 transition-all"
            style="height: {part.volume * 100}%"
          ></div>
        </div>
        <span class="text-[10px] font-mono text-stone-400">{Math.round(part.volume * 100)}%</span>
      </div>
    {/each}
  </div>

  <div class="bg-stone-900 rounded-xl p-4 border border-stone-800 flex flex-col gap-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h3 class="text-[10px] font-black text-stone-500 uppercase tracking-widest">Loop FX Rack: {activePartData.label}</h3>
        <p class="text-[10px] text-stone-600 font-bold uppercase mt-1">Per-part insert effects after the recorded loop</p>
      </div>
      <div class="flex flex-wrap gap-2 justify-end">
        {#each looperFxPresets as preset}
          <button
            type="button"
            on:click={() => applyLooperFxPreset(preset)}
            class="px-3 py-1.5 rounded border border-stone-700 bg-stone-800 text-[9px] font-black uppercase text-stone-300 hover:text-white hover:border-orange-500 transition-colors"
          >
            {preset.name}
          </button>
        {/each}
      </div>
    </div>

    <div class="grid grid-cols-2 md:grid-cols-4 xl:grid-cols-7 gap-3">
      {#each looperFxControls as control}
        <label class="flex flex-col gap-1 rounded border border-stone-800 bg-stone-950 p-2">
          <div class="flex items-center justify-between gap-2">
            <span class="text-[10px] font-bold text-stone-500 uppercase">{control.label}</span>
            <span class="text-[10px] font-mono text-stone-300">{control.valueText(activePartData.fxValues[control.id])}</span>
          </div>
          <input
            aria-label={`${activePartData.label} ${control.label}`}
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={activePartData.fxValues[control.id]}
            on:input={(event) => updateLooperFx(control.id, parseFloat(event.currentTarget.value))}
            class="w-full h-1 bg-stone-700 rounded-lg appearance-none cursor-pointer {control.accent}"
          >
        </label>
      {/each}
    </div>
  </div>
</div>
