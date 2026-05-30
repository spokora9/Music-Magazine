<script>
  import { NOTES, SCALES, normalizeNoteName } from "../lib/data";
  import {
    STRING_TUNING, STRING_LABELS, FRET_COUNT,
    buildFretboard, getScalePositions, getDiatonicChords, getProgressionsForScale
  } from "../lib/scaleShapes";
  import { SCALE_LABELS } from "../lib/modalMixture";
  import { jamHandoffTrack } from "../lib/stores";
  import { Audio } from "../lib/audio";

  // ── Controls ──────────────────────────────────────────────────────────────
  let selectedKey   = "C";
  let selectedScale = "pentatonic_min";
  let viewMode      = "simple";      // "simple" | "positions"
  let labelMode     = "degree";      // "degree" | "note"
  let selectedPos   = 0;             // index within positions array (positions view)
  let playingChord  = null;

  // ── Derived data ──────────────────────────────────────────────────────────
  $: root         = normalizeNoteName(selectedKey) || "C";
  $: fretboard    = buildFretboard(root, selectedScale);
  $: positions    = getScalePositions(root, selectedScale);
  $: diatonic     = getDiatonicChords(root, selectedScale);
  $: progressions = getProgressionsForScale(root, selectedScale);
  $: activePos    = positions[selectedPos] || positions[0];
  $: scaleLabel   = SCALE_LABELS[selectedScale] || selectedScale.replace(/_/g, " ");

  // Beginner-friendly scales shown first; all others still reachable
  const BEGINNER_SCALES = ["pentatonic_min", "pentatonic_maj", "blues", "major", "minor"];
  const ALL_SCALE_KEYS  = Object.keys(SCALES);
  $: orderedScales = [
    ...BEGINNER_SCALES.filter(s => SCALES[s]),
    ...ALL_SCALE_KEYS.filter(s => !BEGINNER_SCALES.includes(s))
  ];

  // ── Helpers ───────────────────────────────────────────────────────────────
  function dotClass(note) {
    if (!note.inScale) return "opacity-0 pointer-events-none";
    if (note.isRoot) return "bg-cyan-500 text-black border-cyan-300 ring-2 ring-cyan-300";
    return "bg-stone-700 text-stone-100 border-stone-500 hover:bg-stone-600";
  }

  function dotLabel(note) {
    if (!note.inScale) return "";
    if (labelMode === "degree") return note.isRoot ? "R" : note.degree ?? "";
    return note.noteName;
  }

  // ── Jam Station handoff ───────────────────────────────────────────────────
  function loadProgressionIntoJam(prog) {
    if (!prog?.track) return;
    jamHandoffTrack.set(prog.track);
  }

  async function previewChord(chord) {
    if (!chord) return;
    playingChord = chord.name;
    try {
      const midiNotes = chord.notes.map(n => {
        const match = String(n).match(/^([A-G](?:#|b)?)(-?\d+)$/);
        if (!match) return 60;
        const idx = NOTES.indexOf(normalizeNoteName(match[1]));
        const oct = parseInt(match[2]);
        return idx + (oct + 1) * 12;
      });
      await Audio.playJamTrack(
        [{ notes: midiNotes, beats: 4, name: chord.name }],
        60
      );
    } catch (e) {
      console.error("Chord preview failed", e);
    }
    setTimeout(() => { playingChord = null; }, 2000);
  }

  function qualityColor(quality) {
    if (quality === "maj") return "text-cyan-300 border-cyan-700";
    if (quality === "min") return "text-stone-300 border-stone-600";
    if (quality === "dim") return "text-red-400 border-red-900";
    if (quality === "aug") return "text-yellow-300 border-yellow-800";
    return "text-white border-stone-700";
  }
</script>

<div class="h-full min-h-0 flex flex-col gap-4 overflow-hidden">

  <!-- ── Header bar ─────────────────────────────────────────────────────── -->
  <section class="shrink-0 bg-stone-900 rounded-lg p-4 border border-stone-800">
    <div class="flex flex-col 2xl:flex-row 2xl:items-center 2xl:justify-between gap-4">
      <div>
        <div class="text-[10px] font-bold uppercase tracking-[0.2em] text-stone-500">Scale Explorer</div>
        <div class="mt-1 text-2xl font-black font-serif text-white">
          {root} <span class="text-cyan-300">{scaleLabel}</span>
        </div>
      </div>

      <div class="flex flex-wrap items-end gap-3">
        <!-- Key -->
        <label for="se-key" class="flex flex-col gap-1 text-[10px] font-bold text-stone-500 uppercase">
          Key
          <select id="se-key" bind:value={selectedKey}
            class="h-9 bg-stone-800 text-cyan-300 font-bold text-xs rounded px-2 border border-stone-700 outline-none">
            {#each NOTES as note}
              <option value={note}>{note}</option>
            {/each}
          </select>
        </label>

        <!-- Scale -->
        <label for="se-scale" class="flex flex-col gap-1 text-[10px] font-bold text-stone-500 uppercase">
          Scale
          <select id="se-scale" bind:value={selectedScale}
            class="h-9 bg-stone-800 text-orange-300 font-bold text-xs rounded px-2 border border-stone-700 outline-none">
            {#each orderedScales as s}
              <option value={s}>{SCALE_LABELS[s] || s}</option>
            {/each}
          </select>
        </label>

        <!-- View tabs -->
        <div class="flex items-center gap-1 h-9 bg-stone-800 p-1 rounded border border-stone-700">
          <button on:click={() => viewMode = "simple"}
            class="h-7 px-3 rounded text-xs font-bold uppercase transition-all
            {viewMode === 'simple' ? 'bg-stone-700 text-cyan-300' : 'text-stone-500 hover:text-white'}">
            Full Neck
          </button>
          <button on:click={() => viewMode = "positions"}
            class="h-7 px-3 rounded text-xs font-bold uppercase transition-all
            {viewMode === 'positions' ? 'bg-stone-700 text-orange-300' : 'text-stone-500 hover:text-white'}">
            Positions
          </button>
        </div>

        <!-- Label mode -->
        <div class="flex items-center gap-1 h-9 bg-stone-800 p-1 rounded border border-stone-700">
          <button on:click={() => labelMode = "degree"}
            class="h-7 px-3 rounded text-xs font-bold uppercase transition-all
            {labelMode === 'degree' ? 'bg-stone-700 text-green-300' : 'text-stone-500 hover:text-white'}">
            Degrees
          </button>
          <button on:click={() => labelMode = "note"}
            class="h-7 px-3 rounded text-xs font-bold uppercase transition-all
            {labelMode === 'note' ? 'bg-stone-700 text-green-300' : 'text-stone-500 hover:text-white'}">
            Notes
          </button>
        </div>
      </div>
    </div>
  </section>

  <!-- ── Fretboard area ─────────────────────────────────────────────────── -->
  {#if viewMode === "simple"}
    <!-- Full neck view -->
    <section class="shrink-0 bg-stone-900 rounded-lg p-4 border border-stone-800 overflow-x-auto custom-scrollbar">
      <div class="min-w-[52rem]">
        {#each fretboard as string}
          <div class="grid border-b border-stone-800 last:border-b-0"
            style="grid-template-columns: 3.5rem repeat({FRET_COUNT + 1}, minmax(2.6rem, 1fr));">
            <div class="bg-stone-900 border-r-4 border-stone-500 flex items-center justify-center text-[10px] font-bold text-stone-500">
              {string.label}
            </div>
            {#each string.frets as note}
              <div class="h-11 border-r border-stone-800 relative flex items-center justify-center">
                <div class="absolute left-0 right-0 top-1/2 h-px bg-stone-700"></div>
                <!-- Nut -->
                {#if note.fret === 0}
                  <div class="absolute left-0 top-0 bottom-0 w-1 bg-stone-400 rounded-r"></div>
                {/if}
                <div class="relative z-10 w-7 h-7 rounded-full flex items-center justify-center text-[10px] font-black border transition-all
                  {dotClass(note)}">
                  {dotLabel(note)}
                </div>
              </div>
            {/each}
          </div>
        {/each}
        <!-- Fret numbers -->
        <div class="grid mt-1 text-[10px] text-stone-600 font-mono"
          style="grid-template-columns: 3.5rem repeat({FRET_COUNT + 1}, minmax(2.6rem, 1fr));">
          <div></div>
          {#each Array.from({length: FRET_COUNT + 1}, (_, i) => i) as fret}
            <div class="text-center {[3,5,7,9,12,15].includes(fret) ? 'text-stone-400 font-bold' : ''}">{fret}</div>
          {/each}
        </div>
      </div>
    </section>

  {:else}
    <!-- Positions view -->
    <section class="shrink-0 bg-stone-900 rounded-lg border border-stone-800 overflow-hidden">
      <!-- Position selector tabs -->
      <div class="grid grid-cols-5 border-b border-stone-800">
        {#each positions as pos, i}
          <button on:click={() => selectedPos = i}
            class="px-2 py-3 text-center transition-colors hover:bg-stone-800/60
            {selectedPos === i ? 'bg-stone-800 border-b-2 border-orange-400' : ''}">
            <div class="text-[10px] font-bold uppercase tracking-widest
              {selectedPos === i ? 'text-orange-300' : 'text-stone-500'}">
              {pos.label}
            </div>
            <div class="text-[10px] font-mono text-stone-600 mt-0.5">
              Frets {pos.startFret}–{pos.endFret}
            </div>
            <div class="text-[10px] font-bold mt-0.5
              {selectedPos === i ? 'text-white' : 'text-stone-600'}">
              Starts: {pos.startNote} ({pos.startDegreeLabel})
            </div>
          </button>
        {/each}
      </div>

      <!-- Active position fretboard -->
      {#if activePos}
        <div class="p-4 overflow-x-auto custom-scrollbar">
          <div class="text-xs font-bold text-stone-400 mb-3">
            {root} {scaleLabel} — {activePos.label}
            <span class="text-stone-600 ml-2">
              Lowest note on low E: <span class="text-orange-300">{activePos.startNote}</span>
              ({activePos.startDegreeLabel})
            </span>
          </div>
          <div class="min-w-[32rem]">
            {#each activePos.windowNotes as stringNotes, si}
              <div class="grid border-b border-stone-800 last:border-b-0"
                style="grid-template-columns: 3.5rem repeat(5, minmax(3.5rem, 1fr));">
                <div class="bg-stone-900 border-r-4 border-stone-500 flex items-center justify-center text-[10px] font-bold text-stone-500">
                  {STRING_LABELS[si]}
                </div>
                {#each Array.from({length: 5}, (_, fi) => activePos.startFret + fi) as fret}
                  {@const noteHere = stringNotes.find(n => n.fret === fret)}
                  <div class="h-14 border-r border-stone-800 relative flex items-center justify-center">
                    <div class="absolute left-0 right-0 top-1/2 h-px bg-stone-700"></div>
                    {#if fret === 0}
                      <div class="absolute left-0 top-0 bottom-0 w-1 bg-stone-400 rounded-r"></div>
                    {/if}
                    {#if noteHere}
                      <div class="relative z-10 w-8 h-8 rounded-full flex items-center justify-center text-[11px] font-black border transition-all
                        {noteHere.isRoot
                          ? 'bg-cyan-500 text-black border-cyan-300 ring-2 ring-cyan-400 scale-110'
                          : 'bg-stone-700 text-stone-100 border-stone-500'}">
                        {labelMode === 'degree'
                          ? (noteHere.isRoot ? 'R' : noteHere.degree)
                          : noteHere.noteName}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/each}
            <!-- Fret labels -->
            <div class="grid mt-1 text-[10px] text-stone-500 font-mono"
              style="grid-template-columns: 3.5rem repeat(5, minmax(3.5rem, 1fr));">
              <div></div>
              {#each Array.from({length: 5}, (_, fi) => activePos.startFret + fi) as fret}
                <div class="text-center font-bold {[3,5,7,9,12,15].includes(fret) ? 'text-stone-300' : ''}">
                  {fret}
                </div>
              {/each}
            </div>
          </div>

          <!-- Position tip -->
          <div class="mt-3 rounded border border-stone-700 bg-stone-950 px-3 py-2 text-xs text-stone-400">
            <span class="font-bold text-stone-200">Tip:</span>
            Cyan dot = root ({root}). Start and end each phrase on a <span class="text-cyan-300">cyan dot</span> to lock in the key.
            {#if activePos.startDegree !== 1}
              The lowest note here is the <span class="text-orange-300">{activePos.startDegreeLabel}</span> —
              this is the <strong>{activePos.label}</strong> position.
            {/if}
          </div>
        </div>
      {/if}
    </section>
  {/if}

  <!-- ── Diatonic Chords ────────────────────────────────────────────────── -->
  {#if diatonic.length > 0}
    <section class="shrink-0 bg-stone-900 rounded-lg border border-stone-800 p-4">
      <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500 mb-3">
        Diatonic Chords — {root} {scaleLabel}
      </div>
      <div class="flex flex-wrap gap-2">
        {#each diatonic as chord}
          <button
            on:click={() => previewChord(chord)}
            class="min-w-[4.5rem] rounded border px-3 py-2 text-center transition-all hover:bg-stone-800
              {playingChord === chord.name ? 'bg-stone-700 border-cyan-400' : `bg-stone-950 border-stone-700 ${qualityColor(chord.quality)}`}">
            <div class="text-[10px] font-bold text-stone-500">{chord.roman}</div>
            <div class="text-sm font-black">{chord.name}</div>
            <div class="text-[9px] font-mono mt-0.5
              {chord.quality === 'maj' ? 'text-cyan-600' :
               chord.quality === 'min' ? 'text-stone-500' :
               chord.quality === 'dim' ? 'text-red-700' : 'text-yellow-700'}">
              {chord.quality}
            </div>
          </button>
        {/each}
      </div>
      <div class="mt-2 text-[10px] text-stone-600">Tap a chord to preview it.</div>
    </section>
  {/if}

  <!-- ── Try it on — Progressions ──────────────────────────────────────── -->
  {#if progressions.length > 0}
    <section class="shrink-0 bg-stone-900 rounded-lg border border-stone-800 p-4">
      <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500 mb-3">
        Try It On — Practice Progressions
      </div>
      <div class="flex flex-wrap gap-3">
        {#each progressions as prog}
          <div class="rounded border border-stone-700 bg-stone-950 px-4 py-3 min-w-[13rem]">
            <div class="text-xs font-bold text-white mb-2">{prog.label}</div>
            <div class="flex flex-wrap gap-1 mb-3">
              {#each prog.chords as chord}
                <span class="px-2 py-0.5 rounded text-[10px] font-black bg-stone-800 text-stone-200">
                  {chord.name}
                </span>
              {/each}
            </div>
            <button
              on:click={() => loadProgressionIntoJam(prog)}
              class="w-full px-3 py-1.5 rounded bg-cyan-700 hover:bg-cyan-600 text-[10px] font-bold uppercase text-white transition-colors">
              Open in Jam Station →
            </button>
          </div>
        {/each}
      </div>
      <div class="mt-2 text-[10px] text-stone-600">
        Progressions use the diatonic chords of {root} {scaleLabel}.
        Jam Station will start playing immediately so you can solo over it.
      </div>
    </section>
  {/if}

</div>

<style>
  .custom-scrollbar::-webkit-scrollbar { width: 4px; height: 4px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: #292524; border-radius: 8px; }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #44403c; }
</style>
