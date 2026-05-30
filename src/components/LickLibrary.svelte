<script>
  import { onDestroy, onMount } from "svelte";
  import { Audio } from "../lib/audio";
  import { getMidiNumber, NOTES } from "../lib/data";
  import { jamHandoffTrack } from "../lib/stores";
  import { LICK_LIBRARY, lickToJamTrack, stringFretToMidi } from "../lib/lickLibraryData";

  const FRETS = Array.from({ length: 16 }, (_, index) => index);
  const STRING_ROWS = [
    { index: 5, label: "e", name: "E4" },
    { index: 4, label: "B", name: "B3" },
    { index: 3, label: "G", name: "G3" },
    { index: 2, label: "D", name: "D3" },
    { index: 1, label: "A", name: "A2" },
    { index: 0, label: "E", name: "E2" }
  ];
  const DOT_FRETS = new Set([3, 5, 7, 9, 12, 15]);
  const MIN_NOTE_MS = 70;
  const TRAINER_STEP = 0.05;
  const TRAINER_MAX = 1.5;

  let search = "";
  let artistFilter = "";
  let styleFilter = "";
  let activeLickId = LICK_LIBRARY[0]?.id || "";
  let playbackRate = 1;
  let backingTrackEnabled = true;
  let speedTrainerEnabled = false;
  let restoringState = true;
  let mounted = false;
  let persistFingerprint = "";
  let persistHandle = null;

  let isPlaying = false;
  let currentChord = "-";
  let currentProgressionIndex = 0;
  let elapsedSeconds = 0;
  let activeNoteKeys = new Set();
  let heldMidi = new Set();
  let timers = [];
  let ticker = null;
  let startedAt = 0;
  let cycleTotalMs = 0;
  let playbackBeatSeconds = 0;
  let handoffStatus = "";

  $: activeLick = LICK_LIBRARY.find(lick => lick.id === activeLickId) || LICK_LIBRARY[0];
  $: artists = [...new Set(LICK_LIBRARY.map(lick => lick.artist))];
  $: styles = [...new Set(LICK_LIBRARY.map(lick => lick.style))];
  $: filteredLicks = LICK_LIBRARY.filter(matchesFilters);
  $: noteMarkers = activeLick ? buildNoteMarkers(activeLick) : [];
  $: markerMap = new Map(noteMarkers.map(marker => [marker.key, marker]));
  $: adjustedBpm = activeLick ? Math.round(activeLick.bpm * Number(playbackRate || 1)) : 0;
  $: timeDisplay = `${Math.floor(elapsedSeconds / 60)}:${String(Math.floor(elapsedSeconds % 60)).padStart(2, "0")}`;
  $: moduleState = {
    schema_version: 1,
    activeLickId,
    search,
    artistFilter,
    styleFilter,
    playbackRate: Number(playbackRate),
    backingTrackEnabled,
    speedTrainerEnabled
  };
  $: if (mounted && !restoringState) queuePersist(moduleState);

  onMount(() => {
    mounted = true;
    restoreModuleState();

    return () => {
      stopPlayback();
      if (persistHandle) window.clearTimeout(persistHandle);
    };
  });

  onDestroy(() => {
    stopPlayback();
  });

  async function restoreModuleState() {
    try {
      const persisted = await Audio.loadPersistence();
      const saved = persisted?.module_state?.lickLibrary;
      if (saved) {
        const savedLickExists = LICK_LIBRARY.some(lick => lick.id === saved.activeLickId);
        activeLickId = savedLickExists ? saved.activeLickId : activeLickId;
        search = saved.search || "";
        artistFilter = saved.artistFilter || "";
        styleFilter = saved.styleFilter || "";
        playbackRate = Number.isFinite(Number(saved.playbackRate)) ? Number(saved.playbackRate) : playbackRate;
        backingTrackEnabled = saved.backingTrackEnabled !== false;
        speedTrainerEnabled = Boolean(saved.speedTrainerEnabled);
      }
    } catch (error) {
      console.error("Failed to restore Lick Library state:", error);
    } finally {
      persistFingerprint = JSON.stringify(moduleState);
      restoringState = false;
    }
  }

  function queuePersist(state) {
    const nextFingerprint = JSON.stringify(state);
    if (nextFingerprint === persistFingerprint) return;

    persistFingerprint = nextFingerprint;
    if (persistHandle) window.clearTimeout(persistHandle);
    persistHandle = window.setTimeout(() => {
      Audio.saveModuleState("lickLibrary", state)
        .catch(error => console.error("Failed to persist Lick Library state:", error));
    }, 180);
  }

  function matchesFilters(lick) {
    const query = search.trim().toLowerCase();
    const haystack = `${lick.name} ${lick.artist} ${lick.style} ${lick.key} ${lick.description}`.toLowerCase();
    const matchesSearch = !query || haystack.includes(query);
    const matchesArtist = !artistFilter || lick.artist === artistFilter;
    const matchesStyle = !styleFilter || lick.style === styleFilter;

    return matchesSearch && matchesArtist && matchesStyle;
  }

  function selectLick(id) {
    if (id === activeLickId) return;
    stopPlayback();
    activeLickId = id;
    currentChord = "-";
    currentProgressionIndex = 0;
    elapsedSeconds = 0;
    handoffStatus = "";
  }

  function noteNameFromMidi(midi) {
    if (!Number.isFinite(midi)) return "";
    return NOTES[((midi % 12) + 12) % 12];
  }

  function noteKey(event) {
    return `${event.s}-${event.f}`;
  }

  function buildNoteMarkers(lick) {
    const markers = new Map();

    for (const event of lick.sequence || []) {
      const fret = Number(event.f);
      const stringIndex = Number(event.s);
      if (!Number.isInteger(stringIndex) || !Number.isFinite(fret) || fret < 0 || fret > 15) continue;

      const midi = stringFretToMidi(event);
      if (midi === null) continue;

      const key = noteKey(event);
      const existing = markers.get(key);
      if (existing) {
        existing.count += 1;
        if (!existing.types.includes(event.type)) existing.types.push(event.type);
      } else {
        markers.set(key, {
          key,
          stringIndex,
          fret,
          midi,
          note: noteNameFromMidi(midi),
          count: 1,
          types: [event.type]
        });
      }
    }

    return [...markers.values()];
  }

  function markerFor(stringIndex, fret) {
    return markerMap.get(`${stringIndex}-${fret}`);
  }

  function markerClass(marker) {
    if (activeNoteKeys.has(marker.key)) {
      return "bg-orange-500 text-black border-orange-200 scale-110 shadow-[0_0_18px_rgba(249,115,22,0.55)]";
    }

    if (marker.types.includes("bend")) return "bg-purple-500 text-white border-purple-300";
    if (marker.types.includes("vibrato")) return "bg-cyan-500 text-black border-cyan-200";
    if (marker.types.includes("slide-down")) return "bg-emerald-500 text-black border-emerald-200";
    return "bg-stone-700 text-stone-100 border-stone-500";
  }

  function articulationLabel(types) {
    if (types.includes("bend")) return "Bend";
    if (types.includes("vibrato")) return "Vibrato";
    if (types.includes("slide-down")) return "Slide";
    return "Pick";
  }

  function getTotalBeats(lick) {
    const progressionBeats = (lick.progression || []).reduce((total, chord) => total + (Number(chord.beats) || 0), 0);
    const sequenceEnd = (lick.sequence || []).reduce((end, event) => {
      const eventEnd = Number(event.t || 0) + Number(event.d || 0);
      return Math.max(end, eventEnd);
    }, 0);

    return Math.max(progressionBeats, sequenceEnd, 1);
  }

  function chordAtBeat(lick, beat) {
    let cursor = 0;
    const progression = lick.progression || [];

    for (let index = 0; index < progression.length; index += 1) {
      const chord = progression[index];
      const beats = Number(chord.beats) || 4;
      if (beat >= cursor && beat < cursor + beats) {
        return { label: chord.chord, index };
      }
      cursor += beats;
    }

    const lastIndex = Math.max(progression.length - 1, 0);
    return { label: progression[lastIndex]?.chord || "-", index: lastIndex };
  }

  function trackToAudioChords(track) {
    return (track?.progression || []).map(chord => ({
      name: chord.name,
      beats: Number(chord.beats) || 4,
      notes: (chord.notes || []).map(note => getMidiNumber(note))
    }));
  }

  async function startBackingTrack(lick, rate) {
    if (!backingTrackEnabled) return;

    const track = lickToJamTrack(lick);
    const chords = trackToAudioChords(track);
    if (!chords.length) return;

    try {
      await Audio.playJamTrack(chords, Math.round(lick.bpm * rate));
    } catch (error) {
      console.error("Failed to start Lick Library backing track:", error);
    }
  }

  function togglePlayback() {
    if (isPlaying) {
      stopPlayback();
      return;
    }

    startPlayback();
  }

  function startPlayback() {
    if (!activeLick) return;

    stopPlayback();

    const rate = Math.max(0.5, Math.min(TRAINER_MAX, Number(playbackRate) || 1));
    playbackRate = rate;
    playbackBeatSeconds = 60 / (activeLick.bpm * rate);
    cycleTotalMs = getTotalBeats(activeLick) * playbackBeatSeconds * 1000;
    startedAt = performance.now();
    isPlaying = true;
    elapsedSeconds = 0;
    activeNoteKeys = new Set();

    const firstChord = chordAtBeat(activeLick, 0);
    currentChord = firstChord.label;
    currentProgressionIndex = firstChord.index;

    startBackingTrack(activeLick, rate);

    for (const event of activeLick.sequence || []) {
      const delayMs = Math.max(0, Number(event.t || 0) * playbackBeatSeconds * 1000);
      const durationMs = Math.max(MIN_NOTE_MS, Number(event.d || 0) * playbackBeatSeconds * 1000);
      const handle = window.setTimeout(() => startScheduledNote(event, durationMs), delayMs);
      timers.push(handle);
    }

    ticker = window.setInterval(updateTransport, 60);
    timers.push(window.setTimeout(finishPlaybackCycle, cycleTotalMs + 120));
  }

  function startScheduledNote(event, durationMs) {
    if (!isPlaying) return;

    const midi = stringFretToMidi(event);
    if (midi === null) return;

    const key = noteKey(event);
    activeNoteKeys.add(key);
    activeNoteKeys = new Set(activeNoteKeys);
    heldMidi.add(midi);
    heldMidi = new Set(heldMidi);

    Audio.noteOn(midi, velocityFor(event))
      .catch(error => console.error("Failed to start lick note:", error));

    const releaseHandle = window.setTimeout(() => stopScheduledNote(midi, key), durationMs);
    timers.push(releaseHandle);
  }

  function velocityFor(event) {
    if (event.type === "bend") return 118;
    if (event.type === "vibrato") return 112;
    if (event.type === "slide-down") return 104;
    return 108;
  }

  function stopScheduledNote(midi, key) {
    Audio.noteOff(midi).catch(error => console.error("Failed to stop lick note:", error));
    heldMidi.delete(midi);
    heldMidi = new Set(heldMidi);
    activeNoteKeys.delete(key);
    activeNoteKeys = new Set(activeNoteKeys);
  }

  function updateTransport() {
    if (!isPlaying || !activeLick || !playbackBeatSeconds) return;

    elapsedSeconds = Math.min((performance.now() - startedAt) / 1000, cycleTotalMs / 1000);
    const beat = elapsedSeconds / playbackBeatSeconds;
    const chord = chordAtBeat(activeLick, beat);
    currentChord = chord.label;
    currentProgressionIndex = chord.index;
  }

  function finishPlaybackCycle() {
    const shouldRepeat = speedTrainerEnabled && Boolean(activeLick);

    clearScheduledTimers();
    stopHeldNotes();
    activeNoteKeys = new Set();
    isPlaying = false;
    Audio.stopChord().catch(() => {});

    if (shouldRepeat) {
      playbackRate = Math.min(TRAINER_MAX, Number((Number(playbackRate) + TRAINER_STEP).toFixed(2)));
      timers.push(window.setTimeout(startPlayback, 180));
      return;
    }

    currentChord = "-";
    currentProgressionIndex = 0;
    elapsedSeconds = 0;
  }

  function clearScheduledTimers() {
    for (const handle of timers) window.clearTimeout(handle);
    timers = [];

    if (ticker) {
      window.clearInterval(ticker);
      ticker = null;
    }
  }

  function stopHeldNotes() {
    for (const midi of heldMidi) {
      Audio.noteOff(midi).catch(() => {});
    }
    heldMidi = new Set();
  }

  function stopPlayback() {
    clearScheduledTimers();
    stopHeldNotes();
    activeNoteKeys = new Set();
    isPlaying = false;
    currentChord = "-";
    currentProgressionIndex = 0;
    elapsedSeconds = 0;
    Audio.stopChord().catch(() => {});
  }

  function handleRateChange() {
    playbackRate = Math.max(0.5, Math.min(TRAINER_MAX, Number(playbackRate) || 1));
    if (isPlaying) startPlayback();
  }

  async function toggleBandMode() {
    backingTrackEnabled = !backingTrackEnabled;
    if (!isPlaying) return;

    if (backingTrackEnabled) {
      await startBackingTrack(activeLick, Number(playbackRate) || 1);
    } else {
      Audio.stopChord().catch(() => {});
    }
  }

  function toggleSpeedTrainer() {
    speedTrainerEnabled = !speedTrainerEnabled;
  }

  function handoffToJam() {
    const track = lickToJamTrack(activeLick);
    if (!track) return;

    jamHandoffTrack.set(track);
    handoffStatus = `${track.title} queued for Jam Station`;
  }
