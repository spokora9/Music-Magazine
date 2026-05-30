<script>
  import { onDestroy } from "svelte";
  import { MUSICIAN_CHALLENGE_CARDS, PRODUCER_CHALLENGE_CARDS } from "../lib/learningData";

  let mode = "musician";
  let currentChallenge = MUSICIAN_CHALLENGE_CARDS[0];
  let spinning = false;
  let spinInterval = null;

  $: challengeList = getChallengeList(mode);
  $: categoryLabel = mode === "musician" ? "Musician drills" : "Producer drills";

  function getChallengeList(nextMode) {
    return nextMode === "musician" ? MUSICIAN_CHALLENGE_CARDS : PRODUCER_CHALLENGE_CARDS;
  }

  function switchMode(nextMode) {
    mode = nextMode;
    stopSpin();
    currentChallenge = getChallengeList(nextMode)[0];
  }

  function drawChallenge() {
    stopSpin();
    spinning = true;
    let ticks = 0;

    spinInterval = window.setInterval(() => {
      currentChallenge = challengeList[Math.floor(Math.random() * challengeList.length)];
      ticks += 1;

      if (ticks >= 12) {
        stopSpin();
      }
    }, 75);
  }

  function stopSpin() {
    spinning = false;
    if (spinInterval) {
      window.clearInterval(spinInterval);
      spinInterval = null;
    }
  }

  onDestroy(stopSpin);
</script>

<div class="h-full grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_22rem] gap-6 overflow-hidden">
  <section class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg overflow-hidden flex flex-col">
    <header class="p-6 border-b border-stone-800 flex flex-col gap-4">
      <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-4">
        <div>
          <span class="text-xs font-bold text-shed-orange uppercase tracking-widest">Challenge Mode</span>
          <h2 class="text-4xl font-black font-serif text-white mt-2">DAILY RANDOM DRILL</h2>
          <p class="text-stone-400 mt-2 max-w-2xl">
            Draw one constraint, commit to it, and turn the result into a loop, take, or mix decision.
          </p>
        </div>

        <div class="flex gap-2">
          <button
            on:click={() => switchMode("musician")}
            class="px-4 py-2 rounded font-bold text-sm transition-all {mode === 'musician' ? 'bg-shed-orange text-black' : 'bg-stone-800 text-stone-400 hover:text-white'}">
            MUSICIAN
          </button>
          <button
            on:click={() => switchMode("producer")}
            class="px-4 py-2 rounded font-bold text-sm transition-all {mode === 'producer' ? 'bg-shed-orange text-black' : 'bg-stone-800 text-stone-400 hover:text-white'}">
            PRODUCER
          </button>
        </div>
      </div>
    </header>

    <div class="flex-1 min-h-0 p-6 flex items-center justify-center">
      <article class="w-full max-w-3xl bg-stone-950 border-2 border-stone-800 rounded-lg p-8 shadow-2xl transition-all {spinning ? 'opacity-70 scale-[0.98]' : 'opacity-100 scale-100'}">
        <div class="flex items-start justify-between gap-4 mb-8">
          <div>
            <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">{categoryLabel}</div>
            <h1 class="text-4xl md:text-5xl font-black font-serif text-white leading-tight mt-2">
              {currentChallenge.title}
            </h1>
          </div>
          <div class="bg-stone-900 border border-stone-800 rounded px-3 py-2 text-right shrink-0">
            <div class="text-[10px] uppercase tracking-widest text-stone-500">Card</div>
            <div class="font-mono font-bold text-shed-orange">#{currentChallenge.id}</div>
          </div>
        </div>

        <div class="mb-8">
          <span class="inline-flex items-center rounded bg-stone-900 border border-stone-800 px-3 py-1 text-sm font-serif italic text-shed-orange">
            {currentChallenge.artist}
          </span>
        </div>

        <p class="text-xl leading-relaxed text-stone-200">
          {currentChallenge.text}
        </p>

        <div class="mt-10 flex flex-wrap gap-3">
          <button
            on:click={drawChallenge}
            class="bg-shed-orange hover:bg-orange-500 text-black px-8 py-3 rounded font-black uppercase tracking-widest transition-all active:scale-95">
            {spinning ? "Drawing..." : "New Card"}
          </button>
          <button
            on:click={() => currentChallenge = challengeList[(challengeList.findIndex(challenge => challenge.id === currentChallenge.id) + 1) % challengeList.length]}
            class="bg-stone-800 hover:bg-stone-700 text-white px-8 py-3 rounded font-black uppercase tracking-widest border border-stone-700 transition-all active:scale-95">
            Next
          </button>
        </div>
      </article>
    </div>
  </section>

  <aside class="min-h-0 bg-stone-900 border border-stone-800 rounded-lg p-6 overflow-y-auto custom-scrollbar">
    <h3 class="text-xs font-bold text-shed-orange uppercase tracking-widest">Deck</h3>
    <p class="text-sm text-stone-400 mt-2 mb-5">
      {challengeList.length} challenge cards are available in this mode.
    </p>

    <div class="grid gap-2">
      {#each challengeList as challenge}
        <button
          on:click={() => { stopSpin(); currentChallenge = challenge; }}
          class="text-left p-3 rounded border transition-all
          {currentChallenge.id === challenge.id ? 'bg-stone-800 border-shed-orange text-white' : 'bg-stone-950 border-stone-800 text-stone-400 hover:text-white hover:border-stone-700'}">
          <div class="flex items-start justify-between gap-3">
            <span class="font-bold text-sm leading-tight">{challenge.title}</span>
            <span class="text-[10px] font-mono text-stone-500">#{challenge.id}</span>
          </div>
          <div class="mt-1 text-[10px] uppercase tracking-wider text-stone-500">{challenge.artist}</div>
        </button>
      {/each}
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
