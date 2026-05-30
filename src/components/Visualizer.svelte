<script>
  import { onDestroy, onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { Audio } from "../lib/audio";
  import { BACKING_TRACKS, NOTES, getChordNotes, getMidiNumber, getScaleNotes, normalizeNoteName, transposeTrackToKey } from "../lib/data";
  import { jamVisualizerState } from "../lib/stores";
  import TheoryVisualizer from "./TheoryVisualizer.svelte";

  const SCALE_LABELS = {
    major: "Major",
    minor: "Minor",
    pentatonic_maj: "Pentatonic Major",
    pentatonic_min: "Pentatonic Minor",
    blues: "Blues"
  };

  let rootKey = "C";
  let scaleType = "minor";
  let viewMode = "guitar";
  let selectedTrackId = BACKING_TRACKS[0]?.id || "";
  let selectedChordIndex = 0;
  let showChordTones = true;
  let showCommonTones = true;
  let liveJamTrack = null;
  let liveJamPlaying = false;
  let liveJamChordLabel = "";
  let unlistenJamChordStep = () => {};

  const unsubscribeJamState = jamVisualizerState.subscribe(state => {
    if (!state?.track) {
      liveJamPlaying = false;
      return;
    }

    liveJamTrack = state.track;
    liveJamPlaying = Boolean(state.isPlaying);
    liveJamChordLabel = state.currentChordLabel || "";
    selectedTrackId = state.track.id || selectedTrackId;
    selectedChordIndex = Number.isInteger(state.currentChordIndex) ? state.currentChordIndex : 0;
    rootKey = state.rootKey || rootKey;
    scaleType = state.scaleType || scaleType;
    viewMode = state.viewMode || viewMode;
  });

  onMount(async () => {
    unlistenJamChordStep = await listen("jam-chord-step", (event) => {
      const payload = event.payload || {};
      const nextIndex = Number(payload.index);
      if (!Number.isInteger(nextIndex)) return;
      selectedChordIndex = nextIndex;
      liveJamChordLabel = payload.label || liveJamChordLabel;
    });
  });

  onDestroy(() => {
    unsubscribeJamState();
    unlistenJamChordStep();
  });

  $: sourceTrack = liveJamTrack || BACKING_TRACKS.find(track => track.id === selectedTrackId) || BACKING_TRACKS[0];
  $: selectedTrack = sourceTrack ? transposeTrackToKey(sourceTrack, rootKey, scaleType) : null;
  $: selectedChord = selectedTrack?.progression?.[selectedChordIndex] || selectedTrack?.progression?.[0] || null;
  $: scaleNotes = getScaleNotes(rootKey, scaleType);
  $: chordNotes = selectedChord ? normalizeNotes(selectedChord.notes?.length ? selectedChord.notes : getChordNotes(selectedChord.name)) : [];
  $: commonNotes = scaleNotes.filter(note => chordNotes.includes(note));

  function normalizeNotes(notes) {
    return notes
      .map(note => normalizeNoteName(String(note).replace(/\d/g, "")))
      .filter(note => NOTES.includes(note));
  }

  function setTrack(id) {
    liveJamTrack = null;
    liveJamPlaying = false;
    liveJamChordLabel = "";
    selectedTrackId = id;
    selectedChordIndex = 0;
    const track = BACKING_TRACKS.find(item => item.id === id);
    if (!track) return;

    if (track.key?.endsWith("m")) {
      rootKey = normalizeNoteName(track.key.slice(0, -1));
      scaleType = "minor";
    } else if (track.key) {
      rootKey = normalizeNoteName(track.key);
      scaleType = "major";
    }
  }

  function trackToAudioChords(track) {
    return (track?.progression || []).map(chord => ({
      notes: (Array.isArray(chord.notes) && chord.notes.length ? chord.notes : getChordNotes(chord.name)).map(note => getMidiNumber(note)),
      beats: Number(chord.beats) || 4,
      name: chord.name
    }));
  }

  async function applyVisualizerKeyChange() {
    if (!sourceTrack) return;
    const transposedTrack = transposeTrackToKey(sourceTrack, rootKey, scaleType);
    selectedChordIndex = Math.max(0, Math.min(selectedChordIndex, transposedTrack.progression.length - 1));
    liveJamChordLabel = transposedTrack.progression[selectedChordIndex]?.name || liveJamChordLabel;

    if (liveJamPlaying) {
      liveJamTrack = transposedTrack;
      await Audio.playJamTrack(trackToAudioChords(transposedTrack), transposedTrack.bpm);
      jamVisualizerState.set({
        track: transposedTrack,
        isPlaying: true,
        currentChordIndex: selectedChordIndex,
        currentChordNotes: normalizeNotes(transposedTrack.progression[selectedChordIndex]?.notes || []),
        currentChordLabel: transposedTrack.progression[selectedChordIndex]?.name || "",
        rootKey,
        scaleType,
        viewMode
      });
    }
  }

  function nextChord() {
    if (!selectedTrack?.progression?.length) return;
    selectedChordIndex = (selectedChordIndex + 1) % selectedTrack.progression.length;
  }

  function previousChord() {
    if (!selectedTrack?.progression?.length) return;
    selectedChordIndex = (selectedChordIndex - 1 + selectedTrack.progression.length) % selectedTrack.progression.length;
  }
</script>

<div class="h-full grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_22rem] gap-6 overflow-hidden">
  <section class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg overflow-hidden flex flex-col">
    <header class="p-6 border-b border-stone-800 flex flex-col gap-5">
      <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-4">
        <div>
          <span class="text-xs font-bold text-cyan-400 uppercase tracking-widest">Advanced Visualizer</span>
          <h2 class="text-4xl font-black font-serif text-white mt-2">FRETBOARD + KEYS</h2>
          <p class="text-stone-400 mt-2 max-w-2xl">
            See scale notes, chord tones, roots, and common tones across guitar and piano layouts.
          </p>
        </div>

        <div class="flex gap-2 bg-stone-950 border border-stone-800 rounded p-1">
          <button on:click={() => viewMode = "guitar"} class="px-4 py-2 rounded text-sm font-bold uppercase {viewMode === 'guitar' ? 'bg-cyan-600 text-black' : 'text-stone-400 hover:text-white'}">Guitar</button>
          <button on:click={() => viewMode = "piano"} class="px-4 py-2 rounded text-sm font-bold uppercase {viewMode === 'piano' ? 'bg-cyan-600 text-black' : 'text-stone-400 hover:text-white'}">Piano</button>
        </div>
      </div>

      <div class="grid grid-cols-1 lg:grid-cols-4 gap-3">
        <label class="flex flex-col gap-2 text-[10px] font-bold uppercase tracking-widest text-stone-500">
          Track
          <select bind:value={selectedTrackId} on:change={(event) => setTrack(event.currentTarget.value)} class="bg-stone-950 border border-stone-700 rounded px-3 py-2 text-white text-sm outline-none focus:border-cyan-400">
            {#each BACKING_TRACKS as track}
              <option value={track.id}>{track.title}</option>
            {/each}
          </select>
        </label>

        <label class="flex flex-col gap-2 text-[10px] font-bold uppercase tracking-widest text-stone-500">
          Key
          <select bind:value={rootKey} on:change={applyVisualizerKeyChange} class="bg-stone-950 border border-stone-700 rounded px-3 py-2 text-white text-sm outline-none focus:border-cyan-400">
            {#each NOTES as item}
              <option value={item}>{item}</option>
            {/each}
          </select>
        </label>

        <label class="flex flex-col gap-2 text-[10px] font-bold uppercase tracking-widest text-stone-500">
          Scale
          <select bind:value={scaleType} on:change={applyVisualizerKeyChange} class="bg-stone-950 border border-stone-700 rounded px-3 py-2 text-white text-sm outline-none focus:border-cyan-400">
            {#each Object.entries(SCALE_LABELS) as [id, label]}
              <option value={id}>{label}</option>
            {/each}
          </select>
        </label>

        <div class="flex flex-col gap-2">
          <span class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Overlays</span>
          <div class="flex gap-2">
            <button on:click={() => showChordTones = !showChordTones} class="flex-1 px-3 py-2 rounded border text-xs font-bold uppercase {showChordTones ? 'bg-orange-500 text-black border-orange-400' : 'bg-stone-950 text-stone-400 border-stone-700'}">Chord</button>
            <button on:click={() => showCommonTones = !showCommonTones} class="flex-1 px-3 py-2 rounded border text-xs font-bold uppercase {showCommonTones ? 'bg-green-500 text-black border-green-400' : 'bg-stone-950 text-stone-400 border-stone-700'}">Common</button>
          </div>
        </div>
      </div>
    </header>

    <div class="flex-1 min-h-0 p-6 flex flex-col gap-5 overflow-hidden">
      <div class="bg-stone-950 border border-stone-800 rounded-lg p-4 flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Current Chord</div>
          <div class="text-3xl font-black font-serif text-white mt-1">{liveJamChordLabel || selectedChord?.name || "No chord"}</div>
          <div class="text-sm text-stone-500 mt-1">{selectedTrack?.title} / {selectedTrack?.genre} / {selectedTrack?.bpm} BPM</div>
          {#if liveJamPlaying}
            <div class="text-[10px] font-bold uppercase tracking-widest text-green-400 mt-2">Following Jam Station</div>
          {/if}
        </div>

        <div class="flex items-center gap-2">
          <button on:click={previousChord} class="bg-stone-800 hover:bg-stone-700 text-white px-4 py-2 rounded border border-stone-700 font-bold">Prev</button>
          <span class="text-sm font-mono text-stone-500 min-w-16 text-center">{selectedChordIndex + 1} / {selectedTrack?.progression?.length || 0}</span>
          <button on:click={nextChord} class="bg-stone-800 hover:bg-stone-700 text-white px-4 py-2 rounded border border-stone-700 font-bold">Next</button>
        </div>
      </div>

      <div class="flex-1 min-h-0 bg-stone-950 border border-stone-800 rounded-lg p-5 overflow-hidden">
        <TheoryVisualizer
          {rootKey}
          {scaleType}
          {viewMode}
          currentChordNotes={chordNotes}
          currentChordLabel={liveJamChordLabel || selectedChord?.name || ""}
          {showChordTones}
          {showCommonTones}
          contextLabel="Study Map"
        />
      </div>
    </div>
  </section>

  <aside class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg p-6 overflow-y-auto custom-scrollbar">
    <h3 class="text-xs font-bold text-cyan-400 uppercase tracking-widest">Theory Map</h3>

    <div class="grid gap-3 mt-5">
      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Scale</div>
        <div class="text-xl font-black text-white mt-2">{rootKey} {SCALE_LABELS[scaleType]}</div>
        <div class="flex flex-wrap gap-2 mt-3">
          {#each scaleNotes as item}
            <span class="px-2 py-1 rounded bg-stone-800 text-stone-200 text-xs font-bold {item === rootKey ? 'ring-2 ring-cyan-400' : ''}">{item}</span>
          {/each}
        </div>
      </div>

      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Chord Tones</div>
        <div class="flex flex-wrap gap-2 mt-3">
          {#each chordNotes as item}
            <span class="px-2 py-1 rounded bg-orange-500 text-black text-xs font-bold">{item}</span>
          {/each}
        </div>
      </div>

      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Common Tones</div>
        <div class="flex flex-wrap gap-2 mt-3">
          {#each commonNotes as item}
            <span class="px-2 py-1 rounded bg-green-500 text-black text-xs font-black">{item}</span>
          {:else}
            <span class="text-sm text-stone-500">No shared notes in this selection.</span>
          {/each}
        </div>
      </div>
    </div>

    <div class="h-px bg-stone-800 my-6"></div>

    <div class="grid gap-3 text-sm text-stone-400">
      <p>Cyan marks the selected root. Gray notes are in the selected scale.</p>
      <p>Orange marks the selected chord tones. Green rings mark tones shared by the scale and chord.</p>
      <p>When Jam Station is playing, this follows the current progression and chord step for practice.</p>
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
