<script>
  import { SPARK_DATA, getChordNotes } from "../lib/data";
  import { Audio } from "../lib/audio";
  import { jamHandoffTrack } from "../lib/stores";
  import { createEventDispatcher, onMount } from "svelte";

  const dispatch = createEventDispatcher();
  
  let currentSpark = {
    progression: null,
    mood: null,
    constraint: null
  };
  let savedSparks = [];
  let saveStatus = "";

  onMount(async () => {
    try {
      const persisted = await Audio.loadPersistence();
      savedSparks = persisted.saved_sparks || [];
    } catch (e) {
      console.error("Failed to load saved sparks", e);
    }
  });

  function generateSpark() {
    currentSpark = {
      progression: SPARK_DATA.progressions[Math.floor(Math.random() * SPARK_DATA.progressions.length)],
      mood: SPARK_DATA.moods[Math.floor(Math.random() * SPARK_DATA.moods.length)],
      constraint: SPARK_DATA.constraints[Math.floor(Math.random() * SPARK_DATA.constraints.length)]
    };
  }

  function openInJam() {
    if (!currentSpark.progression) return;
    jamHandoffTrack.set(createSparkTrack(currentSpark, `spark-${Date.now()}`, `Spark: ${currentSpark.mood}`));
    dispatch("openJam");
  }

  function createSparkTrack(sparkData, id, title) {
    return {
      id,
      title,
      genre: "Spark",
      bpm: 100,
      key: "C",
      isSpark: true,
      sparkData: JSON.parse(JSON.stringify(sparkData)),
      progression: (sparkData.progression?.chords || []).map(chordName => ({
        name: chordName,
        beats: 4,
        notes: getChordNotes(chordName),
        theory: "Spark Chord"
      }))
    };
  }

  async function saveSpark() {
    if (!currentSpark.progression) return;

    const spark = {
      id: `spark-${Date.now()}`,
      title: `Spark: ${currentSpark.mood}`,
      created_at: new Date().toISOString(),
      spark_data: JSON.parse(JSON.stringify(currentSpark))
    };

    try {
      const persisted = await Audio.saveSpark(spark);
      savedSparks = persisted.saved_sparks || [];
      saveStatus = "SAVED TO MY SHED";
    } catch (e) {
      console.error("Failed to save spark", e);
      saveStatus = "SAVE FAILED";
    }
  }

  async function deleteSpark(id) {
    try {
      const persisted = await Audio.deleteSpark(id);
      savedSparks = persisted.saved_sparks || [];
    } catch (e) {
      console.error("Failed to delete spark", e);
    }
  }

  function loadSavedSpark(spark) {
    currentSpark = JSON.parse(JSON.stringify(spark.spark_data));
    saveStatus = "SPARK LOADED";
  }

  function openSavedSparkInJam(spark) {
    const sparkData = spark.spark_data;
    if (!sparkData?.progression) return;
    jamHandoffTrack.set(createSparkTrack(sparkData, spark.id, spark.title));
    dispatch("openJam");
  }
</script>

