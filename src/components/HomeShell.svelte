<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { Audio } from "../lib/audio";

  const dispatch = createEventDispatcher();

  const workstationModules = [
    { id: "looper", title: "Looper", kicker: "Capture parts", text: "Record, overdub, arrange parts, and keep ideas moving." },
    { id: "synth", title: "Synthesizer", kicker: "Shape tone", text: "Play presets, map controls, and build playable sounds." },
    { id: "mpc", title: "MPC Sampler", kicker: "Build rhythm", text: "Trigger pads, sequence patterns, and sketch drum ideas." },
    { id: "jam", title: "Jam Station", kicker: "Practice harmony", text: "Run backing progressions, bass, harmony, and Spark handoffs." },
    { id: "lickLibrary", title: "Lick Library", kicker: "Learn lead lines", text: "Study artist-style licks, see the fretboard, and hand progressions to Jam Station." },
    { id: "tuner", title: "Tuner", kicker: "Pitch lock", text: "Tune from the microphone with a cents readout and calibration." },
    { id: "visualizer", title: "Visualizer", kicker: "Theory map", text: "Map scale and chord tones across guitar and piano layouts." }
  ];

  const learningModules = [
    { id: "magazine", title: "The Magazine", kicker: "Study", text: "Read producer and musician lessons, then save drills." },
    { id: "practice", title: "Practice Timer", kicker: "Focus", text: "Choose a drill and stay with one constraint until the clock ends." },
    { id: "challenge", title: "Challenge", kicker: "Random drill", text: "Draw a card when you need a specific creative constraint." },
    { id: "myshed", title: "My Shed", kicker: "Saved work", text: "Review saved lessons, sparks, and completed practice sessions." },
    { id: "spark", title: "Spark Generator", kicker: "Start a song", text: "Generate chord, mood, and constraint combinations for Jam Station." }
  ];

  let savedLessons = 0;
  let savedSparks = 0;
  let completedSessions = 0;
  let activeSession = null;
  let lastProject = null;
  let status = "Loading";

  function navigate(id) {
    dispatch("navigate", id);
  }

  async function loadHomeState() {
    try {
      const persisted = await Audio.loadPersistence();
      savedLessons = persisted.saved_lessons?.length || 0;
      savedSparks = persisted.saved_sparks?.length || 0;
      completedSessions = persisted.practice_state?.sessions?.length || 0;
      activeSession = persisted.practice_state?.active_session || null;
      lastProject = persisted.recent_projects?.[0] || null;
      status = "Ready";
    } catch (e) {
      console.error("Failed to load home state", e);
      status = "Storage unavailable";
    }
  }

  onMount(loadHomeState);
</script>

