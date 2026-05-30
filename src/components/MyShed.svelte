<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { Audio } from "../lib/audio";
  import { getChordNotes } from "../lib/data";
  import { jamHandoffTrack } from "../lib/stores";

  const dispatch = createEventDispatcher();

  let savedLessons = [];
  let savedSparks = [];
  let practiceSessions = [];
  let activeSession = null;
  let status = "Loading your shed...";
  let selectedLesson = null;
  let timerSeconds = 0;
  let timerActive = false;
  let timerHandle = null;

  $: lessonCount = savedLessons.length;
  $: sparkCount = savedSparks.length;
  $: completedCount = practiceSessions.filter(session => session.completed_at).length;
  $: timerDisplay = `${Math.floor(timerSeconds / 60)}:${String(timerSeconds % 60).padStart(2, "0")}`;

  function normalizeLesson(lesson) {
    return {
      ...lesson,
      artistName: lesson.artistName || lesson.musician_name || "Unknown Artist",
      volumeTitle: lesson.volumeTitle || lesson.volume_id || "Saved Lesson",
      duration: lesson.duration || 10,
      theory: lesson.theory || "",
      drill: lesson.drill || ""
    };
  }

  async function loadShed() {
    try {
      const persisted = await Audio.loadPersistence();
      savedLessons = (persisted.saved_lessons || []).map(normalizeLesson);
      savedSparks = persisted.saved_sparks || [];
      practiceSessions = persisted.practice_state?.sessions || [];
      activeSession = persisted.practice_state?.active_session || null;
      status = savedLessons.length || savedSparks.length ? "Ready" : "Nothing saved yet";
    } catch (e) {
      console.error("Failed to load My Shed", e);
      status = "Could not load saved items";
    }
  }

  async function removeLesson(id) {
    try {
      const persisted = await Audio.deleteLesson(id);
      savedLessons = (persisted.saved_lessons || []).map(normalizeLesson);
      practiceSessions = persisted.practice_state?.sessions || [];
      activeSession = persisted.practice_state?.active_session || null;
      if (selectedLesson?.id === id) {
        closePractice();
      }
    } catch (e) {
      console.error("Failed to remove lesson", e);
      status = "Remove failed";
    }
  }

  async function removeSpark(id) {
    try {
      const persisted = await Audio.deleteSpark(id);
      savedSparks = persisted.saved_sparks || [];
    } catch (e) {
      console.error("Failed to remove spark", e);
      status = "Remove failed";
    }
  }

  async function startPractice(lesson) {
    selectedLesson = normalizeLesson(lesson);
    timerSeconds = selectedLesson.duration * 60;
    stopTimer();

    try {
      const persisted = await Audio.startPracticeSession(selectedLesson, new Date().toISOString());
      savedLessons = (persisted.saved_lessons || []).map(normalizeLesson);
      activeSession = persisted.practice_state?.active_session || null;
      practiceSessions = persisted.practice_state?.sessions || [];
      status = "Practice session started";
    } catch (e) {
      console.error("Failed to start practice session", e);
      status = "Could not start practice session";
    }
  }

  async function finishPractice() {
    if (!selectedLesson) return;
    stopTimer();

    try {
      const persisted = await Audio.finishPracticeSession(selectedLesson.id, new Date().toISOString());
      practiceSessions = persisted.practice_state?.sessions || [];
      activeSession = persisted.practice_state?.active_session || null;
      status = "Practice session completed";
    } catch (e) {
      console.error("Failed to finish practice session", e);
      status = "Could not finish practice session";
    }
  }

  function toggleTimer() {
    if (!selectedLesson) return;

    if (timerActive) {
      stopTimer();
      return;
    }

    if (timerSeconds <= 0) {
      timerSeconds = selectedLesson.duration * 60;
    }

    timerActive = true;
    timerHandle = window.setInterval(() => {
      if (timerSeconds <= 1) {
        timerSeconds = 0;
        finishPractice();
      } else {
        timerSeconds -= 1;
      }
    }, 1000);
  }

  function stopTimer() {
    timerActive = false;
    if (timerHandle) {
      window.clearInterval(timerHandle);
      timerHandle = null;
    }
  }

  function resetTimer() {
    if (!selectedLesson) return;
    stopTimer();
    timerSeconds = selectedLesson.duration * 60;
  }

  function closePractice() {
    stopTimer();
    selectedLesson = null;
    timerSeconds = 0;
  }

  function loadSparkToJam(spark) {
    const sparkData = spark.spark_data || spark.sparkData;
    if (!sparkData?.progression) return;

    const chords = sparkData.progression.chords || [];
    jamHandoffTrack.set({
      id: spark.id,
      title: spark.title,
      genre: "Spark",
      bpm: 100,
      key: "C",
      isSpark: true,
      sparkData,
      progression: chords.map(chordName => ({
        name: chordName,
        beats: 4,
        notes: getChordNotes(chordName),
        theory: "Saved Spark"
      }))
    });
    dispatch("openJam");
  }

  onMount(loadShed);
