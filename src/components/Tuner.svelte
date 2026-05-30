<script>
  import { onDestroy, onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { NOTES } from "../lib/data";

  let active = false;
  let status = "Mic stopped";
  let note = "-";
  let cents = 0;
  let frequency = 0;
  let rms = 0;
  let a4 = 440;
  let audioCtx = null;
  let analyser = null;
  let micSource = null;
  let channelSplitter = null;
  let micStream = null;
  let rafId = null;
  let audioInputs = [];
  let selectedDeviceId = "";
  let selectedChannel = "mix";
  let activeDeviceLabel = "System default";
  let activeChannelCount = 0;

  $: inTune = active && note !== "-" && Math.abs(cents) < 5;
  $: centsLabel = cents > 0 ? `+${cents}` : `${cents}`;
  $: needleOffset = Math.max(-50, Math.min(50, cents));
  $: selectedDeviceLabel = audioInputs.find(device => device.deviceId === selectedDeviceId)?.label || "System default";

  onMount(() => {
    let unlistenNativeTuner = () => {};
    loadAudioInputs();

    listen("tuner-reading", (event) => {
      const payload = event.payload || {};
      const nativeFrequency = Number(payload.frequency) || 0;
      const nativeRms = Number(payload.rms) || 0;

      active = true;
      rms = nativeRms;
      activeDeviceLabel = "Native audio input";

      if (nativeFrequency > 0) {
        frequency = nativeFrequency;
        const noteNumber = 12 * (Math.log(nativeFrequency / a4) / Math.log(2)) + 69;
        const rounded = Math.round(noteNumber);
        const noteIndex = ((rounded % 12) + 12) % 12;
        note = NOTES[noteIndex];
        cents = Math.round((noteNumber - rounded) * 100);
        status = "Native pitch locked";
      } else if (nativeRms < 0.003) {
        frequency = 0;
        note = "-";
        cents = 0;
        status = "Native input waiting for signal";
      } else {
        frequency = 0;
        note = "-";
        cents = 0;
        status = "Native input searching";
      }
    }).then(unlisten => {
      unlistenNativeTuner = unlisten;
    }).catch(e => {
      console.error("Failed to listen for native tuner readings", e);
    });

    if (navigator.mediaDevices?.addEventListener) {
      navigator.mediaDevices.addEventListener("devicechange", loadAudioInputs);
    }

    return () => {
      unlistenNativeTuner();
      if (navigator.mediaDevices?.removeEventListener) {
        navigator.mediaDevices.removeEventListener("devicechange", loadAudioInputs);
      }
    };
  });

  async function loadAudioInputs() {
    if (!navigator.mediaDevices?.enumerateDevices) {
      status = "Audio input selection unavailable";
      return;
    }

    try {
      const devices = await navigator.mediaDevices.enumerateDevices();
      audioInputs = devices.filter(device => device.kind === "audioinput");
      if (selectedDeviceId && !audioInputs.some(device => device.deviceId === selectedDeviceId)) {
        selectedDeviceId = "";
      }
    } catch (e) {
      console.error("Failed to enumerate audio inputs", e);
      status = "Could not list audio inputs";
    }
  }

  function getAudioConstraints() {
    const audio = {
      echoCancellation: false,
      noiseSuppression: false,
      autoGainControl: false,
      channelCount: { ideal: 2 }
    };

    if (selectedDeviceId) {
      audio.deviceId = { exact: selectedDeviceId };
    }

    return { audio };
  }

  async function startTuner() {
    if (active) return;

    try {
      audioCtx = audioCtx || new (window.AudioContext || window.webkitAudioContext)();
      if (audioCtx.state === "suspended") {
        await audioCtx.resume();
      }

      try {
        micStream = await navigator.mediaDevices.getUserMedia(getAudioConstraints());
      } catch (firstError) {
        if (!selectedDeviceId) throw firstError;
        console.warn("Selected tuner input failed, retrying with system default", firstError);
        selectedDeviceId = "";
        micStream = await navigator.mediaDevices.getUserMedia(getAudioConstraints());
      }
      await loadAudioInputs();

      const [track] = micStream.getAudioTracks();
      const settings = track?.getSettings?.() || {};
      activeDeviceLabel = track?.label || selectedDeviceLabel;
      activeChannelCount = settings.channelCount || 0;

      analyser = audioCtx.createAnalyser();
      analyser.fftSize = 4096;
      micSource = audioCtx.createMediaStreamSource(micStream);
      connectMicGraph();

      active = true;
      status = `Listening: ${activeDeviceLabel}`;
      updatePitch();
    } catch (e) {
      console.error("Failed to start tuner", e);
      status = "Microphone access failed";
      stopTuner();
    }
  }

  function stopTuner() {
    active = false;
    status = "Mic stopped";
    note = "-";
    cents = 0;
    frequency = 0;
    rms = 0;
    activeChannelCount = 0;
    activeDeviceLabel = selectedDeviceLabel;

    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }

    if (micSource) {
      micSource.disconnect();
      micSource = null;
    }

    if (channelSplitter) {
      channelSplitter.disconnect();
      channelSplitter = null;
    }

    if (micStream) {
      micStream.getTracks().forEach(track => track.stop());
      micStream = null;
    }
  }

  function connectMicGraph() {
    if (!micSource || !analyser || !audioCtx) return;

    if (channelSplitter) {
      channelSplitter.disconnect();
      channelSplitter = null;
    }

    if (selectedChannel === "input1" || selectedChannel === "input2") {
      try {
        channelSplitter = audioCtx.createChannelSplitter(Math.max(2, activeChannelCount || 2));
        micSource.connect(channelSplitter);
        channelSplitter.connect(analyser, selectedChannel === "input2" ? 1 : 0);
        return;
      } catch (e) {
        console.warn("Tuner channel split failed, falling back to input mix", e);
        if (channelSplitter) {
          channelSplitter.disconnect();
          channelSplitter = null;
        }
      }
    }

    micSource.connect(analyser);
  }

  async function restartTuner() {
    if (!active) return;
    stopTuner();
    await startTuner();
  }

  function updatePitch() {
    if (!active || !analyser || !audioCtx) return;

    const buffer = new Float32Array(analyser.fftSize);
    analyser.getFloatTimeDomainData(buffer);
    const pitch = autoCorrelate(buffer, audioCtx.sampleRate);

    rms = getRms(buffer);

    if (pitch !== -1) {
      frequency = pitch;
      const noteNumber = 12 * (Math.log(pitch / a4) / Math.log(2)) + 69;
      const rounded = Math.round(noteNumber);
      const noteIndex = ((rounded % 12) + 12) % 12;
      note = NOTES[noteIndex];
      cents = Math.round((noteNumber - rounded) * 100);
      status = "Pitch locked";
    } else if (rms < 0.003) {
      status = "Waiting for signal";
    } else {
      status = "Searching";
    }

    rafId = requestAnimationFrame(updatePitch);
  }

  function getRms(buffer) {
    let sum = 0;
    for (let i = 0; i < buffer.length; i += 1) {
      sum += buffer[i] * buffer[i];
    }
    return Math.sqrt(sum / buffer.length);
  }

  function autoCorrelate(buffer, sampleRate) {
    const size = buffer.length;
    const volume = getRms(buffer);
    if (volume < 0.003) return -1;

    let start = 0;
    let end = size - 1;
    const threshold = Math.max(0.02, Math.min(0.2, volume * 0.6));

    for (let i = 0; i < size / 2; i += 1) {
      if (Math.abs(buffer[i]) < threshold) {
        start = i;
        break;
      }
    }

    for (let i = 1; i < size / 2; i += 1) {
      if (Math.abs(buffer[size - i]) < threshold) {
        end = size - i;
        break;
      }
    }

    const trimmed = buffer.slice(start, end);
    const trimmedSize = trimmed.length;
    const correlations = new Array(trimmedSize).fill(0);

    for (let lag = 0; lag < trimmedSize; lag += 1) {
      for (let i = 0; i < trimmedSize - lag; i += 1) {
        correlations[lag] += trimmed[i] * trimmed[i + lag];
      }
    }

    let dip = 0;
    while (dip < trimmedSize - 1 && correlations[dip] > correlations[dip + 1]) {
      dip += 1;
    }

    let maxValue = -1;
    let maxPosition = -1;
    for (let i = dip; i < trimmedSize; i += 1) {
      if (correlations[i] > maxValue) {
        maxValue = correlations[i];
        maxPosition = i;
      }
    }

    if (maxPosition <= 0) return -1;

    const left = correlations[maxPosition - 1] || 0;
    const center = correlations[maxPosition] || 0;
    const right = correlations[maxPosition + 1] || 0;
    const correction = (right - left) / (2 * (2 * center - right - left));
    return sampleRate / (maxPosition + (Number.isFinite(correction) ? correction : 0));
  }

  onDestroy(stopTuner);