</script>

<div class="h-full min-h-0 grid grid-cols-1 xl:grid-cols-[20rem_minmax(0,1fr)] gap-4 overflow-hidden">
  <aside class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg flex flex-col overflow-hidden">
    <div class="p-4 border-b border-stone-800 space-y-3">
      <div>
        <div class="text-[10px] font-bold uppercase tracking-widest text-orange-500">Lick Library</div>
        <h2 class="mt-1 text-xl font-black text-white">MVP Studio</h2>
      </div>

      <label class="block">
        <span class="sr-only">Search licks</span>
        <input
          type="search"
          bind:value={search}
          placeholder="Search licks"
          class="w-full bg-stone-950 border border-stone-700 rounded px-3 py-2 text-sm text-white outline-none focus:border-orange-500" />
      </label>

      <div class="grid grid-cols-2 gap-2">
        <label>
          <span class="sr-only">Artist filter</span>
          <select bind:value={artistFilter} class="w-full bg-stone-800 border border-stone-700 rounded px-2 py-2 text-xs text-stone-200 outline-none">
            <option value="">All Artists</option>
            {#each artists as artist}
              <option value={artist}>{artist}</option>
            {/each}
          </select>
        </label>

        <label>
          <span class="sr-only">Style filter</span>
          <select bind:value={styleFilter} class="w-full bg-stone-800 border border-stone-700 rounded px-2 py-2 text-xs text-stone-200 outline-none">
            <option value="">All Styles</option>
            {#each styles as style}
              <option value={style}>{style}</option>
            {/each}
          </select>
        </label>
      </div>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar p-2">
      {#if filteredLicks.length === 0}
        <div class="border border-dashed border-stone-800 rounded p-5 text-center text-xs text-stone-500">No matching licks</div>
      {:else}
        <div class="grid gap-1">
          {#each filteredLicks as lick}
            <button
              type="button"
              on:click={() => selectLick(lick.id)}
              class="text-left rounded border px-3 py-3 transition-colors {activeLick?.id === lick.id ? 'bg-stone-800 border-orange-500/60' : 'bg-transparent border-transparent hover:bg-stone-800 hover:border-stone-700'}">
              <div class="flex items-start justify-between gap-3">
                <span class="font-bold text-sm text-white leading-tight">{lick.name}</span>
                <span class="shrink-0 rounded border border-stone-700 bg-stone-950 px-1.5 py-0.5 text-[10px] font-bold text-stone-400">{lick.key}</span>
              </div>
              <div class="mt-1 flex items-center justify-between gap-2 text-[10px] uppercase tracking-wider text-stone-500">
                <span>{lick.artist}</span>
                <span>{lick.bpm} BPM</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </aside>

  <main class="min-h-0 overflow-y-auto custom-scrollbar flex flex-col gap-4">
    {#if activeLick}
      <section class="bg-stone-900 border border-stone-800 rounded-lg p-5">
        <div class="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-4">
          <div>
            <div class="flex flex-wrap items-center gap-2 text-[10px] font-bold uppercase tracking-widest">
              <span class="text-orange-500">{activeLick.artist}</span>
              <span class="text-stone-600">/</span>
              <span class="text-stone-400">{activeLick.style}</span>
              <span class="text-stone-600">/</span>
              <span class="text-stone-400">{activeLick.key}</span>
            </div>
            <h1 class="mt-2 text-3xl font-black text-white tracking-tight">{activeLick.name}</h1>
            <p class="mt-2 max-w-3xl text-sm text-stone-400">{activeLick.description}</p>
          </div>

          <div class="grid grid-cols-3 gap-2 min-w-64">
            <div class="rounded border border-stone-800 bg-stone-950 px-3 py-2">
              <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Tempo</div>
              <div class="text-xl font-black text-white">{adjustedBpm}</div>
            </div>
            <div class="rounded border border-stone-800 bg-stone-950 px-3 py-2">
              <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Rate</div>
              <div class="text-xl font-black text-white">{Math.round(Number(playbackRate) * 100)}%</div>
            </div>
            <div class="rounded border border-stone-800 bg-stone-950 px-3 py-2">
              <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Time</div>
              <div class="text-xl font-mono font-black text-white">{timeDisplay}</div>
            </div>
          </div>
        </div>
      </section>

      <section class="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_18rem] gap-4">
        <div class="bg-stone-900 border border-stone-800 rounded-lg p-4 flex flex-col md:flex-row md:items-center md:justify-between gap-4">
          <div class="flex items-center gap-4">
            <button
              type="button"
              on:click={togglePlayback}
              class="w-14 h-14 rounded-full bg-orange-500 text-black font-black text-lg flex items-center justify-center hover:bg-orange-400 active:scale-95 transition-transform"
              aria-label={isPlaying ? "Stop lick playback" : "Play lick"}>
              {#if isPlaying}
                <span class="block w-5 h-5 rounded-sm bg-black"></span>
              {:else}
                <span class="ml-1 block w-0 h-0 border-l-[16px] border-l-black border-y-[10px] border-y-transparent"></span>
              {/if}
            </button>

            <div class="min-w-52">
              <div class="flex items-center justify-between text-[10px] uppercase tracking-widest font-bold text-stone-500">
                <span>Speed</span>
                <span class="text-stone-200">{Math.round(Number(playbackRate) * 100)}%</span>
              </div>
              <input
                type="range"
                min="0.5"
                max={TRAINER_MAX}
                step="0.05"
                bind:value={playbackRate}
                on:change={handleRateChange}
                class="mt-2 w-full accent-orange-500" />
            </div>
          </div>

          <div class="flex flex-wrap gap-2">
            <button
              type="button"
              on:click={toggleBandMode}
              class="px-3 py-2 rounded border text-xs font-bold uppercase tracking-widest transition-colors {backingTrackEnabled ? 'bg-cyan-500 text-black border-cyan-300' : 'bg-stone-800 text-stone-400 border-stone-700'}">
              Band {backingTrackEnabled ? "On" : "Off"}
            </button>
            <button
              type="button"
              on:click={toggleSpeedTrainer}
              class="px-3 py-2 rounded border text-xs font-bold uppercase tracking-widest transition-colors {speedTrainerEnabled ? 'bg-orange-500 text-black border-orange-300' : 'bg-stone-800 text-stone-400 border-stone-700'}">
              Trainer {speedTrainerEnabled ? "On" : "Off"}
            </button>
            <button
              type="button"
              on:click={handoffToJam}
              class="px-3 py-2 rounded border border-stone-700 bg-stone-800 text-stone-200 hover:bg-stone-700 text-xs font-bold uppercase tracking-widest">
              Send to Jam
            </button>
          </div>
        </div>

        <div class="bg-stone-900 border border-stone-800 rounded-lg p-4">
          <div class="text-[10px] uppercase tracking-widest font-bold text-stone-500">Current Chord</div>
          <div class="mt-2 text-4xl font-black text-white">{currentChord === "-" ? activeLick.progression[0]?.chord : currentChord}</div>
          {#if handoffStatus}
            <div class="mt-2 text-[10px] font-bold uppercase tracking-widest text-cyan-300">{handoffStatus}</div>
          {/if}
        </div>
      </section>

      <section class="bg-stone-900 border border-stone-800 rounded-lg p-4 overflow-hidden">
        <div class="flex items-center justify-between gap-3 mb-3">
          <h2 class="text-xs font-bold uppercase tracking-widest text-stone-500">Fretboard</h2>
          <div class="flex flex-wrap gap-2 text-[10px] font-bold uppercase tracking-widest text-stone-500">
            <span class="text-cyan-300">Vibrato</span>
            <span class="text-purple-300">Bend</span>
            <span class="text-emerald-300">Slide</span>
          </div>
        </div>

        <div class="overflow-x-auto custom-scrollbar">
          <div class="min-w-[58rem] select-none">
            <div class="grid text-[10px] text-stone-500 font-mono mb-1" style="grid-template-columns: 3rem repeat(16, minmax(2.8rem, 1fr));">
              <div></div>
              {#each FRETS as fret}
                <div class="text-center">{fret}</div>
              {/each}
            </div>

            {#each STRING_ROWS as row}
              <div class="grid border-b border-stone-800 last:border-b-0" style="grid-template-columns: 3rem repeat(16, minmax(2.8rem, 1fr));">
                <div class="h-12 bg-stone-950 border-r-4 border-stone-500 flex flex-col items-center justify-center">
                  <span class="text-xs font-black text-stone-300">{row.label}</span>
                  <span class="text-[9px] text-stone-600">{row.name}</span>
                </div>
                {#each FRETS as fret}
                  {@const marker = markerFor(row.index, fret)}
                  <div class="relative h-12 border-r border-stone-800 flex items-center justify-center {fret === 0 ? 'bg-stone-950/60' : 'bg-stone-900'}">
                    <div class="absolute left-0 right-0 top-1/2 h-px bg-stone-600"></div>
                    {#if DOT_FRETS.has(fret)}
                      <div class="absolute bottom-1 left-1/2 w-1 h-1 -translate-x-1/2 rounded-full bg-stone-700"></div>
                    {/if}
                    {#if marker}
                      <div
                        class="relative z-10 w-8 h-8 rounded-full border flex items-center justify-center text-[10px] font-black transition-all {markerClass(marker)}"
                        title={`${marker.note} / fret ${marker.fret} / ${articulationLabel(marker.types)}`}>
                        {marker.note}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/each}
          </div>
        </div>
      </section>

      <section class="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_22rem] gap-4">
        <div class="bg-stone-900 border border-stone-800 rounded-lg p-5 overflow-hidden">
          <div class="flex items-center justify-between gap-3 mb-4">
            <h2 class="text-xs font-bold uppercase tracking-widest text-stone-500">Tablature</h2>
            <span class="text-xs font-bold text-orange-500">{activeLick.key}</span>
          </div>
          <pre class="tab-font overflow-x-auto custom-scrollbar whitespace-pre text-sm leading-8 text-stone-200 bg-stone-950 border border-stone-800 rounded p-4">{activeLick.tab}</pre>
        </div>

        <div class="bg-stone-900 border border-stone-800 rounded-lg p-5">
          <h2 class="text-xs font-bold uppercase tracking-widest text-stone-500">Theory</h2>
          <p class="mt-4 text-sm leading-relaxed text-stone-300">{activeLick.theory}</p>

          <div class="mt-5 grid gap-2">
            {#each noteMarkers as marker}
              <div class="flex items-center justify-between gap-3 rounded border border-stone-800 bg-stone-950 px-3 py-2">
                <div class="flex items-center gap-2">
                  <span class="w-7 h-7 rounded-full flex items-center justify-center text-[10px] font-black border {markerClass(marker)}">{marker.note}</span>
                  <span class="text-xs font-bold text-stone-300">String {marker.stringIndex + 1}, fret {marker.fret}</span>
                </div>
                <span class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">{articulationLabel(marker.types)}</span>
              </div>
            {/each}
          </div>
        </div>
      </section>

      <section class="flex gap-3 overflow-x-auto custom-scrollbar pb-1">
        {#each activeLick.progression as chord, index}
          <div class="min-w-40 rounded border px-4 py-3 transition-colors {currentProgressionIndex === index && (isPlaying || currentChord !== '-') ? 'bg-orange-500 text-black border-orange-300' : 'bg-stone-900 text-stone-300 border-stone-800'}">
            <div class="text-[10px] uppercase tracking-widest font-bold opacity-70">Beat {index + 1}</div>
            <div class="mt-1 text-2xl font-black">{chord.chord}</div>
            <div class="mt-1 text-[10px] uppercase tracking-widest font-bold opacity-70">{chord.beats} beats</div>
          </div>
        {/each}
      </section>
    {/if}
  </main>
</div>

<style>
  .tab-font {
    font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;
  }

  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
    height: 4px;
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
