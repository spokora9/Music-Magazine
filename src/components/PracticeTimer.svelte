<script>
  import { onDestroy } from "svelte";
  import { PRODUCER_VOLUMES, GUITAR_VOLUMES } from "../lib/learningData";

  const allLessons = [
    ...GUITAR_VOLUMES.flatMap(volume =>
      volume.musicians.flatMap(musician =>
        musician.lessons.map(lesson => ({
          ...lesson,
          artist: musician.name,
          volume: volume.title,
          discipline: "Musician"
        }))
      )
    ),
    ...PRODUCER_VOLUMES.flatMap(volume =>
      volume.musicians.flatMap(musician =>
        musician.lessons.map(lesson => ({
          ...lesson,
          artist: musician.name,
          volume: volume.title,
          discipline: "Producer"
        }))
      )
    )
  ];

  let selectedLesson = allLessons[0];
  let durationMinutes = selectedLesson?.duration || 10;
  let timeLeft = durationMinutes * 60;
  let timerActive = false;
  let intervalId = null;

  $: progress = durationMinutes > 0
    ? 1 - timeLeft / (durationMinutes * 60)
    : 0;

  $: displayTime = `${Math.floor(timeLeft / 60)}:${String(timeLeft % 60).padStart(2, "0")}`;

  function selectLesson(lesson) {
    selectedLesson = lesson;
    durationMinutes = lesson.duration || 10;
    resetTimer();
  }

  function startTimer() {
    if (timerActive) {
      pauseTimer();
      return;
    }

    if (timeLeft <= 0) {
      timeLeft = durationMinutes * 60;
    }

    timerActive = true;
    intervalId = window.setInterval(() => {
      if (timeLeft <= 1) {
        timeLeft = 0;
        pauseTimer();
      } else {
        timeLeft -= 1;
      }
    }, 1000);
  }

  function pauseTimer() {
    timerActive = false;
    if (intervalId) {
      window.clearInterval(intervalId);
      intervalId = null;
    }
  }

  function resetTimer() {
    pauseTimer();
    timeLeft = durationMinutes * 60;
  }

  onDestroy(() => {
    pauseTimer();
  });
</script>

