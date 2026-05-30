<script>
  import { onDestroy, onMount } from "svelte";
  import {
    MUSICIAN_CHALLENGE_CARDS,
    PRODUCER_VOLUMES,
    PRODUCER_CHALLENGE_CARDS,
    GUITAR_VOLUMES
  } from "../lib/learningData";
  import { Audio } from "../lib/audio";

  let currentMode = "producers"; // "producers" or "musicians"
  let activeLearningView = "library"; // "library", "practice", "myshed", "challenge"
  let selectedVolume = PRODUCER_VOLUMES[0];
  let selectedMusician = selectedVolume.musicians[0];
  let savedLessons = [];
  let activeLesson = null;
  let routineIndex = 0;
  let isRoutineActive = false;
  let timerSeconds = 0;
  let timerActive = false;
  let timerHandle = null;
  let challengeCard = null;

  const CHALLENGE_CARDS = {
    musicians: MUSICIAN_CHALLENGE_CARDS,
    producers: PRODUCER_CHALLENGE_CARDS
  };

  // Theme color mapping: theme name -> { accent: 'text-XXX', border: 'border-XXX', hoverBorder: 'hover:border-XXX' }
  const THEME_COLORS = {
    purple: { accent: 'text-purple-400', border: 'border-purple-500', hoverBorder: 'hover:border-purple-400', accentBg: 'bg-purple-500' },
    amber: { accent: 'text-amber-400', border: 'border-amber-500', hoverBorder: 'hover:border-amber-400', accentBg: 'bg-amber-500' },
    orange: { accent: 'text-shed-orange', border: 'border-shed-orange', hoverBorder: 'hover:border-orange-400', accentBg: 'bg-orange-500' },
    cyan: { accent: 'text-cyan-400', border: 'border-cyan-500', hoverBorder: 'hover:border-cyan-400', accentBg: 'bg-cyan-500' },
    emerald: { accent: 'text-emerald-400', border: 'border-emerald-500', hoverBorder: 'hover:border-emerald-400', accentBg: 'bg-emerald-500' },
    green: { accent: 'text-green-400', border: 'border-green-500', hoverBorder: 'hover:border-green-400', accentBg: 'bg-green-500' },
    lime: { accent: 'text-lime-400', border: 'border-lime-500', hoverBorder: 'hover:border-lime-400', accentBg: 'bg-lime-500' },
    pink: { accent: 'text-pink-400', border: 'border-pink-500', hoverBorder: 'hover:border-pink-400', accentBg: 'bg-pink-500' },
    red: { accent: 'text-red-400', border: 'border-red-500', hoverBorder: 'hover:border-red-400', accentBg: 'bg-red-500' },
    rose: { accent: 'text-rose-400', border: 'border-rose-500', hoverBorder: 'hover:border-rose-400', accentBg: 'bg-rose-500' },
    teal: { accent: 'text-teal-400', border: 'border-teal-500', hoverBorder: 'hover:border-teal-400', accentBg: 'bg-teal-500' },
    yellow: { accent: 'text-yellow-400', border: 'border-yellow-500', hoverBorder: 'hover:border-yellow-400', accentBg: 'bg-yellow-500' },
    blue: { accent: 'text-blue-400', border: 'border-blue-500', hoverBorder: 'hover:border-blue-400', accentBg: 'bg-blue-500' },
    indigo: { accent: 'text-indigo-400', border: 'border-indigo-500', hoverBorder: 'hover:border-indigo-400', accentBg: 'bg-indigo-500' },
    sky: { accent: 'text-sky-400', border: 'border-sky-500', hoverBorder: 'hover:border-sky-400', accentBg: 'bg-sky-500' },
    slate: { accent: 'text-slate-300', border: 'border-slate-500', hoverBorder: 'hover:border-slate-400', accentBg: 'bg-slate-500' },
    zinc: { accent: 'text-zinc-300', border: 'border-zinc-500', hoverBorder: 'hover:border-zinc-400', accentBg: 'bg-zinc-500' },
    stone: { accent: 'text-stone-300', border: 'border-stone-500', hoverBorder: 'hover:border-stone-400', accentBg: 'bg-stone-500' }
  };
  const DEFAULT_THEME = THEME_COLORS.orange;

  function getThemeColors(theme) {
    return THEME_COLORS[theme] || DEFAULT_THEME;
  }

  function switchMode(mode) {
    currentMode = mode;
    activeLearningView = "library";
    if (mode === "producers") {
      selectedVolume = PRODUCER_VOLUMES[0];
    } else {
      selectedVolume = GUITAR_VOLUMES[0];
    }
    selectedMusician = selectedVolume.musicians[0];
    challengeCard = null;
  }

  function makeLesson(lesson, musician = selectedMusician, volume = selectedVolume) {
    if (lesson.musician_id || lesson.artistId) {
      return normalizeLesson(lesson);
    }

    const lessonId = `${currentMode}:${volume.id}:${musician.id}:${lesson.id}`;
    return {
      ...lesson,
      id: lessonId,
      sourceId: lesson.id,
      artistId: musician.id,
      artistName: musician.name,
      archetype: musician.archetype,
      volumeId: volume.id,
      volumeTitle: volume.title,
      mode: currentMode,
      musician_id: musician.id,
      musician_name: musician.name,
      volume_id: volume.id,
      duration: lesson.duration || 10,
      theory: lesson.theory || "",
      drill: lesson.drill || ""
    };
  }

  function normalizeLesson(lesson) {
    return {
      ...lesson,
      artistId: lesson.artistId || lesson.musician_id,
      artistName: lesson.artistName || lesson.musician_name,
      archetype: lesson.archetype || "",
      volumeId: lesson.volumeId || lesson.volume_id,
      volumeTitle: lesson.volumeTitle || lesson.volume_id,
      musician_id: lesson.musician_id || lesson.artistId,
      musician_name: lesson.musician_name || lesson.artistName,
      volume_id: lesson.volume_id || lesson.volumeId,
      duration: lesson.duration || 10,
      theory: lesson.theory || "",
      drill: lesson.drill || ""
    };
  }

  function isSaved(lessonId) {
    return savedLessons.some((lesson) => lesson.id === lessonId);
  }

  async function toggleSaveLesson(lesson, musician = selectedMusician, volume = selectedVolume) {
    const packed = makeLesson(lesson, musician, volume);
    try {
      const persisted = isSaved(packed.id)
        ? await Audio.deleteLesson(packed.id)
        : await Audio.saveLesson(packed);
      savedLessons = (persisted.saved_lessons || []).map(normalizeLesson);
      if (persisted.practice_state?.active_session?.lesson_id === packed.id) {
        activeLesson = packed;
      }
    } catch (e) {
      console.error("Failed to persist saved lesson", e);
    }
  }

  async function startPractice(lesson, musician = selectedMusician, volume = selectedVolume, routine = false, index = 0) {
    const packed = makeLesson(lesson, musician, volume);
    activeLesson = packed;
    activeLearningView = "practice";
    isRoutineActive = routine;
    routineIndex = index;
    timerSeconds = (packed.duration || 10) * 60;
    stopTimer();
    try {
      const persisted = await Audio.startPracticeSession(packed, new Date().toISOString());
      savedLessons = (persisted.saved_lessons || []).map(normalizeLesson);
    } catch (e) {
      console.error("Failed to persist practice session", e);
    }
  }

  async function startRoutine() {
    if (!savedLessons.length) return;
    activeLesson = savedLessons[0];
    activeLearningView = "practice";
    isRoutineActive = true;
    routineIndex = 0;
    timerSeconds = (activeLesson.duration || 10) * 60;
    stopTimer();
    try {
      await Audio.startPracticeSession(activeLesson, new Date().toISOString());
    } catch (e) {
      console.error("Failed to persist routine session", e);
    }
  }

  async function nextRoutineStep() {
    const nextIndex = routineIndex + 1;
    if (!isRoutineActive || nextIndex >= savedLessons.length) {
      if (activeLesson) {
        try {
          await Audio.finishPracticeSession(activeLesson.id, new Date().toISOString());
        } catch (e) {
          console.error("Failed to finish practice session", e);
        }
      }
      activeLearningView = "myshed";
      isRoutineActive = false;
      activeLesson = null;
      stopTimer();
      return;
    }
    routineIndex = nextIndex;
    activeLesson = savedLessons[nextIndex];
    timerSeconds = (activeLesson.duration || 10) * 60;
    stopTimer();
    try {
      await Audio.startPracticeSession(activeLesson, new Date().toISOString());
    } catch (e) {
      console.error("Failed to persist routine step", e);
    }
  }

  function stopTimer() {
    timerActive = false;
    if (timerHandle) clearInterval(timerHandle);
    timerHandle = null;
  }

  function toggleTimer() {
    if (!activeLesson) return;
    if (timerActive) {
      stopTimer();
      return;
    }
    timerActive = true;
    timerHandle = setInterval(() => {
      timerSeconds = Math.max(0, timerSeconds - 1);
      if (timerSeconds === 0) {
        stopTimer();
        if (activeLesson) {
          Audio.finishPracticeSession(activeLesson.id, new Date().toISOString())
            .catch(e => console.error("Failed to finish practice session", e));
        }
      }
    }, 1000);
  }

  function resetTimer() {
    if (!activeLesson) return;
    timerSeconds = (activeLesson.duration || 10) * 60;
    stopTimer();
  }

  function formatTime(seconds) {
    const minutes = Math.floor(seconds / 60);
    const remaining = seconds % 60;
    return `${minutes}:${remaining < 10 ? "0" : ""}${remaining}`;
  }

  function pickChallenge() {
    const options = CHALLENGE_CARDS[currentMode] || [];
    challengeCard = options.length
      ? options[Math.floor(Math.random() * options.length)]
      : null;
  }

  onMount(async () => {
    try {
      const persisted = await Audio.loadPersistence();
      savedLessons = (persisted.saved_lessons || []).map(normalizeLesson);
      const activeSession = persisted.practice_state?.active_session;
      if (activeSession) {
        activeLesson = savedLessons.find((lesson) => lesson.id === activeSession.lesson_id) || null;
      }
    } catch (e) {
      console.error("Failed to load lesson persistence", e);
      savedLessons = [];
    }
    pickChallenge();
  });

  onDestroy(() => stopTimer());

  $: currentVolumes = currentMode === "producers" ? PRODUCER_VOLUMES : GUITAR_VOLUMES;
  $: theme = getThemeColors(selectedMusician?.theme);
  $: nextRoutineLesson = isRoutineActive ? savedLessons[routineIndex + 1] : null;