</script>

<div class="h-full grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_22rem] gap-6 overflow-hidden">
  <section class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg overflow-hidden flex flex-col">
    <header class="p-6 border-b border-stone-800 flex flex-col md:flex-row md:items-end md:justify-between gap-4">
      <div>
        <span class="text-xs font-bold text-cyan-400 uppercase tracking-widest">Tuner</span>
        <h2 class="text-4xl font-black font-serif text-white mt-2">PITCH LOCK</h2>
        <p class="text-stone-400 mt-2 max-w-2xl">
          Tune from the microphone with a direct cents readout and stable note target.
        </p>
      </div>

      <div class="grid gap-3 sm:grid-cols-[minmax(12rem,1fr)_9rem_8rem] md:max-w-xl">
        <label class="flex flex-col gap-2 text-[10px] font-bold uppercase tracking-widest text-stone-500">
          Mic Input
          <select
            bind:value={selectedDeviceId}
            on:change={restartTuner}
            class="min-w-0 bg-stone-950 border border-stone-700 rounded px-3 py-2 text-white text-sm font-mono outline-none focus:border-cyan-400">
            <option value="">System Default</option>
            {#each audioInputs as device, index}
              <option value={device.deviceId}>{device.label || `Audio Input ${index + 1}`}</option>
            {/each}
          </select>
        </label>

        <label class="flex flex-col gap-2 text-[10px] font-bold uppercase tracking-widest text-stone-500">
          Interface Ch
          <select
            bind:value={selectedChannel}
            on:change={restartTuner}
            class="bg-stone-950 border border-stone-700 rounded px-3 py-2 text-white text-sm font-mono outline-none focus:border-cyan-400">
            <option value="mix">Input 1+2</option>
            <option value="input1">Input 1</option>
            <option value="input2">Input 2</option>
          </select>
        </label>

        <label class="flex flex-col gap-2 text-[10px] font-bold uppercase tracking-widest text-stone-500">
          A4 Reference
          <input
            type="number"
            min="400"
            max="480"
            step="1"
            bind:value={a4}
            class="bg-stone-950 border border-stone-700 rounded px-3 py-2 text-white text-sm font-mono outline-none focus:border-cyan-400" />
        </label>
      </div>
    </header>

    <div class="flex-1 min-h-0 flex flex-col items-center justify-center p-8">
      <div class="relative mb-10">
        <div class="w-56 h-56 rounded-full border-8 flex items-center justify-center transition-all duration-200 {inTune ? 'border-green-500 shadow-[0_0_34px_rgba(34,197,94,0.28)]' : active ? 'border-cyan-900' : 'border-stone-700'} bg-stone-950">
          <div class="text-center">
            <span class="block text-7xl font-black text-white">{note}</span>
            <span class="block text-sm font-mono text-stone-500 mt-1">{frequency ? `${frequency.toFixed(1)} Hz` : "-- Hz"}</span>
          </div>
        </div>
        {#if active && note !== "-"}
          <div class="absolute -bottom-8 left-1/2 -translate-x-1/2 text-sm font-mono {inTune ? 'text-green-400' : 'text-stone-400'}">
            {centsLabel} ct
          </div>
        {/if}
      </div>

      <div class="w-full max-w-md h-4 bg-stone-800 rounded relative mb-10 overflow-hidden border border-stone-700">
        <div class="absolute left-1/2 top-0 bottom-0 w-0.5 bg-white z-10"></div>
        <div class="absolute top-0 bottom-0 w-3 rounded transition-all duration-100 {inTune ? 'bg-green-500' : 'bg-red-500'}"
          style="left: calc(50% + {needleOffset * 3}px); transform: translateX(-50%)"></div>
      </div>

      <div class="flex flex-wrap justify-center gap-3">
        <button
          on:click={active ? stopTuner : startTuner}
          class="{active ? 'bg-red-500/20 text-red-300 border-red-500/50 hover:bg-red-500/30' : 'bg-cyan-600 text-black border-cyan-500 hover:bg-cyan-500'} px-8 py-4 rounded border font-black uppercase tracking-widest transition-all active:scale-95">
          {active ? "Stop Mic" : "Start Tuner"}
        </button>
      </div>

      <div class="mt-6 text-xs font-mono uppercase tracking-widest text-stone-500">{status}</div>
      {#if activeChannelCount}
        <div class="mt-2 text-[10px] font-mono uppercase tracking-widest text-stone-600">
          Stream channels: {activeChannelCount}
        </div>
      {/if}
    </div>
  </section>

  <aside class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg p-6 overflow-y-auto custom-scrollbar">
    <h3 class="text-xs font-bold text-cyan-400 uppercase tracking-widest">Reading</h3>
    <div class="grid gap-3 mt-5">
      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Target</div>
        <div class="text-3xl font-black text-white mt-1">{note}</div>
      </div>
      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Cents</div>
        <div class="text-3xl font-black mt-1 {inTune ? 'text-green-400' : 'text-white'}">{centsLabel}</div>
      </div>
      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Input</div>
        <div class="h-2 bg-stone-800 rounded mt-3 overflow-hidden">
          <div class="h-full bg-cyan-500 transition-all" style="width: {Math.min(100, rms * 2500)}%"></div>
        </div>
        <div class="text-[10px] uppercase tracking-widest text-stone-600 font-bold mt-3 break-words">{active ? activeDeviceLabel : selectedDeviceLabel}</div>
      </div>
    </div>

    <div class="h-px bg-stone-800 my-6"></div>

    <div class="grid gap-3 text-sm text-stone-400">
      <p>Green means the pitch is within 5 cents of the nearest chromatic note.</p>
      <p>Use a clear single note. Chords, heavy distortion, and room noise can confuse pitch detection.</p>
      <p>If your interface has two inputs, choose the interface above, then try Input 1, Input 2, or Input 1+2 depending on where the instrument is plugged in.</p>
      <p>The tuner uses the system microphone directly; it does not route signal through the Rust audio engine yet.</p>
    </div>
  </aside>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #292524;
    border-radius: 8px;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #44403c;
  }
</style>
