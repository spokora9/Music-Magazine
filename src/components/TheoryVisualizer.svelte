<script>
  import { NOTES, getScaleNotes, normalizeNoteName } from "../lib/data";

  export let rootKey = "C";
  export let scaleType = "major";
  export let viewMode = "guitar";
  export let currentChordNotes = [];
  export let currentChordLabel = "";
  export let showChordTones = true;
  export let showCommonTones = true;
  export let contextLabel = "Theory Visualizer";
  export let showHeader = true;

  const GUITAR_STRINGS = ["E", "B", "G", "D", "A", "E"];
  const FRETS = Array.from({ length: 13 }, (_, index) => index);
  const PIANO_NOTES = [...NOTES, ...NOTES.slice(0, 5)];

  $: normalizedRoot = normalizeNoteName(rootKey) || "C";
  $: scaleNotes = getScaleNotes(normalizedRoot, scaleType);
  $: chordNotes = normalizeNotes(currentChordNotes);
  $: commonNotes = scaleNotes.filter(note => chordNotes.includes(note));
  $: activeNotes = showChordTones ? [...new Set([...scaleNotes, ...chordNotes])] : scaleNotes;
  $: scaleLabel = String(scaleType || "major").replace(/_/g, " ");

  function normalizeNotes(notes) {
    return (notes || [])
      .map(note => {
        if (typeof note === "number") return NOTES[((note % 12) + 12) % 12];
        return normalizeNoteName(String(note).replace(/\d/g, ""));
      })
      .filter(note => NOTES.includes(note));
  }

  function noteAt(startNote, fret) {
    const startIndex = NOTES.indexOf(startNote);
    return NOTES[(startIndex + fret) % NOTES.length];
  }

  function noteState(note) {
    const normalized = normalizeNoteName(note);
    const inScale = scaleNotes.includes(normalized);
    const isRoot = normalized === normalizedRoot;
    const isChordTone = chordNotes.includes(normalized);
    const isCommon = commonNotes.includes(normalized);
    const visible = activeNotes.includes(normalized);

    return { inScale, isRoot, isChordTone, isCommon, visible };
  }

  function fretClass(note) {
    const state = noteState(note);
    if (!state.visible) return "opacity-0";
    if (showChordTones && state.isChordTone) return "bg-orange-500 text-black border-orange-300";
    if (state.isRoot) return "bg-cyan-500 text-black border-cyan-300";
    if (state.inScale) return "bg-stone-700 text-stone-100 border-stone-500";
    return "bg-stone-800 text-stone-400 border-stone-700";
  }

  function commonRingClass(note) {
    const state = noteState(note);
    return showCommonTones && state.isCommon && !state.isRoot ? "ring-2 ring-green-300" : "";
  }

  function rootRingClass(note) {
    return noteState(note).isRoot ? "ring-2 ring-cyan-300" : "";
  }

  function chordAccentClass(note) {
    return showChordTones && noteState(note).isChordTone
      ? "scale-110 shadow-[0_0_18px_rgba(249,115,22,0.85)]"
      : "";
  }

  function pianoClass(note, black) {
    const state = noteState(note);
    if (showChordTones && state.isChordTone) return black ? "bg-orange-700 border-orange-400" : "bg-orange-200 border-orange-500";
    if (state.isRoot) return black ? "bg-cyan-700 border-cyan-400" : "bg-cyan-200 border-cyan-500";
    if (state.inScale) return black ? "bg-cyan-950 border-cyan-800" : "bg-stone-100 border-stone-300";
    return black ? "bg-stone-800 border-stone-900 opacity-40" : "bg-stone-300 border-stone-400 opacity-35";
  }
</script>