</script>

<div class="h-full flex flex-col gap-6 overflow-y-auto pr-2 custom-scrollbar">
  <header class="flex flex-col gap-4">
    <h2 class="text-xs font-bold text-shed-orange uppercase tracking-widest">The Magazine</h2>
    
    <!-- Mode Selector -->
    <div class="flex gap-2">
      <button
        on:click={() => switchMode("musicians")}
        class="px-4 py-2 rounded-lg font-bold text-sm transition-all {currentMode === 'musicians' ? 'bg-shed-orange text-black' : 'bg-stone-800 text-stone-400 hover:text-white'}">
        [MUSICIANS]
      </button>
      <button
        on:click={() => switchMode("producers")}
        class="px-4 py-2 rounded-lg font-bold text-sm transition-all {currentMode === 'producers' ? 'bg-shed-orange text-black' : 'bg-stone-800 text-stone-400 hover:text-white'}">
        [PRODUCERS]
      </button>
    </div>

    <div class="flex flex-col lg:flex-row lg:items-end justify-between gap-4">
      <h1 class="text-4xl font-black font-serif">{currentMode === "producers" ? "PRODUCER" : "MUSICIAN"} VOLUMES</h1>
      <div class="flex gap-2">
        <button on:click={() => activeLearningView = "library"} class="px-3 py-2 rounded-lg font-bold text-xs uppercase transition-all {activeLearningView === 'library' ? 'bg-white text-black' : 'bg-stone-800 text-stone-400 hover:text-white'}">Library</button>
        <button on:click={() => activeLearningView = "myshed"} class="px-3 py-2 rounded-lg font-bold text-xs uppercase transition-all {activeLearningView === 'myshed' ? 'bg-white text-black' : 'bg-stone-800 text-stone-400 hover:text-white'}">My Shed ({savedLessons.length})</button>
        <button on:click={() => { activeLearningView = "challenge"; if (!challengeCard) pickChallenge(); }} class="px-3 py-2 rounded-lg font-bold text-xs uppercase transition-all {activeLearningView === 'challenge' ? 'bg-white text-black' : 'bg-stone-800 text-stone-400 hover:text-white'}">Daily Drill</button>
      </div>
    </div>
  </header>

  {#if activeLearningView === "practice" && activeLesson}
    <section class="bg-stone-900 border border-stone-800 rounded-2xl p-8 shadow-2xl min-h-[560px] flex flex-col">
      <div class="flex justify-between items-start gap-4">
        <div>
          <button on:click={() => activeLearningView = isRoutineActive ? "myshed" : "library"} class="text-stone-500 hover:text-white text-sm font-bold uppercase mb-4">
            Exit Focus Mode
          </button>
          <div class="text-shed-orange text-xs font-bold uppercase tracking-[0.2em] mb-2">
            {isRoutineActive ? `Routine: Step ${routineIndex + 1} of ${savedLessons.length}` : "Current Session"}
          </div>
          <h2 class="text-5xl font-black font-serif">{activeLesson.title}</h2>
          <p class="text-stone-500 text-sm font-bold uppercase mt-3">{activeLesson.artistName} / {activeLesson.volumeTitle}</p>
          <p class="text-xl italic text-stone-400 font-serif mt-4">"{activeLesson.theory}"</p>
        </div>
        {#if isRoutineActive}
          <button on:click={nextRoutineStep} class="bg-stone-800 hover:bg-stone-700 text-stone-300 px-4 py-2 rounded-lg text-xs font-bold uppercase tracking-wider border border-stone-700">
            Skip Step
          </button>
        {/if}
      </div>

      <div class="flex-1 flex flex-col items-center justify-center py-8">
        <div class="relative w-64 h-64 flex items-center justify-center rounded-full border-4 {timerActive ? 'border-shed-orange animate-pulse' : 'border-stone-700'} bg-stone-800 shadow-2xl mb-8">
          <div class="text-center">
            <span class="block text-6xl font-mono font-bold">{formatTime(timerSeconds)}</span>
            <span class="text-xs uppercase text-stone-500 font-bold tracking-widest mt-1">Remaining</span>
          </div>
        </div>
        <div class="flex gap-3">
          <button on:click={toggleTimer} class="bg-white text-black px-10 py-4 rounded-lg font-bold text-sm uppercase tracking-widest hover:bg-stone-200 transition-colors">
            {timerActive ? "Pause Session" : "Start Timer"}
          </button>
          <button on:click={resetTimer} class="bg-stone-800 text-stone-300 px-5 py-4 rounded-lg border border-stone-700 font-bold text-sm uppercase tracking-widest hover:text-white transition-colors">
            Reset
          </button>
        </div>
      </div>

      <div class="grid md:grid-cols-2 gap-6">
        <div class="bg-stone-950 p-6 rounded-xl border border-stone-800">
          <h3 class="text-stone-400 text-xs font-bold uppercase mb-2">The Assignment</h3>
          <p class="text-xl font-medium leading-relaxed">{activeLesson.drill}</p>
        </div>
        {#if nextRoutineLesson}
          <div class="bg-stone-950/50 p-6 rounded-xl border border-stone-800 border-dashed flex flex-col justify-center">
            <h3 class="text-stone-600 text-xs font-bold uppercase mb-2">Next Up</h3>
            <p class="text-stone-300 font-bold">{nextRoutineLesson.title}</p>
            <p class="text-stone-500 text-sm line-clamp-2 mt-1">{nextRoutineLesson.drill}</p>
          </div>
        {:else if isRoutineActive}
          <div class="bg-green-900/20 p-6 rounded-xl border border-green-900/30 border-dashed flex items-center justify-center text-green-500/70 font-bold uppercase text-xs tracking-widest">
            Final Step
          </div>
        {/if}
      </div>
    </section>
  {:else if activeLearningView === "myshed"}
    <section class="bg-stone-900 border border-stone-800 rounded-2xl p-8 shadow-2xl">
      <div class="flex flex-col md:flex-row md:items-end justify-between gap-4 mb-8">
        <div>
          <h2 class="text-4xl font-serif font-black mb-2">My Shed</h2>
          <p class="text-stone-500">Your curated list of essential drills.</p>
        </div>
        {#if savedLessons.length > 0}
          <button on:click={startRoutine} class="bg-shed-orange hover:bg-orange-500 text-black px-6 py-3 rounded-lg font-bold uppercase tracking-widest shadow-lg transition-all">
            Start Routine
          </button>
        {/if}
      </div>

      {#if savedLessons.length === 0}
        <div class="text-center py-20 border-2 border-dashed border-stone-800 rounded-lg">
          <p class="text-stone-400">No lessons saved yet. Explore the volumes and save a drill.</p>
          <button on:click={() => activeLearningView = "library"} class="mt-4 text-shed-orange font-bold hover:underline">Explore Volumes</button>
        </div>
      {:else}
        <div class="grid gap-4">
          {#each savedLessons as lesson}
            <div class="bg-stone-950 p-6 rounded-lg border border-stone-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
              <div>
                <h3 class="font-bold font-serif text-lg text-white">{lesson.title}</h3>
                <p class="text-xs text-stone-500 uppercase font-bold mt-1">{lesson.artistName} / {lesson.duration || 10}m</p>
                <p class="text-sm text-stone-400 line-clamp-1 mt-2">{lesson.drill}</p>
              </div>
              <div class="flex items-center gap-3">
                <button on:click={() => toggleSaveLesson(lesson, { id: lesson.artistId, name: lesson.artistName, archetype: lesson.archetype }, { id: lesson.volumeId, title: lesson.volumeTitle })} class="px-3 py-2 text-shed-orange hover:bg-stone-800 rounded font-bold text-xs uppercase">
                  Remove
                </button>
                <button on:click={() => startPractice(lesson, { id: lesson.artistId, name: lesson.artistName, archetype: lesson.archetype }, { id: lesson.volumeId, title: lesson.volumeTitle })} class="bg-white text-black px-4 py-2 rounded font-bold text-xs uppercase tracking-wider hover:bg-stone-200">
                  Start Practice
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {:else if activeLearningView === "challenge"}
    <section class="bg-stone-900 border border-stone-800 rounded-2xl p-8 shadow-2xl min-h-[560px] flex flex-col justify-center">
      <div class="max-w-2xl mx-auto w-full">
        <div class="text-center mb-8">
          <h2 class="text-shed-orange font-bold tracking-widest uppercase text-sm mb-2">The Master Workbook</h2>
          <h3 class="text-5xl font-black font-serif">Daily Random Drill</h3>
        </div>
        {#if challengeCard}
          <div class="bg-stone-800 border-2 border-stone-700 p-8 rounded-xl shadow-2xl">
            <div class="flex justify-between items-start mb-6">
              <span class="bg-stone-900 text-stone-400 px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wider">#{challengeCard.id}</span>
              <span class="text-shed-orange font-serif italic text-lg">{challengeCard.artist}</span>
            </div>
            <h4 class="text-3xl font-bold mb-6 text-white leading-tight">{challengeCard.title}</h4>
            <p class="text-stone-300 leading-relaxed text-lg mb-8">{challengeCard.text}</p>
            <div class="flex justify-center">
              <button on:click={pickChallenge} class="bg-shed-orange hover:bg-orange-500 text-black px-6 py-3 rounded-lg font-bold transition-colors uppercase tracking-wider">
                New Card
              </button>
            </div>
          </div>
        {/if}
      </div>
    </section>
  {:else}
  <div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
    <!-- Sidebar: Volumes & Musicians -->
    <aside class="lg:col-span-1 flex flex-col gap-4">
      {#each currentVolumes as vol}
        <div class="flex flex-col gap-1">
          <h3 class="text-[10px] font-bold text-stone-500 uppercase px-2">{vol.title}</h3>
          {#each vol.musicians as mus}
            <button 
              on:click={() => { selectedVolume = vol; selectedMusician = mus; }}
              class="text-left px-3 py-2 rounded transition-all text-sm font-bold
              {selectedMusician.id === mus.id ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
              {mus.name}
            </button>
          {/each}
        </div>
      {/each}
    </aside>

    <!-- Main Content: Musician Detail -->
    <main class="lg:col-span-3 bg-stone-900 border border-stone-800 rounded-2xl p-8 shadow-2xl">
      <div class="flex flex-col gap-6">
        <header>
          <span class="text-xs font-bold {theme.accent} uppercase tracking-widest">{selectedMusician.archetype}</span>
          <h2 class="text-5xl font-black font-serif mt-2">{selectedMusician.name}</h2>
          <p class="text-xl italic text-stone-400 font-serif mt-4">"{selectedMusician.quote}"</p>
        </header>

        <!-- Origin with drop cap -->
        {#if selectedMusician.origin}
          <div class="border-l-4 border-stone-700 pl-6 py-2">
            <p class="text-lg leading-relaxed text-stone-300 font-serif">
              <span class="float-left text-5xl font-black font-serif mr-3 mt-[-4px] text-white leading-none">{selectedMusician.origin.charAt(0)}</span>
              {selectedMusician.origin.slice(1)}
            </p>
          </div>
        {/if}

        <div class="h-px bg-stone-800"></div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          {#each selectedMusician.lessons as lesson}
            {@const theoryLabel = lesson.theoryLabel || "The Theory"}
            {@const drillLabel = lesson.drillLabel || "The Drill"}
            {@const packedLesson = makeLesson(lesson)}
            <div class="bg-stone-950 border border-stone-800 p-6 rounded-xl flex flex-col gap-4 {theme.hoverBorder} transition-colors">
              <div class="flex justify-between items-start gap-3">
                <h3 class="text-xl font-bold font-serif text-white">{lesson.title}</h3>
                <button
                  on:click={() => toggleSaveLesson(lesson)}
                  class="text-xs font-black uppercase tracking-widest leading-none transition-colors {isSaved(packedLesson.id) ? theme.accent : 'text-stone-600 hover:text-white'}"
                  aria-label={isSaved(packedLesson.id) ? "Remove lesson from My Shed" : "Save lesson to My Shed"}>
                  {isSaved(packedLesson.id) ? "Saved" : "Save"}
                </button>
              </div>
              <div>
                <span class="text-[10px] font-bold uppercase tracking-wider {theme.accent} block mb-1">{theoryLabel}</span>
                <p class="text-sm text-stone-300 leading-relaxed italic">"{lesson.theory}"</p>
              </div>
              <div class="bg-stone-900 p-4 rounded-lg border-l-4 {theme.border}">
                <span class="text-[10px] font-bold uppercase tracking-wider text-stone-400 block mb-1">{drillLabel}</span>
                <p class="text-sm text-white font-medium leading-relaxed">{lesson.drill}</p>
              </div>
              <div class="flex justify-between items-center mt-auto pt-4 text-[10px] font-mono text-stone-500">
                <span>DURATION: {lesson.duration}m</span>
                <button
                  on:click={() => startPractice(lesson)}
                  class="text-stone-400 hover:text-shed-orange transition-colors">
                  START PRACTICE ({lesson.duration}m)
                </button>
              </div>
            </div>
          {/each}
        </div>

        <!-- Artist Challenge Section -->
        {#if selectedMusician.artistChallenge}
          <div class="mt-12 bg-stone-800 p-8 rounded-xl border border-stone-700">
            <span class="text-xs font-bold uppercase tracking-wider text-stone-400">Artist Challenge</span>
            <h3 class="text-xl font-bold text-white">{selectedMusician.artistChallenge.title}</h3>
            <p class="text-stone-300">{selectedMusician.artistChallenge.description}</p>
          </div>
        {/if}
      </div>
    </main>
  </div>
  {/if}
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
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #44403c;
  }
</style>