<div class="h-full grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_22rem] gap-6 overflow-hidden">
  <section class="min-h-0 flex flex-col bg-stone-900 border border-stone-800 rounded-lg overflow-hidden">
    <header class="p-6 border-b border-stone-800 flex flex-col gap-2">
      <span class="text-xs font-bold text-shed-orange uppercase tracking-widest">Practice Mode</span>
      <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-4">
        <div>
          <h2 class="text-4xl font-black font-serif text-white">FOCUS TIMER</h2>
          <p class="text-stone-400 mt-2 max-w-2xl">
            Pick one drill, start the clock, and keep the assignment visible while you work.
          </p>
        </div>
        <div class="flex items-center gap-3 text-xs font-mono text-stone-500">
          <span>{allLessons.length} drills loaded</span>
          <span class="h-4 w-px bg-stone-700"></span>
          <span>{selectedLesson.discipline}</span>
        </div>
      </div>
    </header>

    <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar p-6">
      <div class="grid grid-cols-1 lg:grid-cols-[18rem_minmax(0,1fr)] gap-6">
        <aside class="flex flex-col gap-2">
          <h3 class="text-[10px] font-bold text-stone-500 uppercase tracking-widest px-1">Drill Library</h3>
          {#each allLessons as lesson}
            <button
              on:click={() => selectLesson(lesson)}
              class="text-left p-3 rounded border transition-all
              {selectedLesson.id === lesson.id ? 'bg-stone-800 border-shed-orange text-white' : 'bg-stone-950 border-stone-800 text-stone-400 hover:text-white hover:border-stone-700'}">
              <div class="flex items-start justify-between gap-3">
                <span class="font-bold text-sm leading-tight">{lesson.title}</span>
                <span class="text-[10px] font-mono text-stone-500 shrink-0">{lesson.duration || 10}m</span>
              </div>
              <div class="mt-1 text-[10px] uppercase tracking-wider text-stone-500">{lesson.artist}</div>
            </button>
          {/each}
        </aside>

        <article class="min-h-[36rem] bg-stone-950 border border-stone-800 rounded-lg p-6 flex flex-col">
          <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-4 mb-8">
            <div>
              <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500 mb-2">
                {selectedLesson.volume}
              </div>
              <h1 class="text-4xl md:text-5xl font-black font-serif text-white leading-none">
                {selectedLesson.title}
              </h1>
              <p class="text-shed-orange font-bold mt-3">{selectedLesson.artist}</p>
            </div>
            <label class="flex flex-col gap-2 text-[10px] font-bold uppercase tracking-widest text-stone-500">
              Minutes
              <input
                type="number"
                min="1"
                max="120"
                bind:value={durationMinutes}
                on:change={resetTimer}
                class="w-24 bg-stone-900 border border-stone-700 rounded px-3 py-2 text-white text-base font-mono outline-none focus:border-shed-orange" />
            </label>
          </div>

          <div class="flex-1 flex flex-col items-center justify-center gap-8">
            <div class="relative w-64 h-64 rounded-full border-4 border-stone-800 bg-stone-900 flex items-center justify-center shadow-2xl overflow-hidden">
              <div
                class="absolute inset-x-0 bottom-0 bg-shed-orange/20 transition-all duration-300"
                style="height: {Math.max(0, Math.min(1, progress)) * 100}%">
              </div>
              <div class="relative text-center">
                <span class="block text-6xl font-mono font-black text-white">{displayTime}</span>
                <span class="text-xs uppercase tracking-widest text-stone-500 font-bold">
                  {timerActive ? "In session" : timeLeft === 0 ? "Complete" : "Remaining"}
                </span>
              </div>
            </div>

            <div class="flex flex-wrap justify-center gap-3">
              <button
                on:click={startTimer}
                class="bg-white hover:bg-stone-200 text-black px-8 py-3 rounded font-black uppercase tracking-widest transition-all active:scale-95">
                {timerActive ? "Pause" : timeLeft === 0 ? "Restart" : "Start"}
              </button>
              <button
                on:click={resetTimer}
                class="bg-stone-800 hover:bg-stone-700 text-white px-8 py-3 rounded font-black uppercase tracking-widest border border-stone-700 transition-all active:scale-95">
                Reset
              </button>
            </div>
          </div>

          <div class="grid md:grid-cols-2 gap-4 mt-8">
            <div class="bg-stone-900 border border-stone-800 rounded-lg p-5">
              <span class="text-[10px] font-bold uppercase tracking-widest text-shed-orange">The Focus</span>
              <p class="text-stone-300 italic leading-relaxed mt-2">"{selectedLesson.theory}"</p>
            </div>
            <div class="bg-stone-900 border border-stone-800 rounded-lg p-5">
              <span class="text-[10px] font-bold uppercase tracking-widest text-stone-500">The Assignment</span>
              <p class="text-white font-medium leading-relaxed mt-2">{selectedLesson.drill}</p>
            </div>
          </div>
        </article>
      </div>
    </div>
  </section>

  <aside class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg p-6 flex flex-col gap-5 overflow-y-auto custom-scrollbar">
    <div>
      <h3 class="text-xs font-bold text-shed-orange uppercase tracking-widest">Session Notes</h3>
      <p class="text-sm text-stone-400 mt-2 leading-relaxed">
        Keep the loop small. The timer is for staying with one constraint long enough to hear what changes.
      </p>
    </div>

    <div class="h-px bg-stone-800"></div>

    <div class="grid gap-3 text-sm">
      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="font-bold text-white">1. Set the sound</div>
        <p class="text-stone-500 mt-1">Choose the instrument, kit, or backing track before starting.</p>
      </div>
      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="font-bold text-white">2. Run the drill</div>
        <p class="text-stone-500 mt-1">Do not redesign the task while the timer is active.</p>
      </div>
      <div class="bg-stone-950 border border-stone-800 rounded p-4">
        <div class="font-bold text-white">3. Capture one result</div>
        <p class="text-stone-500 mt-1">Record a loop, save a patch, or write down the usable move.</p>
      </div>
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
