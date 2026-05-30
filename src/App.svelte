<script>
  import Looper from "./components/Looper.svelte";
  import Synth from "./components/Synth.svelte";
  import MPC from "./components/MPC.svelte";
  import JamStation from "./components/JamStation.svelte";
  import LickLibrary from "./components/LickLibrary.svelte";
  import Magazine from "./components/Magazine.svelte";
  import SparkGenerator from "./components/SparkGenerator.svelte";
  import ScaleExplorer from "./components/ScaleExplorer.svelte";
  import PracticeTimer from "./components/PracticeTimer.svelte";
  import ChallengeDeck from "./components/ChallengeDeck.svelte";
  import MyShed from "./components/MyShed.svelte";
  import HomeShell from "./components/HomeShell.svelte";
  import Tuner from "./components/Tuner.svelte";
  import Visualizer from "./components/Visualizer.svelte";
  import { Audio } from "./lib/audio";
  import { isMidiLearnMode, isMetronomeEnabled, jamVisualizerState, metronomeBpm } from "./lib/stores";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  let activeModule = "home"; 
  let learnMode = false;
  let micActive = true;
  let midiActive = true;
  let persistencePath = "";
  let recentProjects = [];
  let showProjectMenu = false;
  let persistenceStatus = "";
  
  // Debug State
  let showDebug = false;
  let engineInfo = { input_device: "Scanning...", sample_rate: 0 };
  let lastMidi = null;

  // Device Management State
  let deviceNotifications = [];
  let nextNotificationId = 1;
  const notificationTimers = new Map();
  const notificationTimeoutMs = 3500;
  let lastDeviceEvent = null;

  isMidiLearnMode.subscribe(v => learnMode = v);

  $: normalizeDeviceNotifications();

  onMount(async () => {
    try {
      await migrateLegacyBrowserState();
      const persisted = await Audio.loadPersistence();
      persistencePath = await Audio.getPersistencePath();
      applyPersistence(persisted);
    } catch (e) {
      console.error("Failed to load persistence", e);
    }

    Audio.setContext(activeModule);
    Audio.setInputState(micActive, midiActive);

    // Debug Listeners
    const unlistenInfo = await listen("engine-info", e => engineInfo = e.payload);
    const unlistenMidi = await listen("midi-debug", e => {
        lastMidi = e.payload;
    });

    const unlistenMidiActive = await listen("midi-active-state", e => {
        midiActive = e.payload;
    });

    const unlistenAllSoundsStopped = await listen("all-sounds-stopped", () => {
        applyAllSoundsStoppedState();
    });

    const unlistenSaveComplete = await listen("save-complete", e => {
        if (e.payload) {
          persistenceStatus = "Project saved";
          notify("✅ Project saved");
          refreshPersistenceState();
        } else {
          persistenceStatus = "Project save failed";
          notify("❌ Project save failed");
        }
    });

    const unlistenLoadComplete = await listen("load-complete", e => {
        const { all_empty, samples } = e.payload;
        persistenceStatus = all_empty ? "Project loaded empty" : `Project loaded (${samples} samples)`;
        notify(all_empty ? "Project loaded empty" : "Project loaded");
        refreshPersistenceState();
    });

    // Device Management Listeners
    const unlistenDeviceConnected = await listen("device-connected", e => {
        const { name, type } = e.payload;
        console.log(`Device connected: ${name} (${type})`);
        lastDeviceEvent = { action: "connected", name, type, time: new Date().toLocaleTimeString() };
        deviceNotifications.push(`✅ ${name} connected`);
        if (deviceNotifications.length > 3) deviceNotifications.shift();
        deviceNotifications = deviceNotifications;
    });

    const unlistenDeviceDisconnected = await listen("device-disconnected", e => {
        const { name, type } = e.payload;
        console.log(`Device disconnected: ${name} (${type})`);
        lastDeviceEvent = { action: "disconnected", name, type, time: new Date().toLocaleTimeString() };
        deviceNotifications.push(`❌ ${name} disconnected`);
        if (deviceNotifications.length > 3) deviceNotifications.shift();
        deviceNotifications = deviceNotifications;
    });

    return () => {
        unlistenInfo();
        unlistenMidi();
        unlistenMidiActive();
        unlistenAllSoundsStopped();
        unlistenSaveComplete();
        unlistenLoadComplete();
        unlistenDeviceConnected();
        unlistenDeviceDisconnected();
        notificationTimers.forEach(timeoutId => clearTimeout(timeoutId));
        notificationTimers.clear();
    };
  });

  async function migrateLegacyBrowserState() {
    const migrationFlag = "shed_power_legacy_migrated";
    if (localStorage.getItem(migrationFlag) === "1") return;

    const payload = {};
    for (const key of ["shed_midi_map", "theshed_saved", "theshed_sparks", "theshed_active_mixer_spark"]) {
      const raw = localStorage.getItem(key);
      if (!raw) continue;
      try {
        payload[key] = JSON.parse(raw);
      } catch (e) {
        console.warn(`Skipping invalid legacy storage key ${key}`, e);
      }
    }

    if (Object.keys(payload).length === 0) {
      localStorage.setItem(migrationFlag, "1");
      return;
    }

    try {
      await Audio.importLegacyBrowserState(payload);
      localStorage.setItem(migrationFlag, "1");
    } catch (e) {
      console.error("Legacy browser state migration failed", e);
    }
  }

  function applyPersistence(persisted) {
    if (!persisted) return;
    recentProjects = persisted.recent_projects || [];
    if (persisted.settings) {
      micActive = persisted.settings.mic_active;
      midiActive = persisted.settings.midi_active;
      activeModule = persisted.settings.active_module || activeModule;
      if (typeof persisted.settings.metronome_enabled === "boolean") {
        isMetronomeEnabled.set(persisted.settings.metronome_enabled);
      }
      if (Number.isFinite(Number(persisted.settings.metronome_bpm))) {
        metronomeBpm.set(Number(persisted.settings.metronome_bpm));
      }
    }
  }

  async function refreshPersistenceState() {
    try {
      const persisted = await Audio.loadPersistence();
      applyPersistence(persisted);
    } catch (e) {
      console.error("Failed to refresh persistence", e);
    }
  }

  function notify(message) {
    deviceNotifications = [...deviceNotifications, message].slice(-3);
  }

  function normalizeDeviceNotifications() {
    if (!deviceNotifications.some(notification => typeof notification === "string")) return;

    const normalized = deviceNotifications
      .map(notification => typeof notification === "string"
        ? { id: nextNotificationId++, message: notification }
        : notification)
      .slice(-3);
    const activeIds = new Set(normalized.map(notification => notification.id));

    for (const id of notificationTimers.keys()) {
      if (!activeIds.has(id)) clearNotificationTimer(id);
    }

    deviceNotifications = normalized;
    deviceNotifications.forEach(notification => scheduleNotificationDismiss(notification.id));
  }

  function scheduleNotificationDismiss(id) {
    if (notificationTimers.has(id)) return;
    const timeoutId = setTimeout(() => dismissNotification(id), notificationTimeoutMs);
    notificationTimers.set(id, timeoutId);
  }

  function dismissNotification(id) {
    clearNotificationTimer(id);
    deviceNotifications = deviceNotifications.filter(notification => notification.id !== id);
  }

  function clearNotificationTimer(id) {
    const timeoutId = notificationTimers.get(id);
    if (!timeoutId) return;
    clearTimeout(timeoutId);
    notificationTimers.delete(id);
  }

  function applyAllSoundsStoppedState() {
    isMetronomeEnabled.set(false);
    jamVisualizerState.update(state => ({
      ...state,
      isPlaying: false,
      currentChordNotes: [],
      currentChordLabel: ""
    }));
  }

  async function handleStopAllSounds() {
    try {
      await Audio.stopAllSounds();
      applyAllSoundsStoppedState();
      persistMetronomeSettings(false, $metronomeBpm);
      notify("All sounds stopped");
    } catch (e) {
      console.error("Failed to stop all sounds", e);
      notify("Stop all sounds failed");
    }
  }

  async function handleSave() {
    const path = prompt("Enter folder path to save project:", "C:/Users/Public/Shed_Project");
    if (path) {
      try {
        persistenceStatus = "Saving project";
        await Audio.saveProject(path);
      } catch (e) {
        console.error("Failed to request project save", e);
        persistenceStatus = "Project save failed";
        notify("❌ Project save failed");
      }
    }
  }

  async function handleLoad() {
    const path = prompt("Enter folder path to load project from:", "C:/Users/Public/Shed_Project");
    if (path) {
      try {
        persistenceStatus = "Loading project";
        await Audio.loadProject(path);
      } catch (e) {
        console.error("Failed to load project", e);
        persistenceStatus = `Load failed: ${e}`;
        notify("❌ Project load failed");
      }
    }
  }

  async function loadRecentProject(path) {
    try {
      persistenceStatus = "Loading project";
      await Audio.loadProject(path);
      showProjectMenu = false;
    } catch (e) {
      console.error("Failed to load recent project", e);
      persistenceStatus = `Load failed: ${e}`;
      notify("❌ Recent project invalid");
    }
  }

  async function exportPersistence() {
    const path = prompt("Export app state JSON to:", "C:/Users/Public/shed-power-app-state.json");
    if (!path) return;
    try {
      await Audio.exportPersistence(path);
      persistenceStatus = "Exported app state";
      notify("✅ App state exported");
    } catch (e) {
      console.error("Failed to export app state", e);
      persistenceStatus = `Export failed: ${e}`;
      notify("❌ App state export failed");
    }
  }

  async function importPersistence() {
    const path = prompt("Import app state JSON from:", "C:/Users/Public/shed-power-app-state.json");
    if (!path) return;
    try {
      const persisted = await Audio.importPersistenceFile(path);
      applyPersistence(persisted);
      Audio.setContext(activeModule);
      Audio.setInputState(micActive, midiActive);
      persistenceStatus = "Imported app state";
      notify("✅ App state imported");
    } catch (e) {
      console.error("Failed to import app state", e);
      persistenceStatus = `Import failed: ${e}`;
      notify("❌ App state import failed");
    }
  }

  function switchPage(page) {
      console.log("Switching to:", page);
      const leavingJamPractice =
        (activeModule === "jam" || activeModule === "visualizer") &&
        page !== "jam" &&
        page !== "visualizer";
      if (leavingJamPractice) {
        Audio.stopChord().catch(e => console.error("Failed to stop Jam playback on navigation", e));
        jamVisualizerState.update(state => ({
          ...state,
          isPlaying: false,
          currentChordNotes: [],
          currentChordLabel: ""
        }));
      }
      activeModule = page;
      Audio.setContext(page);
      persistSettings();
  }
  
  function toggleMic() {
    micActive = !micActive;
    Audio.setInputState(micActive, midiActive);
    persistSettings();
  }

  function toggleMidi() {
    midiActive = !midiActive;
    Audio.setInputState(micActive, midiActive);
    persistSettings();
  }

  function persistSettings() {
    const bpm = Number($metronomeBpm);
    Audio.saveAppSettings({
      mic_active: micActive,
      midi_active: midiActive,
      active_module: activeModule,
      metronome_enabled: $isMetronomeEnabled,
      metronome_bpm: Number.isFinite(bpm) ? bpm : 120
    }).catch(e => console.error("Failed to persist settings", e));
  }

  function persistMetronomeSettings(enabled, bpm) {
    const nextBpm = Number(bpm);
    Audio.saveAppSettings({
      metronome_enabled: enabled,
      metronome_bpm: Number.isFinite(nextBpm) ? nextBpm : 120
    }).catch(e => console.error("Failed to persist metronome settings", e));
  }

  async function refreshDevices() {
    try {
      await Audio.refreshMidi();
      deviceNotifications.push("🔄 MIDI refreshed");
    } catch (e) {
      deviceNotifications.push("❌ MIDI refresh failed");
    }
    if (deviceNotifications.length > 3) deviceNotifications.shift();
    deviceNotifications = deviceNotifications;
  }

  async function scanDevices() {
    try {
      const devices = await Audio.scanDevices();
      console.log("Devices found:", devices);
      deviceNotifications.push(`📡 Found ${devices.length} devices`);
    } catch (e) {
      deviceNotifications.push("❌ Device scan failed");
    }
    if (deviceNotifications.length > 3) deviceNotifications.shift();
    deviceNotifications = deviceNotifications;
  }

  // --- Keyboard Shortcuts ---
  let activeLooperPart = 0;

  function handleKeydown(e) {
    if (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA") return;

    switch (e.code) {
      case "Space":
        e.preventDefault();
        if (activeModule === "looper" || activeModule === "jam") {
          Audio.play();
        }
        break;
      case "KeyR":
        if (activeModule === "looper") {
          Audio.record(activeLooperPart);
        }
        break;
      case "KeyO":
        if (activeModule === "looper") {
          Audio.overdub(activeLooperPart);
        }
        break;
      case "Digit1":
      case "Numpad1":
        Audio.selectPart(0);
        activeLooperPart = 0;
        break;
      case "Digit2":
      case "Numpad2":
        Audio.selectPart(1);
        activeLooperPart = 1;
        break;
      case "Digit3":
      case "Numpad3":
        Audio.selectPart(2);
        activeLooperPart = 2;
        break;
      case "KeyM":
        isMetronomeEnabled.update(v => {
          const next = !v;
          persistMetronomeSettings(next, $metronomeBpm);
          return next;
        });
        break;
      case "KeyS":
        handleSave();
        break;
      case "KeyL":
        handleLoad();
        break;
      case "Escape":
        if (learnMode) {
          isMidiLearnMode.set(false);
        }
        break;
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<main class="w-screen h-screen flex flex-col bg-stone-950 text-white select-none overflow-hidden transition-all duration-300 {learnMode ? "cursor-crosshair border-4 border-orange-500 box-border" : ""}">
  <header class="h-16 bg-stone-900 border-b border-stone-800 flex items-center justify-between px-6 shrink-0 z-50">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 bg-shed-orange rounded flex items-center justify-center font-black text-black">SP</div>
      <h1 class="text-xl font-bold tracking-tight">SHED <span class="text-shed-orange">POWER</span></h1>
    </div>

    <div class="flex items-center gap-4 text-xs font-mono text-stone-500">
      <button on:click={handleStopAllSounds} class="px-3 py-1 rounded font-black uppercase tracking-wide transition-all bg-red-700/90 text-white border border-red-500 hover:bg-red-600 hover:border-red-400 shadow-[0_0_14px_rgba(185,28,28,0.35)]">
        Stop All
      </button>
      <button on:click={toggleMic} class="px-2 py-1 rounded font-bold uppercase transition-all {micActive ? 'bg-green-900/30 text-green-500 border border-green-800' : 'bg-stone-800 text-stone-500 border border-stone-700'}">
        MIC: {micActive ? "ON" : "OFF"}
      </button>
      <button on:click={toggleMidi} class="px-2 py-1 rounded font-bold uppercase transition-all {midiActive ? 'bg-cyan-900/30 text-cyan-500 border border-cyan-800' : 'bg-stone-800 text-stone-500 border border-stone-700'}">
        MIDI: {midiActive ? "ON" : "OFF"}
      </button>
      <div class="h-4 w-px bg-stone-800"></div>

      <button on:click={refreshDevices} class="px-2 py-1 rounded font-bold uppercase text-[10px] transition-all bg-stone-800 text-stone-500 border border-stone-700 hover:text-cyan-500 hover:border-cyan-500">
        🔄 MIDI
      </button>

      <button on:click={scanDevices} class="px-2 py-1 rounded font-bold uppercase text-[10px] transition-all bg-stone-800 text-stone-500 border border-stone-700 hover:text-blue-500 hover:border-blue-500">
        📡 SCAN
      </button>

      <button on:click={() => showDebug = !showDebug} class="text-stone-600 hover:text-white transition-colors">
        🐞
      </button>

      <button on:click={handleSave} class="bg-stone-800 hover:bg-stone-700 text-stone-300 px-3 py-1 rounded border border-stone-700 transition-colors uppercase font-bold">
        Save
      </button>

      <button on:click={handleLoad} class="bg-stone-800 hover:bg-stone-700 text-stone-300 px-3 py-1 rounded border border-stone-700 transition-colors uppercase font-bold">
        Load
      </button>

      <button on:click={() => showProjectMenu = !showProjectMenu} class="bg-stone-800 hover:bg-stone-700 text-stone-300 px-3 py-1 rounded border border-stone-700 transition-colors uppercase font-bold">
        Projects
      </button>
      <div class="h-4 w-px bg-stone-800"></div>

      <button on:click={() => isMidiLearnMode.update(v => !v)}
        class="flex items-center gap-2 px-3 py-1 rounded border transition-all uppercase font-bold
        {learnMode ? "bg-orange-600 border-orange-500 text-white animate-pulse" : "bg-stone-800 border-stone-700 text-stone-400 hover:text-white"}">
        <div class="w-2 h-2 rounded-full {learnMode ? "bg-white" : "bg-orange-500"}"></div>
        <span>{learnMode ? "LEARNING..." : "MIDI LEARN"}</span>
      </button>

      <span>CPU: 1%</span>
    </div>
  </header>

  {#if showProjectMenu}
    <div class="absolute top-16 right-6 z-[110] w-96 max-w-[calc(100vw-2rem)] bg-black/95 border border-stone-700 rounded-lg p-4 shadow-2xl font-mono text-xs text-stone-300 flex flex-col gap-4">
      <div class="flex justify-between items-center border-b border-stone-800 pb-2">
        <span class="font-bold text-white uppercase">Projects & App State</span>
        <button on:click={() => showProjectMenu = false} class="text-red-500 font-bold">X</button>
      </div>

      <div>
        <div class="flex justify-between items-center mb-2">
          <span class="text-stone-500 font-bold uppercase">Recent Projects</span>
          <span class="text-stone-600">{recentProjects.length}</span>
        </div>
        {#if recentProjects.length > 0}
          <div class="flex flex-col gap-2 max-h-48 overflow-y-auto custom-scrollbar">
            {#each recentProjects as project}
              <button on:click={() => loadRecentProject(project.path)} class="text-left bg-stone-900 hover:bg-stone-800 border border-stone-800 rounded p-2 transition-colors">
                <span class="text-white block truncate">{project.path}</span>
                <span class="text-stone-500 uppercase">{project.action} / schema {project.schema_version}</span>
              </button>
            {/each}
          </div>
        {:else}
          <p class="text-stone-600 border border-dashed border-stone-800 rounded p-4 text-center">No saved project paths yet.</p>
        {/if}
      </div>

      <div class="border-t border-stone-800 pt-3">
        <span class="text-stone-500 font-bold uppercase block mb-2">App State JSON</span>
        <p class="text-stone-600 break-all mb-3">{persistencePath || "Not loaded"}</p>
        <div class="flex gap-2">
          <button on:click={exportPersistence} class="flex-1 bg-stone-800 hover:bg-stone-700 text-stone-200 rounded px-3 py-2 font-bold uppercase">Export</button>
          <button on:click={importPersistence} class="flex-1 bg-stone-800 hover:bg-stone-700 text-stone-200 rounded px-3 py-2 font-bold uppercase">Import</button>
        </div>
        {#if persistenceStatus}
          <p class="text-stone-500 mt-2 uppercase">{persistenceStatus}</p>
        {/if}
      </div>
    </div>
  {/if}

  <!-- DEBUG PANEL -->
  {#if showDebug}
    <div class="absolute bottom-4 right-4 z-[100] w-64 bg-black/90 border border-stone-700 rounded-lg p-4 shadow-2xl font-mono text-[10px] text-green-400 flex flex-col gap-2">
        <div class="flex justify-between border-b border-stone-800 pb-1 mb-1">
            <span class="font-bold text-white">SYSTEM DIAGNOSTICS</span>
            <button on:click={() => showDebug = false} class="text-red-500 font-bold">X</button>
        </div>
        
        <div>
            <span class="text-stone-500">INPUT DEVICE:</span><br>
            <span class="text-white">{engineInfo.input_device}</span>
        </div>
        <div>
            <span class="text-stone-500">SAMPLE RATE:</span>
            <span class="text-white">{engineInfo.sample_rate} Hz</span>
        </div>
        
        <div class="mt-2 pt-2 border-t border-stone-800">
            <span class="text-stone-500">DEVICE STATUS:</span><br>
            {#if lastDeviceEvent}
                <div class="text-xs">
                    <span class="text-{lastDeviceEvent.action === 'connected' ? 'green' : 'red'}-400">
                        {lastDeviceEvent.action === 'connected' ? '✅' : '❌'} {lastDeviceEvent.name}
                    </span><br>
                    <span class="text-stone-600">{lastDeviceEvent.time} ({lastDeviceEvent.type})</span>
                </div>
            {:else}
                <span class="text-stone-600 italic">No device events</span>
            {/if}
        </div>
        
        <div class="mt-2 pt-2 border-t border-stone-800">
            <span class="text-stone-500">PERSISTENCE:</span><br>
            <span class="text-stone-600 break-all">{persistencePath || "Not loaded"}</span>
        </div>
        
        <div class="mt-2 pt-2 border-t border-stone-800">
            <span class="text-stone-500">LAST MIDI EVENT:</span><br>
            {#if lastMidi}
                <div class="grid grid-cols-3 gap-1 mt-1 text-center">
                    <div class="bg-stone-900 p-1 rounded border border-stone-700">
                        <div class="text-[8px] text-stone-500">STATUS</div>
                        <div class="text-cyan-400 font-bold">0x{lastMidi.status.toString(16).toUpperCase()}</div>
                    </div>
                    <div class="bg-stone-900 p-1 rounded border border-stone-700">
                        <div class="text-[8px] text-stone-500">DATA1</div>
                        <div class="text-orange-400 font-bold">{lastMidi.data1}</div>
                    </div>
                    <div class="bg-stone-900 p-1 rounded border border-stone-700">
                        <div class="text-[8px] text-stone-500">DATA2</div>
                        <div class="text-green-400 font-bold">{lastMidi.data2}</div>
                    </div>
                </div>
            {:else}
                <span class="text-stone-600 italic">Waiting for signal...</span>
            {/if}
        </div>
    </div>
  {/if}

  <!-- DEVICE NOTIFICATIONS -->
  {#if deviceNotifications.length > 0}
    <div class="fixed top-4 right-4 z-[100] flex max-w-[min(22rem,calc(100vw-2rem))] flex-col gap-2 pointer-events-none">
      {#each deviceNotifications as notification, i (notification.id)}
        <div class="bg-black/90 border border-stone-700 rounded px-3 py-2 shadow-2xl font-mono text-xs text-white animate-in slide-in-from-right duration-300"
             style="animation-delay: {i * 100}ms">
          {notification.message}
        </div>
      {/each}
    </div>
  {/if}

  <div class="flex-1 flex overflow-hidden">
    <nav class="w-64 bg-stone-900 border-r border-stone-800 flex flex-col p-4 gap-2 z-50 relative shrink-0">
      <button on:click={() => switchPage("home")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "home" ? "bg-shed-orange text-black shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        HOME
      </button>

      <div class="text-xs font-bold text-stone-500 uppercase tracking-widest mb-2 px-2">Workstation</div>

      <button on:click={() => switchPage("looper")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "looper" ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        LOOPER
      </button>

      <button on:click={() => switchPage("synth")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "synth" ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        SYNTHESIZER
      </button>

      <button on:click={() => switchPage("mpc")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "mpc" ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        MPC SAMPLER
      </button>

      <button on:click={() => switchPage("jam")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "jam" ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        JAM STATION
      </button>

      <button on:click={() => switchPage("lickLibrary")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "lickLibrary" ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        LICK LIBRARY
      </button>

      <button on:click={() => switchPage("tuner")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "tuner" ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        TUNER
      </button>

      <button on:click={() => switchPage("visualizer")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "visualizer" ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        VISUALIZER
      </button>

      <button on:click={() => switchPage("scales")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "scales" ? "bg-stone-800 text-white border border-stone-700 shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        SCALE EXPLORER
      </button>

      <div class="text-xs font-bold text-stone-500 uppercase tracking-widest mb-2 mt-6 px-2">Knowledge</div>

      <button on:click={() => switchPage("magazine")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "magazine" ? "bg-shed-orange text-black shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        THE MAGAZINE
      </button>

      <button on:click={() => switchPage("practice")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "practice" ? "bg-shed-orange text-black shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        PRACTICE TIMER
      </button>

      <button on:click={() => switchPage("challenge")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "challenge" ? "bg-shed-orange text-black shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        CHALLENGE
      </button>

      <button on:click={() => switchPage("myshed")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "myshed" ? "bg-shed-orange text-black shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        MY SHED
      </button>

      <button on:click={() => switchPage("spark")}
        class="text-left px-4 py-3 rounded transition-all font-bold text-sm
        {activeModule === "spark" ? "bg-cyan-600 text-black shadow-sm" : "text-stone-400 hover:text-white hover:bg-stone-800/50"}">
        SPARK GENERATOR
      </button>
    </nav>

    <div class="flex-1 bg-stone-950 relative overflow-hidden p-6 z-0">
      {#key activeModule}
        {#if activeModule === "home"}
          <HomeShell on:navigate={(event) => switchPage(event.detail)} />
        {:else if activeModule === "looper"}
          <Looper />
        {:else if activeModule === "synth"}
          <Synth />
        {:else if activeModule === "mpc"}
          <MPC />
        {:else if activeModule === "jam"}
          <JamStation />
        {:else if activeModule === "lickLibrary"}
          <LickLibrary />
        {:else if activeModule === "tuner"}
          <Tuner />
        {:else if activeModule === "visualizer"}
          <Visualizer />
        {:else if activeModule === "magazine"}
          <Magazine />
        {:else if activeModule === "practice"}
          <PracticeTimer />
        {:else if activeModule === "challenge"}
          <ChallengeDeck />
        {:else if activeModule === "myshed"}
          <MyShed on:openJam={() => switchPage("jam")} />
        {:else if activeModule === "spark"}
          <SparkGenerator on:openJam={() => switchPage("jam")} />
        {:else if activeModule === "scales"}
          <ScaleExplorer />
        {:else}
          <div class="h-full flex flex-col items-center justify-center opacity-50">
            <div class="text-6xl mb-4 text-stone-800">??</div>
            <h2 class="text-2xl font-bold text-stone-700 uppercase tracking-widest">Under Construction</h2>
          </div>
        {/if}
      {/key}
    </div>
  </div>
</main>

<style>
  :global(.text-shed-orange) {
    color: #ea580c;
  }
  :global(.bg-shed-orange) {
    background-color: #ea580c;
  }
</style>