<div class="h-full min-h-0 flex flex-col gap-4">
  {#if showHeader}
    <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
      <div>
        <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">{contextLabel}</div>
        <div class="mt-1 flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <span class="text-2xl font-black font-serif text-white">{currentChordLabel || "No chord"}</span>
          <span class="text-sm font-bold uppercase text-cyan-400">{normalizedRoot} {scaleLabel}</span>
        </div>
        {#if showChordTones && chordNotes.length}
          <div class="mt-2 flex flex-wrap gap-1.5">
            {#each chordNotes as note}
              <span class="px-2 py-1 rounded bg-orange-500 text-black text-[10px] font-black">{note}</span>
            {/each}
          </div>
        {/if}
      </div>

      <div class="flex flex-wrap gap-2 text-[10px] font-bold uppercase tracking-widest">
        <span class="px-2 py-1 rounded border border-cyan-500/60 text-cyan-300">Root</span>
        <span class="px-2 py-1 rounded border border-stone-600 text-stone-300">Scale</span>
        {#if showChordTones}
          <span class="px-2 py-1 rounded border border-orange-400/70 text-orange-300">Chord</span>
        {/if}
        {#if showCommonTones}
          <span class="px-2 py-1 rounded border border-green-400/70 text-green-300">Common</span>
        {/if}
      </div>
    </div>
  {/if}

  <div class="flex-1 min-h-0 overflow-auto custom-scrollbar">
    {#if viewMode === "guitar"}
      <div class="min-w-[48rem] select-none">
        {#each GUITAR_STRINGS as stringNote, stringIndex}
          <div class="grid border-b border-stone-800 last:border-b-0" style="grid-template-columns: 3rem repeat(13, minmax(2.75rem, 1fr));">
            <div class="bg-stone-900 border-r-4 border-stone-500 flex items-center justify-center text-xs font-bold text-stone-500">
              {stringIndex === GUITAR_STRINGS.length - 1 ? "E2" : stringNote}
            </div>
            {#each FRETS as fret}
              {@const fretNote = noteAt(stringNote, fret)}
              <div class="h-12 border-r border-stone-800 relative flex items-center justify-center">
                <div class="absolute left-0 right-0 top-1/2 h-px bg-stone-700"></div>
                <div class="relative z-10 w-7 h-7 rounded-full flex items-center justify-center text-[10px] font-black border transition-all duration-300 {fretClass(fretNote)} {commonRingClass(fretNote)} {rootRingClass(fretNote)} {chordAccentClass(fretNote)}">
                  {noteState(fretNote).visible ? fretNote : ""}
                </div>
              </div>
            {/each}
          </div>
        {/each}
        <div class="grid mt-2 text-[10px] text-stone-600 font-mono" style="grid-template-columns: 3rem repeat(13, minmax(2.75rem, 1fr));">
          <div></div>
          {#each FRETS as fret}
            <div class="text-center">{fret}</div>
          {/each}
        </div>
      </div>
    {:else}
      <div class="min-w-[44rem] h-64 flex justify-center items-start pt-4 select-none">
        {#each PIANO_NOTES as pianoNote, index}
          {#if !pianoNote.includes("#")}
            {@const nextNote = PIANO_NOTES[index + 1]}
            <div class="relative shrink-0">
              <div class="w-14 h-52 border rounded-b-lg flex items-end justify-center pb-3 transition-all duration-300 {pianoClass(pianoNote, false)} {commonRingClass(pianoNote)} {rootRingClass(pianoNote)} {chordAccentClass(pianoNote)}">
                <span class="text-xs font-black text-stone-700">{noteState(pianoNote).visible ? pianoNote : ""}</span>
              </div>
              {#if nextNote?.includes("#")}
                <div class="absolute top-0 -right-4 w-8 h-32 z-10 border rounded-b flex items-end justify-center pb-2 transition-all duration-300 {pianoClass(nextNote, true)} {commonRingClass(nextNote)} {rootRingClass(nextNote)} {chordAccentClass(nextNote)}">
                  <span class="text-[10px] font-black text-white">{noteState(nextNote).visible ? nextNote : ""}</span>
                </div>
              {/if}
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
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