</script>

<div class="h-full grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_22rem] gap-6 overflow-hidden">
  <section class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg overflow-hidden flex flex-col">
    <header class="p-6 border-b border-stone-800 flex flex-col gap-4">
      <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-4">
        <div>
          <span class="text-xs font-bold text-shed-orange uppercase tracking-widest">My Shed</span>
          <h2 class="text-4xl font-black font-serif text-white mt-2">SAVED PRACTICE</h2>
          <p class="text-stone-400 mt-2 max-w-2xl">
            Saved drills, sparks, and practice history from native app storage.
          </p>
        </div>
        <button
          on:click={loadShed}
          class="bg-stone-800 hover:bg-stone-700 text-stone-300 px-4 py-2 rounded border border-stone-700 font-bold text-xs uppercase tracking-widest">
          Refresh
        </button>
      </div>

      <div class="grid grid-cols-3 gap-3">
        <div class="bg-stone-950 border border-stone-800 rounded p-4">
          <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Lessons</div>
          <div class="text-2xl font-black text-white mt-1">{lessonCount}</div>
        </div>
        <div class="bg-stone-950 border border-stone-800 rounded p-4">
          <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Sparks</div>
          <div class="text-2xl font-black text-cyan-400 mt-1">{sparkCount}</div>
        </div>
        <div class="bg-stone-950 border border-stone-800 rounded p-4">
          <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Finished</div>
          <div class="text-2xl font-black text-shed-orange mt-1">{completedCount}</div>
        </div>
      </div>
    </header>

    <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar p-6">
      {#if selectedLesson}
        <article class="bg-stone-950 border border-stone-800 rounded-lg p-6 mb-6">
          <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-4">
            <div>
              <button on:click={closePractice} class="text-stone-500 hover:text-white text-xs font-bold uppercase mb-4">
                Close Focus Session
              </button>
              <div class="text-[10px] font-bold uppercase tracking-widest text-shed-orange">Now Practicing</div>
              <h3 class="text-4xl font-black font-serif text-white mt-2">{selectedLesson.title}</h3>
              <p class="text-stone-500 text-sm font-bold uppercase mt-2">{selectedLesson.artistName} / {selectedLesson.duration}m</p>
            </div>
            <div class="text-right">
              <div class="text-5xl font-mono font-black text-white">{timerDisplay}</div>
              <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Remaining</div>
            </div>
          </div>

          <div class="grid md:grid-cols-2 gap-4 my-6">
            <div class="bg-stone-900 border border-stone-800 rounded p-4">
              <div class="text-[10px] font-bold uppercase tracking-widest text-shed-orange">Focus</div>
              <p class="text-stone-300 italic mt-2">"{selectedLesson.theory}"</p>
            </div>
            <div class="bg-stone-900 border border-stone-800 rounded p-4">
              <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Assignment</div>
              <p class="text-white mt-2">{selectedLesson.drill}</p>
            </div>
          </div>

          <div class="flex flex-wrap gap-3">
            <button on:click={toggleTimer} class="bg-white hover:bg-stone-200 text-black px-6 py-3 rounded font-black uppercase tracking-widest">
              {timerActive ? "Pause" : "Start"}
            </button>
            <button on:click={resetTimer} class="bg-stone-800 hover:bg-stone-700 text-white px-6 py-3 rounded border border-stone-700 font-black uppercase tracking-widest">
              Reset
            </button>
            <button on:click={finishPractice} class="bg-shed-orange hover:bg-orange-500 text-black px-6 py-3 rounded font-black uppercase tracking-widest">
              Finish
            </button>
          </div>
        </article>
      {/if}

      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <section>
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-xs font-bold text-shed-orange uppercase tracking-widest">Saved Lessons</h3>
            <span class="text-[10px] text-stone-500 font-mono">{lessonCount} total</span>
          </div>

          {#if savedLessons.length === 0}
            <div class="border-2 border-dashed border-stone-800 rounded-lg p-8 text-center text-stone-500">
              Save drills from Magazine to build a practice list.
            </div>
          {:else}
            <div class="grid gap-3">
              {#each savedLessons as lesson}
                <article class="bg-stone-950 border border-stone-800 rounded-lg p-5">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <h4 class="font-bold text-white font-serif text-lg">{lesson.title}</h4>
                      <p class="text-[10px] text-stone-500 uppercase font-bold mt-1">{lesson.artistName} / {lesson.duration}m</p>
                    </div>
                    <button on:click={() => removeLesson(lesson.id)} class="text-stone-500 hover:text-red-400 text-xs font-bold">
                      DELETE
                    </button>
                  </div>
                  <p class="text-sm text-stone-400 mt-3 line-clamp-2">{lesson.drill}</p>
                  <button on:click={() => startPractice(lesson)} class="mt-4 bg-stone-800 hover:bg-stone-700 text-white px-4 py-2 rounded border border-stone-700 text-xs font-bold uppercase tracking-widest">
                    Start Practice
                  </button>
                </article>
              {/each}
            </div>
          {/if}
        </section>

        <section>
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-xs font-bold text-cyan-400 uppercase tracking-widest">Saved Sparks</h3>
            <span class="text-[10px] text-stone-500 font-mono">{sparkCount} total</span>
          </div>

          {#if savedSparks.length === 0}
            <div class="border-2 border-dashed border-stone-800 rounded-lg p-8 text-center text-stone-500">
              Save sparks from the Spark Generator to collect song starters.
            </div>
          {:else}
            <div class="grid gap-3">
              {#each savedSparks as spark}
                {@const sparkData = spark.spark_data || spark.sparkData}
                <article class="bg-stone-950 border border-stone-800 rounded-lg p-5">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <h4 class="font-bold text-white font-serif text-lg">{spark.title}</h4>
                      <p class="text-[10px] text-cyan-400 font-mono mt-1">{sparkData?.progression?.pattern || "No progression"}</p>
                    </div>
                    <button on:click={() => removeSpark(spark.id)} class="text-stone-500 hover:text-red-400 text-xs font-bold">
                      DELETE
                    </button>
                  </div>
                  <p class="text-sm text-stone-400 mt-3">{sparkData?.constraint || "No constraint saved"}</p>
                  <button on:click={() => loadSparkToJam(spark)} class="mt-4 bg-cyan-600 hover:bg-cyan-500 text-black px-4 py-2 rounded text-xs font-black uppercase tracking-widest">
                    Load to Jam
                  </button>
                </article>
              {/each}
            </div>
          {/if}
        </section>
      </div>
    </div>
  </section>

  <aside class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg p-6 overflow-y-auto custom-scrollbar">
    <h3 class="text-xs font-bold text-shed-orange uppercase tracking-widest">Practice Log</h3>
    <p class="text-sm text-stone-400 mt-2 mb-5">{status}</p>

    {#if activeSession}
      <div class="bg-stone-950 border border-shed-orange/40 rounded p-4 mb-5">
        <div class="text-[10px] font-bold uppercase tracking-widest text-shed-orange">Active Session</div>
        <div class="font-bold text-white mt-2">{activeSession.title}</div>
        <div class="text-xs text-stone-500 mt-1">{activeSession.duration} minutes</div>
      </div>
    {/if}

    {#if practiceSessions.length === 0}
      <div class="text-sm text-stone-500 border border-stone-800 rounded p-4">
        Completed sessions will appear here.
      </div>
    {:else}
      <div class="grid gap-3">
        {#each practiceSessions.slice(0, 12) as session}
          <div class="bg-stone-950 border border-stone-800 rounded p-4">
            <div class="font-bold text-white">{session.title}</div>
            <div class="text-[10px] text-stone-500 uppercase tracking-widest mt-2">{session.duration}m / {session.completed_at ? "Complete" : "Open"}</div>
          </div>
        {/each}
      </div>
    {/if}
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