<div class="h-full overflow-y-auto custom-scrollbar">
  <div class="min-h-full flex flex-col gap-6">
    <section class="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_22rem] gap-6">
      <div class="bg-stone-900 border border-stone-800 rounded-lg p-8 flex flex-col justify-between min-h-[24rem]">
        <div>
          <span class="text-xs font-bold text-shed-orange uppercase tracking-widest">Home</span>
          <h2 class="text-5xl md:text-6xl font-black font-serif text-white mt-3 leading-none">
            SHED POWER
          </h2>
          <p class="text-xl text-stone-300 mt-5 max-w-3xl leading-relaxed">
            Start in the workstation, study a lesson, run a timed drill, or reopen the ideas saved in My Shed.
          </p>
        </div>

        <div class="flex flex-wrap gap-3 mt-10">
          <button on:click={() => navigate("looper")} class="bg-white hover:bg-stone-200 text-black px-6 py-3 rounded font-black uppercase tracking-widest">
            Open Workstation
          </button>
          <button on:click={() => navigate("magazine")} class="bg-shed-orange hover:bg-orange-500 text-black px-6 py-3 rounded font-black uppercase tracking-widest">
            Study Lessons
          </button>
          <button on:click={() => navigate("myshed")} class="bg-stone-800 hover:bg-stone-700 text-white px-6 py-3 rounded border border-stone-700 font-black uppercase tracking-widest">
            My Shed
          </button>
        </div>
      </div>

      <aside class="bg-stone-900 border border-stone-800 rounded-lg p-6 flex flex-col gap-5">
        <div class="flex items-center justify-between">
          <h3 class="text-xs font-bold text-shed-orange uppercase tracking-widest">Session State</h3>
          <span class="text-[10px] font-mono text-stone-500">{status}</span>
        </div>

        <div class="grid grid-cols-3 gap-3">
          <div class="bg-stone-950 border border-stone-800 rounded p-4">
            <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Lessons</div>
            <div class="text-2xl font-black text-white mt-1">{savedLessons}</div>
          </div>
          <div class="bg-stone-950 border border-stone-800 rounded p-4">
            <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Sparks</div>
            <div class="text-2xl font-black text-cyan-400 mt-1">{savedSparks}</div>
          </div>
          <div class="bg-stone-950 border border-stone-800 rounded p-4">
            <div class="text-[10px] uppercase tracking-widest text-stone-500 font-bold">Done</div>
            <div class="text-2xl font-black text-shed-orange mt-1">{completedSessions}</div>
          </div>
        </div>

        {#if activeSession}
          <button on:click={() => navigate("myshed")} class="text-left bg-stone-950 border border-shed-orange/40 rounded p-4 hover:border-shed-orange transition-colors">
            <div class="text-[10px] font-bold uppercase tracking-widest text-shed-orange">Active Practice</div>
            <div class="font-bold text-white mt-2">{activeSession.title}</div>
            <div class="text-xs text-stone-500 mt-1">{activeSession.duration} minutes</div>
          </button>
        {:else}
          <button on:click={() => navigate("practice")} class="text-left bg-stone-950 border border-stone-800 rounded p-4 hover:border-shed-orange/50 transition-colors">
            <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">No Active Practice</div>
            <div class="font-bold text-white mt-2">Start a focused drill</div>
            <div class="text-xs text-stone-500 mt-1">Open Practice Timer</div>
          </button>
        {/if}

        {#if lastProject}
          <div class="bg-stone-950 border border-stone-800 rounded p-4">
            <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Recent Project</div>
            <div class="text-sm text-stone-300 mt-2 break-all">{lastProject.path}</div>
          </div>
        {/if}
      </aside>
    </section>

    <section class="grid grid-cols-1 xl:grid-cols-2 gap-6">
      <div class="bg-stone-900 border border-stone-800 rounded-lg p-6">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-xs font-bold text-shed-orange uppercase tracking-widest">Workstation</h3>
          <span class="text-[10px] font-mono text-stone-500">{workstationModules.length} modules</span>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
          {#each workstationModules as module}
            <button on:click={() => navigate(module.id)} class="text-left bg-stone-950 border border-stone-800 rounded p-5 hover:border-stone-600 transition-colors">
              <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">{module.kicker}</div>
              <div class="text-xl font-black text-white mt-2">{module.title}</div>
              <p class="text-sm text-stone-400 mt-2 leading-relaxed">{module.text}</p>
            </button>
          {/each}
        </div>
      </div>

      <div class="bg-stone-900 border border-stone-800 rounded-lg p-6">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-xs font-bold text-cyan-400 uppercase tracking-widest">Learning And Creation</h3>
          <span class="text-[10px] font-mono text-stone-500">{learningModules.length} surfaces</span>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
          {#each learningModules as module}
            <button on:click={() => navigate(module.id)} class="text-left bg-stone-950 border border-stone-800 rounded p-5 hover:border-cyan-900 transition-colors">
              <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">{module.kicker}</div>
              <div class="text-xl font-black text-white mt-2">{module.title}</div>
              <p class="text-sm text-stone-400 mt-2 leading-relaxed">{module.text}</p>
            </button>
          {/each}
        </div>
      </div>
    </section>
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
    border-radius: 8px;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: #44403c;
  }
</style>