<div class="h-full overflow-y-auto custom-scrollbar">
<div class="min-h-full flex flex-col items-center justify-center gap-8 animate-in fade-in duration-500 py-8">
  <div class="text-center flex flex-col gap-2">
    <h2 class="text-xs font-bold text-cyan-400 uppercase tracking-widest">Creative Spark Mode</h2>
    <h1 class="text-5xl font-black font-serif text-white">INSTANT INSPIRATION</h1>
  </div>

  {#if currentSpark.progression}
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6 w-full max-w-5xl">
      <div class="bg-stone-900 border border-stone-800 p-8 rounded-2xl flex flex-col gap-2 shadow-xl hover:border-cyan-500/50 transition-colors">
        <span class="text-[10px] font-bold text-stone-500 uppercase">Progression</span>
        <h3 class="text-3xl font-black text-cyan-400">{currentSpark.progression.pattern}</h3>
        <p class="text-xs text-stone-400 mt-2">{currentSpark.progression.name}</p>
      </div>

      <div class="bg-stone-900 border border-stone-800 p-8 rounded-2xl flex flex-col gap-2 shadow-xl hover:border-shed-orange/50 transition-colors">
        <span class="text-[10px] font-bold text-stone-500 uppercase">Vibe</span>
        <h3 class="text-3xl font-black text-shed-orange uppercase">{currentSpark.mood}</h3>
      </div>

      <div class="bg-stone-900 border border-stone-800 p-8 rounded-2xl flex flex-col gap-2 shadow-xl hover:border-white/20 transition-colors">
        <span class="text-[10px] font-bold text-stone-500 uppercase">Constraint</span>
        <p class="text-lg font-bold text-white leading-tight">{currentSpark.constraint}</p>
      </div>
    </div>
  {:else}
    <div class="w-full max-w-xl aspect-video bg-stone-900/50 border-2 border-dashed border-stone-800 rounded-3xl flex flex-col items-center justify-center text-stone-600 gap-4">
      <div class="text-6xl">?</div>
      <p class="font-bold uppercase tracking-widest text-sm text-stone-500">Nothing Generated Yet</p>
    </div>
  {/if}

  <div class="flex gap-4">
    <button 
      on:click={generateSpark}
      class="bg-cyan-600 hover:bg-cyan-500 text-stone-950 px-10 py-4 rounded-full font-black uppercase tracking-widest shadow-lg shadow-cyan-900/20 transition-all active:scale-95">
      {currentSpark.progression ? "GENERATE NEW SPARK" : "IGNITE MY INSPIRATION"}
    </button>
    
    {#if currentSpark.progression}
      <button 
        on:click={openInJam}
        class="bg-stone-800 hover:bg-stone-700 text-white px-10 py-4 rounded-full font-black uppercase tracking-widest border border-stone-700 transition-all active:scale-95">
        LOAD TO JAM MIXER
      </button>
      <button 
        on:click={saveSpark}
        class="bg-stone-900 hover:bg-stone-800 text-cyan-300 px-10 py-4 rounded-full font-black uppercase tracking-widest border border-cyan-900 transition-all active:scale-95">
        SAVE SPARK
      </button>
    {/if}
  </div>

  {#if saveStatus}
    <p class="text-xs font-bold uppercase tracking-widest text-stone-500">{saveStatus}</p>
  {/if}

  {#if savedSparks.length > 0}
    <section class="w-full max-w-5xl mt-4">
      <h2 class="text-xs font-bold text-stone-500 uppercase tracking-widest mb-3">Saved Sparks</h2>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        {#each savedSparks as spark}
          <article class="bg-stone-900 border border-stone-800 rounded-lg p-4">
            <div class="flex justify-between gap-3">
              <div>
                <h3 class="font-bold text-white">{spark.title}</h3>
                <p class="text-[10px] text-cyan-400 font-mono mt-1">{spark.spark_data?.progression?.pattern}</p>
              </div>
              <button on:click={() => deleteSpark(spark.id)} class="text-stone-500 hover:text-red-400 text-xs font-bold">DELETE</button>
            </div>
            <p class="text-xs text-stone-400 mt-3">{spark.spark_data?.constraint}</p>
            <div class="flex gap-2 mt-4">
              <button on:click={() => loadSavedSpark(spark)} class="flex-1 bg-stone-800 hover:bg-stone-700 text-white rounded px-3 py-2 text-[10px] font-bold uppercase">
                Load
              </button>
              <button on:click={() => openSavedSparkInJam(spark)} class="flex-1 bg-cyan-700 hover:bg-cyan-600 text-black rounded px-3 py-2 text-[10px] font-bold uppercase">
                Jam
              </button>
            </div>
          </article>
        {/each}
      </div>
    </section>
  {/if}
</div>
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
    border-radius: 10px;
  }
</style>
