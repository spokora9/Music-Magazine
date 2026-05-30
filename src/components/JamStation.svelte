<script>
  import { Audio } from '../lib/audio';
  import { NOTES, SCALES, BACKING_TRACKS, getMidiNumber, getChordNotes, getNoteIndex, getTrackKeyRoot, normalizeNoteName, transposeNote, transposeTrackToKey } from '../lib/data';
  import { onMount } from 'svelte';
  import { listen } from "@tauri-apps/api/event";
  import { isMidiLearnMode, jamHandoffTrack, jamVisualizerState } from "../lib/stores";
  import { SCALE_LABELS, chordScaleOptions, chordToneNames, progressionScaleLinks } from "../lib/modalMixture";
  import TheoryVisualizer from "./TheoryVisualizer.svelte";

  // State
  let activeTrack = null;
    let isPlaying = false;
    let currentChordIndex = 0;
    let currentChordNotes = [];
    let currentChordLabel = "";
    let visualizerRevision = 0;
  let learnMode = false;
  let restoringState = true;
  let playbackError = "";

  isMidiLearnMode.subscribe(v => learnMode = v);
  
  // Theory State
  let rootKey = 'C';
  let scaleType = 'minor';
    let viewMode = 'guitar'; // 'guitar' or 'piano'
    let jamSound = 0; // 0=Piano
    let jamPaceMode = "practice"; // practice caps long modal vamps for faster chord-scale work.
    
    // Enhancement Features
    let basslineEnabled = false;
    let harmonicsEnabled = false;
    let basslineStyle = 0; // 0=Root, 1=Octave, 2=Walking, 3=Rhythmic
    let basslinePreset = 0; // 0=Acoustic, 1=Electric, 2=ModernSynth
    let harmonicsPreset = 0; // 0=Close, 1=Open, 2=Drop-2, 3=Quartal, 4=Extensions
    
    // Custom Song Input
    let customMode = false;
    let customTempo = 120;
    let customParts = [""];
    let customChordErrors = [];
    let openJamPanel = "";
    let builderChordName = "C";
    let builderInsertIndex = -1;
    
    $: selectedBackingTrack = BACKING_TRACKS.find(track => track.id === activeTrack?.id);
    $: if (viewMode && activeTrack) publishVisualizerState();
    $: currentChord = activeTrack?.progression?.[currentChordIndex] || null;
    $: currentChordToneNames = currentChord ? chordToneNames(currentChord) : [];
    $: currentChordScaleOptions = currentChord ? chordScaleOptions(currentChord, rootKey, 8) : [];
    $: currentChordSelectedScale = normalizeScaleChoice(currentChord?.scaleChoice);
    $: currentVisualizerScale = effectiveScaleForChord(currentChord, rootKey, scaleType);
    $: visualizerRootKey = currentVisualizerScale.root;
    $: visualizerScaleType = currentVisualizerScale.type;
    $: progressionScaleOptions = activeTrack ? progressionScaleLinks(activeTrack, rootKey, 8) : [];
    $: customBuilderChords = parseCustomChordTokens(customParts).valid;

    function toPitchClasses(notes) {
        return (notes || [])
            .map(note => normalizeNoteName(String(note).replace(/\d/g, "")))
            .filter(note => NOTES.includes(note));
    }

    function playbackBeatLength(beats) {
        const length = Number(beats) || 4;
        return jamPaceMode === "original" ? length : Math.min(length, 4);
    }

    function trackToAudioChords(track) {
        return (track?.progression || []).map(chord => ({
            notes: (Array.isArray(chord.notes) && chord.notes.length ? chord.notes : getChordNotes(chord.name)).map(n => getMidiNumber(n)),
            beats: playbackBeatLength(chord.beats),
            name: chord.name
        }));
    }

    function playableAudioChords(track) {
        const chords = trackToAudioChords(track).filter(chord => chord.notes.length);
        if (!chords.length) {
            throw new Error("No playable chords were found for this track.");
        }
        return chords;
    }

    function playbackErrorMessage(error, fallback) {
        const detail = error?.message || String(error || "");
        return detail ? `${fallback}: ${detail}` : fallback;
    }

    function scaleLabel(root, type) {
        return `${root} ${SCALE_LABELS[type] || String(type || "").replace(/_/g, " ")}`.trim();
    }

    function normalizeScaleChoice(option) {
        if (!option?.root || !option?.type) return null;
        const root = normalizeNoteName(option.root);
        const type = String(option.type);
        if (getNoteIndex(root) === -1 || !SCALES[type]) return null;
        return {
            root,
            type,
            label: option.label || scaleLabel(root, type)
        };
    }

    function scaleChoiceKey(option) {
        const choice = normalizeScaleChoice(option);
        return choice ? `${choice.root}:${choice.type}` : "";
    }

    function effectiveScaleForChord(chord, homeRoot = rootKey, fallbackScaleType = scaleType) {
        const selected = normalizeScaleChoice(chord?.scaleChoice);
        if (selected) return { ...selected, source: "selected" };

        const suggested = chord ? normalizeScaleChoice(chordScaleOptions(chord, homeRoot, 1)[0]) : null;
        if (suggested) return { ...suggested, source: "suggested" };

        const normalizedHomeRoot = normalizeNoteName(homeRoot) || "C";
        const normalizedFallbackType = SCALES[fallbackScaleType] ? fallbackScaleType : "major";
        return {
            root: normalizedHomeRoot,
            type: normalizedFallbackType,
            label: scaleLabel(normalizedHomeRoot, normalizedFallbackType),
            source: "global"
        };
    }

    function selectedScaleMatches(option) {
        return scaleChoiceKey(option) === scaleChoiceKey(currentChordSelectedScale);
    }

    function isSameTrack(left, right) {
        return left === right || (left?.id && right?.id && left.id === right.id);
    }

    function setPlaybackError(error, fallback) {
        playbackError = playbackErrorMessage(error, fallback);
        console.error(fallback, error);
    }

    function publishVisualizerState(overrides = {}) {
        const scale = effectiveScaleForChord(activeTrack?.progression?.[currentChordIndex] || null, rootKey, scaleType);
        jamVisualizerState.set({
            track: activeTrack,
            isPlaying,
            currentChordIndex,
            currentChordNotes,
            currentChordLabel,
            rootKey: scale.root,
            scaleType: scale.type,
            scaleChoice: scale,
            globalRootKey: rootKey,
            globalScaleType: scaleType,
            viewMode,
            ...overrides
        });
    }

    function handleJamChordStepPayload(payload = {}) {
        if (!activeTrack || !isPlaying) return;
        const nextIndex = Number(payload.index);
        if (Number.isInteger(nextIndex) && activeTrack.progression[nextIndex]) {
            currentChordIndex = nextIndex;
            currentChordNotes = Array.isArray(payload.notes)
                ? payload.notes.map(note => NOTES[note % 12])
                : toPitchClasses(activeTrack.progression[nextIndex].notes);
            currentChordLabel = payload.label || activeTrack.progression[nextIndex].name;
            visualizerRevision += 1;
            playbackError = "";
            publishVisualizerState();
        }
    }
  
    onMount(() => {
        let unlistenJamControl = () => {};
        let unlistenJamChordStep = () => {};
        let queuedHandoffTrack = null;
        const handleBrowserJamChordStep = event => handleJamChordStepPayload(event.detail || {});

        if (typeof window !== "undefined") {
            window.addEventListener("shed-power:jam-chord-step", handleBrowserJamChordStep);
        }

        Audio.loadPersistence()
            .then(persisted => {
                const savedJam = persisted?.module_state?.jam;
                if (savedJam) restoreJamState(savedJam);
            })
            .catch(e => console.error("Failed to load Jam persistence:", e))
            .finally(() => {
                restoringState = false;
                Audio.setJamSound(jamSound).catch(e => console.error("Failed to set default Jam sound:", e));
                Audio.setBasslineEnabled(basslineEnabled).catch(e => console.error("Failed to restore bassline:", e));
                Audio.setHarmonicsEnabled(harmonicsEnabled).catch(e => console.error("Failed to restore harmonics:", e));
                Audio.setBasslineStyle(basslineStyle).catch(e => console.error("Failed to restore bassline style:", e));
                Audio.setBasslinePreset(basslinePreset).catch(e => console.error("Failed to restore bassline preset:", e));
                Audio.setHarmonicsPreset(harmonicsPreset).catch(e => console.error("Failed to restore harmonics preset:", e));
            });

        function consumeHandoffTrack(track) {
            if (!track) return;
            playTrack(track);
            jamHandoffTrack.set(null);
        }

        const unsubscribeHandoff = jamHandoffTrack.subscribe(track => {
            queuedHandoffTrack = track;
            if (unlistenJamControl) consumeHandoffTrack(track);
        });

        listen("jam-control", (event) => {
                const action = event.payload; // 0=Play, 1=Stop, 2=Next, 3=Prev
                console.log("JAM EVENT RECEIVED:", action);
                if (action === 0) {
                    if (isPlaying) stopTrack();
                    else playTrack(activeTrack || BACKING_TRACKS[0]);
                }
                else if (action === 1) stopTrack();
                else if (action === 2) nextTrack();
                else if (action === 3) prevTrack();
        })
            .then(unlisten => {
                unlistenJamControl = unlisten;
                consumeHandoffTrack(queuedHandoffTrack);
            })
            .catch(err => console.error("JamStation Mount Error:", err));

        listen("jam-chord-step", (event) => {
            handleJamChordStepPayload(event.payload || {});
        })
            .then(unlisten => {
                unlistenJamChordStep = unlisten;
            })
            .catch(err => console.error("JamStation chord-step listener error:", err));

        return () => {
            unsubscribeHandoff();
            if (typeof window !== "undefined") {
                window.removeEventListener("shed-power:jam-chord-step", handleBrowserJamChordStep);
            }
            unlistenJamControl();
            unlistenJamChordStep();
            publishVisualizerState();
        };
    });
  
    function nextTrack() {
        const idx = activeTrack ? BACKING_TRACKS.findIndex(track => track.id === activeTrack.id) : -1;
        const nextIdx = (idx + 1) % BACKING_TRACKS.length;
        playTrack(BACKING_TRACKS[nextIdx]);
    }

    function prevTrack() {
        const idx = activeTrack ? BACKING_TRACKS.findIndex(track => track.id === activeTrack.id) : 0;
        if (idx === -1) {
            playTrack(BACKING_TRACKS[0]);
            return;
        }
        const prevIdx = (idx - 1 + BACKING_TRACKS.length) % BACKING_TRACKS.length;
        playTrack(BACKING_TRACKS[prevIdx]);
    }

    function handleMainPlay() {
        if (learnMode) { Audio.learnMidi("jam_play", 0); return; }
        if (isPlaying) stopTrack();
        else playTrack(activeTrack || BACKING_TRACKS[0]);
    }

    function handleMainStop() {
        if (learnMode) { Audio.learnMidi("jam_stop", 0); return; }
        stopTrack();
    }

    function handleNext() {
        if (learnMode) { Audio.learnMidi("jam_next", 0); return; }
        nextTrack();
    }

    function handlePrev() {
        if (learnMode) { Audio.learnMidi("jam_prev", 0); return; }
        prevTrack();
    }

    function handleTrackSelect(event) {
        const track = BACKING_TRACKS.find(item => item.id === event.currentTarget.value);
        if (track) playTrack(track);
    }

    function toggleJamPanel(panel) {
        openJamPanel = openJamPanel === panel ? "" : panel;
        if (openJamPanel === "custom") customMode = true;
        else if (openJamPanel) customMode = false;
    }

    function playSelectedTrack(track) {
        openJamPanel = "";
        playTrack(track);
    }

    function applyGlobalScaleOption(option) {
        if (!option) return;
        rootKey = option.root;
        scaleType = option.type;
        publishVisualizerState();
        persistJamState();
    }

    function updateActiveTrackChord(index, updater) {
        if (!activeTrack?.progression?.[index]) return null;

        const progression = activeTrack.progression.map((chord, chordIndex) =>
            chordIndex === index ? updater(chord) : chord
        );

        activeTrack = {
            ...activeTrack,
            progression
        };

        return progression[index];
    }

    function applyChordScaleOption(option, index = currentChordIndex) {
        const scaleChoice = normalizeScaleChoice(option);
        if (!scaleChoice) return;

        const chord = updateActiveTrackChord(index, item => ({
            ...item,
            scaleChoice
        }));

        if (!chord) return;
        currentChordIndex = index;
        currentChordNotes = toPitchClasses(chord.notes?.length ? chord.notes : getChordNotes(chord.name));
        currentChordLabel = chord.name || "";
        visualizerRevision += 1;
        playbackError = "";
        publishVisualizerState({ rootKey: scaleChoice.root, scaleType: scaleChoice.type, scaleChoice });
        persistJamState();
    }

    function selectProgressionChord(index) {
        if (!activeTrack?.progression?.[index]) return;
        const chord = activeTrack.progression[index];
        currentChordIndex = index;
        currentChordNotes = toPitchClasses(chord.notes?.length ? chord.notes : getChordNotes(chord.name));
        currentChordLabel = chord.name || "";
        visualizerRevision += 1;
        publishVisualizerState();
    }

    function trackScaleChoices(track) {
        return (track?.progression || []).map(chord => normalizeScaleChoice(chord.scaleChoice));
    }

    function hasScaleChoices(track) {
        return trackScaleChoices(track).some(Boolean);
    }

    function applyStoredScaleChoices(track, scaleChoices = []) {
        if (!track || !Array.isArray(scaleChoices) || !scaleChoices.some(Boolean)) return track;
        return {
            ...track,
            progression: (track.progression || []).map((chord, index) => {
                const scaleChoice = normalizeScaleChoice(scaleChoices[index]);
                return scaleChoice ? { ...chord, scaleChoice } : { ...chord };
            })
        };
    }

    function serializeTrack(track) {
        if (!track) return null;
        const known = BACKING_TRACKS.find(item => item.id === track.id);
        if (known) {
            const value = { type: "backing", id: known.id };
            if (hasScaleChoices(track)) value.scaleChoices = trackScaleChoices(track);
            return value;
        }
        return {
            type: track.isSpark ? "spark" : "custom",
            track: JSON.parse(JSON.stringify(track))
        };
    }

    function deserializeTrack(value) {
        if (!value) return null;
        if (value.type === "backing") {
            const track = BACKING_TRACKS.find(item => item.id === value.id) || null;
            return applyStoredScaleChoices(track, value.scaleChoices);
        }
        return value.track || null;
    }

    function serializeJamState() {
        return {
            schema_version: 1,
            rootKey,
            scaleType,
            viewMode,
            jamSound: Number(jamSound),
            jamPaceMode,
            basslineEnabled,
            harmonicsEnabled,
            basslineStyle: Number(basslineStyle),
            basslinePreset: Number(basslinePreset),
            harmonicsPreset: Number(harmonicsPreset),
            customMode,
            customTempo: Number(customTempo),
            customParts,
            activeTrack: serializeTrack(activeTrack)
        };
    }

    function persistJamState() {
        if (restoringState) return;
        Audio.saveModuleState("jam", serializeJamState())
            .catch(e => console.error("Failed to persist Jam state:", e));
    }

    function restoreJamState(state) {
        rootKey = state.rootKey || rootKey;
        scaleType = state.scaleType || scaleType;
        viewMode = state.viewMode || viewMode;
        jamSound = Number.isFinite(Number(state.jamSound)) ? Number(state.jamSound) : jamSound;
        jamPaceMode = state.jamPaceMode === "original" ? "original" : "practice";
        basslineEnabled = Boolean(state.basslineEnabled);
        harmonicsEnabled = Boolean(state.harmonicsEnabled);
        basslineStyle = Number.isFinite(Number(state.basslineStyle)) ? Number(state.basslineStyle) : basslineStyle;
        basslinePreset = Number.isFinite(Number(state.basslinePreset)) ? Number(state.basslinePreset) : basslinePreset;
        harmonicsPreset = Number.isFinite(Number(state.harmonicsPreset)) ? Number(state.harmonicsPreset) : harmonicsPreset;
        customMode = Boolean(state.customMode);
        customTempo = Number.isFinite(Number(state.customTempo)) ? Number(state.customTempo) : customTempo;
        customParts = Array.isArray(state.customParts) && state.customParts.length ? state.customParts.map(String) : customParts;

        const restoredTrack = deserializeTrack(state.activeTrack);
        if (restoredTrack) {
            activeTrack = restoredTrack;
            currentChordIndex = 0;
            currentChordNotes = [];
            currentChordLabel = restoredTrack.progression?.[0]?.name || "";
            publishVisualizerState({ isPlaying: false });
        }
    }

    async function playTrack(track) {
        if (isSameTrack(activeTrack, track) && isPlaying) {
            await stopTrack();
            return;
        }

        if (!track?.progression?.length) {
            playbackError = "Choose a track with at least one chord.";
            return;
        }

        let audioChords;
        try {
            audioChords = playableAudioChords(track);
        } catch (error) {
            setPlaybackError(error, "Jam playback failed");
            return;
        }

        const stopped = await stopTrack(); // Ensure clean state
        if (!stopped) return;
        
        activeTrack = track;
        isPlaying = true;
        currentChordIndex = 0;
        const firstChord = track.progression[0];
        currentChordNotes = toPitchClasses(firstChord?.notes?.length ? firstChord.notes : getChordNotes(firstChord?.name));
        currentChordLabel = firstChord?.name || "";
        visualizerRevision += 1;

        // Auto-Set Visualizer Key/Scale
        const trackKey = String(track.key || "C");
        if (trackKey.endsWith("m")) {
            rootKey = trackKey.slice(0, -1);
            scaleType = "minor";
        } else {
            rootKey = trackKey;
            scaleType = "major";
        }

        try {
            await Audio.playJamTrack(audioChords, track.bpm);
            playbackError = "";
            publishVisualizerState();
            persistJamState();
        } catch (error) {
            isPlaying = false;
            currentChordNotes = [];
            currentChordLabel = "";
            setPlaybackError(error, "Jam playback failed");
            publishVisualizerState();
            persistJamState();
        }
    }

    async function applySelectedKeyToTrack() {
        if (!activeTrack?.progression?.length) {
            persistJamState();
            publishVisualizerState();
            return;
        }

        const sourceRoot = getTrackKeyRoot(activeTrack.key);
        const targetRoot = normalizeNoteName(rootKey) || "C";
        const sourceIndex = getNoteIndex(sourceRoot);
        const targetIndex = getNoteIndex(targetRoot);
        const semitones = sourceIndex === -1 || targetIndex === -1 ? 0 : targetIndex - sourceIndex;
        const transposedTrack = transposeTrackScaleChoices(transposeTrackToKey(activeTrack, rootKey, scaleType), semitones);
        activeTrack = transposedTrack;
        currentChordIndex = Math.min(currentChordIndex, activeTrack.progression.length - 1);
        const currentChord = activeTrack.progression[currentChordIndex] || activeTrack.progression[0];
        currentChordLabel = currentChord?.name || "";
        currentChordNotes = toPitchClasses(currentChord?.notes || []);
        visualizerRevision += 1;

        if (isPlaying) {
            try {
                await Audio.playJamTrack(playableAudioChords(activeTrack), activeTrack.bpm);
                playbackError = "";
            } catch (error) {
                isPlaying = false;
                currentChordNotes = [];
                currentChordLabel = "";
                setPlaybackError(error, "Failed to retune Jam playback");
            }
        }

        publishVisualizerState();
        persistJamState();
    }

    function transposeTrackScaleChoices(track, semitones) {
        if (!track?.progression?.length || !semitones) return track;
        return {
            ...track,
            progression: track.progression.map(chord => {
                const scaleChoice = normalizeScaleChoice(chord.scaleChoice);
                if (!scaleChoice) return chord;
                const root = transposeNote(scaleChoice.root, semitones);
                return {
                    ...chord,
                    scaleChoice: {
                        ...scaleChoice,
                        root,
                        label: scaleLabel(root, scaleChoice.type)
                    }
                };
            })
        };
    }

    async function stopTrack() {
        try {
            await Audio.stopChord();
            playbackError = "";
        } catch (error) {
            setPlaybackError(error, "Failed to stop Jam playback");
            publishVisualizerState();
            return false;
        }

        isPlaying = false;
        currentChordNotes = [];
        currentChordLabel = "";
        publishVisualizerState();
        persistJamState();
        return true;
    }
  
    function changeSound() {
        jamSound = Number(jamSound);
        Audio.setJamSound(jamSound);
        persistJamState();
    }

    async function changeJamPace() {
        jamPaceMode = jamPaceMode === "original" ? "original" : "practice";
        persistJamState();
        if (isPlaying && activeTrack?.progression?.length) {
            try {
                await Audio.playJamTrack(playableAudioChords(activeTrack), activeTrack.bpm);
                playbackError = "";
                publishVisualizerState();
            } catch (error) {
                isPlaying = false;
                setPlaybackError(error, "Failed to update Jam pace");
                publishVisualizerState();
            }
        }
    }

    // Enhancement Functions
    async function toggleBassline() {
        basslineEnabled = !basslineEnabled;
        await Audio.setBasslineEnabled(basslineEnabled);
        persistJamState();
    }

    async function toggleHarmonics() {
        harmonicsEnabled = !harmonicsEnabled;
        await Audio.setHarmonicsEnabled(harmonicsEnabled);
        persistJamState();
    }

    async function updateBasslineStyle(style) {
        basslineStyle = style;
        await Audio.setBasslineStyle(style);
        persistJamState();
    }

    async function updateBasslinePreset(preset) {
        basslinePreset = preset;
        await Audio.setBasslinePreset(preset);
        persistJamState();
    }

    async function updateHarmonicsPreset(preset) {
        harmonicsPreset = preset;
        await Audio.setHarmonicsPreset(preset);
        persistJamState();
    }

    // Custom Song Functions  
    function addCustomPart() {
        customParts = [...customParts, ""];
        persistJamState();
    }

    function removeCustomPart(index) {
        if (customParts.length > 1) {
            customParts = customParts.filter((_, i) => i !== index);
            persistJamState();
        }
    }

    function normalizeCustomChordToken(token) {
        return String(token || "")
            .trim()
            .replace(/^[\[\(]+|[\]\)]+$/g, "")
            .replace(/♭/g, "b");
    }

    function parseCustomChordTokens(parts) {
        const valid = [];
        const invalid = [];

        for (const part of parts) {
            const tokens = String(part || "").split(/[\s,|]+/);
            for (const rawToken of tokens) {
                const token = normalizeCustomChordToken(rawToken);
                if (!token) continue;

                const rootMatch = token.match(/^([A-Ga-g](?:#|b)?)/);
                if (rootMatch && getNoteIndex(rootMatch[1]) !== -1) {
                    valid.push(token);
                } else {
                    invalid.push(rawToken.trim());
                }
            }
        }

        return { valid, invalid };
    }

    function setCustomChordTokens(tokens) {
        customParts = [tokens.join(" ")];
        customChordErrors = [];
        persistJamState();
    }

    function preservedScaleChoicesForCustomProgression(chordNames) {
        const source = activeTrack?.progression || [];
        let sourceIndex = 0;

        return chordNames.map(chordName => {
            const normalizedName = normalizeCustomChordToken(chordName).toLowerCase();
            for (let index = sourceIndex; index < source.length; index += 1) {
                const sourceName = normalizeCustomChordToken(source[index]?.name).toLowerCase();
                if (sourceName === normalizedName) {
                    sourceIndex = index + 1;
                    return normalizeScaleChoice(source[index].scaleChoice);
                }
            }
            return null;
        });
    }

    function addBuilderChord() {
        const token = normalizeCustomChordToken(builderChordName);
        const rootMatch = token.match(/^([A-Ga-g](?:#|b)?)/);
        if (!token || !rootMatch || getNoteIndex(rootMatch[1]) === -1) {
            customChordErrors = token ? [token] : ["empty chord"];
            return;
        }

        const tokens = [...customBuilderChords];
        const insertAt = Math.min(Math.max(Number(builderInsertIndex) + 1, 0), tokens.length);
        tokens.splice(insertAt, 0, token);
        setCustomChordTokens(tokens);
        builderInsertIndex = insertAt;
        builderChordName = "";
    }

    function removeBuilderChord(index) {
        const tokens = customBuilderChords.filter((_, itemIndex) => itemIndex !== index);
        setCustomChordTokens(tokens.length ? tokens : [""]);
        builderInsertIndex = Math.min(builderInsertIndex, tokens.length - 1);
    }

    function loadActiveTrackIntoCustomBuilder() {
        if (!activeTrack?.progression?.length) return;
        setCustomChordTokens(activeTrack.progression.map(chord => chord.name));
        customMode = true;
        openJamPanel = "custom";
    }

    function updateCustomRawProgression(value) {
        customParts = [String(value || "")];
        customChordErrors = [];
        persistJamState();
    }

    function clearCustomProgression() {
        customParts = [""];
        customChordErrors = [];
        builderInsertIndex = -1;
        persistJamState();
    }

    async function playCustomSong() {
        const { valid, invalid } = parseCustomChordTokens(customParts);
        customChordErrors = invalid;
        if (!valid.length) {
            playbackError = "Enter at least one playable custom chord.";
            return;
        }

        const scaleChoices = preservedScaleChoicesForCustomProgression(valid);
        const track = {
            id: `custom-${Date.now()}`,
            title: "Custom Song",
            genre: "Custom",
            bpm: Number(customTempo) || 120,
            key: scaleType === "minor" ? `${rootKey}m` : rootKey,
            progression: valid.map((chordName, index) => {
                const chord = {
                    name: chordName,
                    beats: 4,
                    notes: getChordNotes(chordName),
                    theory: "Custom"
                };
                const scaleChoice = scaleChoices[index];
                return scaleChoice ? { ...chord, scaleChoice } : chord;
            })
        };

        if (!track.progression.length) {
            playbackError = "Enter at least one playable custom chord.";
            return;
        }

        let audioChords;
        try {
            audioChords = playableAudioChords(track);
        } catch (error) {
            setPlaybackError(error, "Custom Jam playback failed");
            return;
        }

        const stopped = await stopTrack();
        if (!stopped) return;
        activeTrack = track;
        isPlaying = true;
        currentChordIndex = 0;
        currentChordNotes = toPitchClasses(track.progression[0]?.notes || []);
        currentChordLabel = track.progression[0]?.name || "";
        visualizerRevision += 1;
        try {
            await Audio.playJamTrack(audioChords, track.bpm);
            playbackError = "";
            publishVisualizerState();
            persistJamState();
        } catch (error) {
            isPlaying = false;
            currentChordNotes = [];
            currentChordLabel = "";
            setPlaybackError(error, "Custom Jam playback failed");
            publishVisualizerState();
            persistJamState();
        }
    }
  </script>

<div class="h-full min-h-0 flex flex-col gap-4 overflow-hidden">
  <section class="shrink-0 bg-stone-900 rounded-lg p-4 border border-stone-800">
    <div class="flex flex-col 2xl:flex-row 2xl:items-center 2xl:justify-between gap-4">
      <div class="min-w-0">
        <div class="text-[10px] font-bold text-stone-500 uppercase tracking-[0.2em]">Jam Practice</div>
        <div class="mt-1 flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <span class="text-3xl font-black font-serif text-white">{currentChordLabel || activeTrack?.progression?.[currentChordIndex]?.name || "No chord"}</span>
          <span class="text-xs font-bold uppercase text-stone-500">{activeTrack?.title || "Choose a backing track"}</span>
          <span class="text-xs font-bold uppercase text-cyan-300">{currentVisualizerScale.label}</span>
          {#if isPlaying}
            <span class="text-[10px] font-bold uppercase tracking-widest text-green-400">Playing</span>
          {/if}
        </div>
      </div>

      <div class="flex flex-wrap items-end gap-3">
        <div class="flex items-center gap-2 bg-stone-950 p-1.5 rounded border border-stone-800">
          <button on:click={handlePrev} class="px-3 h-9 rounded bg-stone-800 flex items-center justify-center text-xs font-bold hover:bg-stone-700 {learnMode ? 'ring-2 ring-orange-500' : ''}">Prev</button>
          <button on:click={handleMainPlay} aria-label={isPlaying ? "Stop Jam playback" : "Start Jam playback"} class="w-11 h-9 rounded bg-cyan-600 flex items-center justify-center hover:bg-cyan-500 {learnMode ? 'ring-2 ring-orange-500' : ''}">
            {#if isPlaying}
              <div class="w-4 h-4 bg-white rounded-sm"></div>
            {:else}
              <div class="w-0 h-0 border-l-[12px] border-l-white border-y-[7px] border-y-transparent ml-1"></div>
            {/if}
          </button>
          <button on:click={handleNext} class="px-3 h-9 rounded bg-stone-800 flex items-center justify-center text-xs font-bold hover:bg-stone-700 {learnMode ? 'ring-2 ring-orange-500' : ''}">Next</button>
          <button on:click={handleMainStop} class="px-3 h-9 rounded bg-red-700/80 border border-red-500 text-xs font-bold uppercase text-white hover:bg-red-600 {learnMode ? 'ring-2 ring-orange-500' : ''}">Stop</button>
        </div>

        <label for="jam-track-main" class="flex flex-col gap-1 text-[10px] font-bold text-stone-500 uppercase">
          Track
          <select id="jam-track-main" value={selectedBackingTrack?.id || ""} on:change={handleTrackSelect} class="h-9 min-w-56 bg-stone-800 text-white font-bold text-xs rounded px-2 border border-stone-700 outline-none">
            {#if activeTrack && !selectedBackingTrack}
              <option value="">{activeTrack.title || "Custom Track"}</option>
            {/if}
            {#each BACKING_TRACKS as track}
              <option value={track.id}>{track.title}</option>
            {/each}
          </select>
        </label>

        <label for="jam-sound-main" class="flex flex-col gap-1 text-[10px] font-bold text-stone-500 uppercase">
          Sound
          <select id="jam-sound-main" bind:value={jamSound} on:change={changeSound} class="h-9 bg-stone-800 text-orange-300 font-bold text-xs rounded px-2 border border-stone-700 outline-none">
            <option value={0}>Grand Piano</option>
            <option value={1}>E-Piano</option>
            <option value={2}>Organ</option>
          </select>
        </label>

        <label for="jam-pace-main" class="flex flex-col gap-1 text-[10px] font-bold text-stone-500 uppercase">
          Pace
          <select id="jam-pace-main" bind:value={jamPaceMode} on:change={changeJamPace} class="h-9 bg-stone-800 text-green-300 font-bold text-xs rounded px-2 border border-stone-700 outline-none">
            <option value="practice">Practice</option>
            <option value="original">Original</option>
          </select>
        </label>

        <label for="jam-key-main" class="flex flex-col gap-1 text-[10px] font-bold text-stone-500 uppercase">
          Key
          <select id="jam-key-main" bind:value={rootKey} on:change={applySelectedKeyToTrack} class="h-9 bg-stone-800 text-cyan-300 font-bold text-xs rounded px-2 border border-stone-700 outline-none">
            {#each NOTES as note}
              <option value={note}>{note}</option>
            {/each}
          </select>
        </label>

        <label for="jam-scale-main" class="flex flex-col gap-1 text-[10px] font-bold text-stone-500 uppercase">
          Scale
          <select id="jam-scale-main" bind:value={scaleType} on:change={applySelectedKeyToTrack} class="h-9 bg-stone-800 text-cyan-300 font-bold text-xs rounded px-2 border border-stone-700 outline-none">
            {#each Object.keys(SCALES) as s}
              <option value={s}>{s}</option>
            {/each}
          </select>
        </label>

        <div class="flex items-center gap-1 h-9 bg-stone-800 p-1 rounded border border-stone-700">
          <button on:click={() => { viewMode = 'guitar'; persistJamState(); }} class="h-7 px-3 rounded text-xs font-bold uppercase transition-all {viewMode === 'guitar' ? 'bg-stone-700 text-cyan-300' : 'text-stone-500 hover:text-white'}">Guitar</button>
          <button on:click={() => { viewMode = 'piano'; persistJamState(); }} class="h-7 px-3 rounded text-xs font-bold uppercase transition-all {viewMode === 'piano' ? 'bg-stone-700 text-cyan-300' : 'text-stone-500 hover:text-white'}">Piano</button>
        </div>
      </div>
    </div>

    {#if playbackError}
      <div role="alert" class="mt-3 rounded border border-red-500/50 bg-red-950/40 px-3 py-2 text-xs font-bold text-red-200">
        {playbackError}
      </div>
    {/if}
  </section>

  <section class="flex-1 min-h-[20rem] bg-stone-900 rounded-lg border border-stone-800 p-4 shadow-inner overflow-hidden">
    {#key `${currentChordIndex}-${visualizerRootKey}-${visualizerScaleType}-${currentChordNotes.join("|")}-${visualizerRevision}`}
      <TheoryVisualizer
        rootKey={visualizerRootKey}
        scaleType={visualizerScaleType}
        {viewMode}
        {currentChordNotes}
        {currentChordLabel}
        showChordTones={true}
        showCommonTones={true}
        contextLabel="Jam Practice Visualizer"
      />
    {/key}
  </section>

  <section class="shrink-0 grid grid-cols-1 xl:grid-cols-[14rem_minmax(0,1fr)_minmax(0,1fr)] gap-3">
    <div class="bg-stone-900 rounded-lg border border-stone-800 p-3 min-w-0">
      <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Chord Tones</div>
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#each currentChordToneNames as tone}
          <span class="px-2 py-1 rounded bg-orange-500 text-black text-xs font-black">{tone}</span>
        {:else}
          <span class="text-xs text-stone-500">Choose or build a chord.</span>
        {/each}
      </div>
    </div>

    <div class="bg-stone-900 rounded-lg border border-stone-800 p-3 min-w-0">
      <div class="flex items-center justify-between gap-3">
        <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Current Chord Scales</div>
        <div class="text-[10px] font-mono {currentChordSelectedScale ? 'text-cyan-300' : 'text-stone-600'}">
          {currentChordSelectedScale ? "Selected" : currentChordScaleOptions.length}
        </div>
      </div>
      <div class="mt-2 flex gap-2 overflow-x-auto pb-1">
        {#each currentChordScaleOptions.slice(0, 6) as option}
          <button
            on:click={() => applyChordScaleOption(option)}
            aria-pressed={selectedScaleMatches(option)}
            class="min-w-40 rounded border px-3 py-2 text-left hover:border-cyan-400 {selectedScaleMatches(option) ? 'border-cyan-300 bg-cyan-950/50' : 'border-stone-700 bg-stone-950'}">
            <span class="block text-xs font-bold text-white truncate">{option.label}</span>
            <span class="block mt-1 text-[10px] font-mono text-stone-500 truncate">{option.notes.join(" ")}</span>
            <span class="block mt-1 text-[10px] font-bold uppercase {selectedScaleMatches(option) ? 'text-cyan-300' : 'text-stone-600'}">
              {selectedScaleMatches(option) ? "Applied" : "Use for chord"}
            </span>
          </button>
        {:else}
          <span class="text-xs text-stone-500">No chord selected.</span>
        {/each}
      </div>
    </div>

    <div class="bg-stone-900 rounded-lg border border-stone-800 p-3 min-w-0">
      <div class="flex items-center justify-between gap-3">
        <div class="text-[10px] font-bold uppercase tracking-widest text-stone-500">Progression Links</div>
        <div class="text-[10px] font-mono text-stone-600">{progressionScaleOptions.length}</div>
      </div>
      <div class="mt-2 flex gap-2 overflow-x-auto pb-1">
        {#each progressionScaleOptions.slice(0, 6) as option}
          <button on:click={() => applyGlobalScaleOption(option)} class="min-w-44 rounded border border-stone-700 bg-stone-950 px-3 py-2 text-left hover:border-green-400">
            <span class="block text-xs font-bold text-white truncate">{option.label}</span>
            <span class="block mt-1 text-[10px] font-mono {option.exact ? 'text-green-400' : 'text-yellow-400'}">
              {option.exact ? "Full cover" : `${Math.round(option.coverage * 100)}% cover`}
            </span>
            {#if !option.exact && option.missing.length}
              <span class="block mt-1 text-[10px] font-mono text-stone-500 truncate">Missing: {option.missing.join(" ")}</span>
            {/if}
          </button>
        {:else}
          <span class="text-xs text-stone-500">Add chords to find a bridge scale.</span>
        {/each}
      </div>
    </div>
  </section>

  {#if activeTrack}
    <div class="shrink-0 flex gap-2 overflow-x-auto pb-1">
      {#each activeTrack.progression as chord, i}
        <button on:click={() => selectProgressionChord(i)}
          class="min-w-36 px-3 py-2 rounded border transition-all text-left
          {currentChordIndex === i ? 'bg-orange-600 border-orange-500 shadow-lg' : 'bg-stone-900 border-stone-800 text-stone-500'}">
          <span class="block text-[10px] font-bold uppercase tracking-wider opacity-75 truncate">{chord.theory}</span>
          <span class="block text-lg font-black text-white truncate">{chord.name}</span>
          {#if chord.scaleChoice}
            <span class="block text-[10px] font-bold uppercase text-cyan-300 truncate">{chord.scaleChoice.label}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}

  <section class="shrink-0 bg-stone-900 rounded-lg border border-stone-800 overflow-hidden">
    <div class="grid grid-cols-1 md:grid-cols-3 divide-y md:divide-y-0 md:divide-x divide-stone-800">
      <button on:click={() => toggleJamPanel("songs")} class="px-4 py-3 flex items-center justify-between text-left hover:bg-stone-800/60 transition-colors">
        <span class="text-xs font-bold uppercase tracking-widest text-stone-300">Song Choices</span>
        <span class="text-xs font-mono text-stone-500">{openJamPanel === "songs" ? "Close" : "Open"}</span>
      </button>
      <button on:click={() => toggleJamPanel("enhancements")} class="px-4 py-3 flex items-center justify-between text-left hover:bg-stone-800/60 transition-colors">
        <span class="text-xs font-bold uppercase tracking-widest text-stone-300">Enhancements</span>
        <span class="text-xs font-mono text-stone-500">{openJamPanel === "enhancements" ? "Close" : "Open"}</span>
      </button>
      <button on:click={() => toggleJamPanel("custom")} class="px-4 py-3 flex items-center justify-between text-left hover:bg-stone-800/60 transition-colors">
        <span class="text-xs font-bold uppercase tracking-widest text-stone-300">Custom Song</span>
        <span class="text-xs font-mono text-stone-500">{openJamPanel === "custom" ? "Close" : "Open"}</span>
      </button>
    </div>

    {#if openJamPanel === "songs"}
      <div class="max-h-64 overflow-y-auto border-t border-stone-800 p-4">
        <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
          {#each BACKING_TRACKS as track}
            <button on:click={() => playSelectedTrack(track)}
              class="text-left p-3 rounded border transition-all
              {activeTrack?.id === track.id ? 'bg-orange-600 border-orange-500 shadow-lg' : 'bg-stone-800 border-stone-700 hover:bg-stone-700'}">
              <span class="text-[10px] font-bold uppercase tracking-wider {activeTrack?.id === track.id ? 'text-white/80' : 'text-stone-500'}">{track.genre}</span>
              <span class="block text-sm font-bold font-serif text-white mt-1 truncate">{track.title}</span>
              <span class="mt-2 flex justify-between text-[10px] font-mono text-stone-400">
                <span>{track.bpm} BPM</span>
                <span>{track.key}</span>
              </span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    {#if openJamPanel === "enhancements"}
      <div class="max-h-64 overflow-y-auto border-t border-stone-800 p-4">
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div class="rounded border border-stone-800 p-4 bg-stone-950">
            <div class="flex items-center justify-between gap-3">
              <div class="text-xs font-bold text-stone-300 uppercase">Bassline</div>
              <button on:click={toggleBassline}
                class="px-3 py-1 rounded text-xs font-bold transition-all {basslineEnabled ? 'bg-cyan-500 text-black' : 'bg-stone-700 text-stone-400'}">
                {basslineEnabled ? 'ON' : 'OFF'}
              </button>
            </div>

            {#if basslineEnabled}
              <div class="grid grid-cols-1 md:grid-cols-2 gap-2 mt-3">
                <select bind:value={basslineStyle} on:change={() => updateBasslineStyle(basslineStyle)} class="bg-stone-800 text-cyan-300 text-xs px-2 py-2 rounded border border-stone-700">
                  <option value={0}>Root Note</option>
                  <option value={1}>Octave Jump</option>
                  <option value={2}>Walking Bass</option>
                  <option value={3}>Rhythmic</option>
                </select>
                <select bind:value={basslinePreset} on:change={() => updateBasslinePreset(basslinePreset)} class="bg-stone-800 text-cyan-300 text-xs px-2 py-2 rounded border border-stone-700">
                  <option value={0}>Upright Acoustic Bass</option>
                  <option value={1}>Electric Bass</option>
                  <option value={2}>Modern Synth Bass</option>
                </select>
              </div>
            {/if}
          </div>

          <div class="rounded border border-stone-800 p-4 bg-stone-950">
            <div class="flex items-center justify-between gap-3">
              <div class="text-xs font-bold text-stone-300 uppercase">Harmonics</div>
              <button on:click={toggleHarmonics}
                class="px-3 py-1 rounded text-xs font-bold transition-all {harmonicsEnabled ? 'bg-purple-500 text-white' : 'bg-stone-700 text-stone-400'}">
                {harmonicsEnabled ? 'ON' : 'OFF'}
              </button>
            </div>

            {#if harmonicsEnabled}
              <select bind:value={harmonicsPreset} on:change={() => updateHarmonicsPreset(harmonicsPreset)} class="w-full mt-3 bg-stone-800 text-purple-300 text-xs px-2 py-2 rounded border border-stone-700">
                <option value={0}>Close Position</option>
                <option value={1}>Open Position</option>
                <option value={2}>Drop-2</option>
                <option value={3}>Quartal</option>
                <option value={4}>Extensions</option>
              </select>
            {/if}
          </div>
        </div>
      </div>
    {/if}

    {#if openJamPanel === "custom"}
      <div class="max-h-64 overflow-y-auto border-t border-stone-800 p-4">
        <div class="grid grid-cols-1 xl:grid-cols-[18rem_minmax(0,1fr)] gap-4">
          <div class="grid gap-3 content-start">
            <label for="jam-custom-tempo-main" class="flex flex-col gap-1 text-xs font-bold text-stone-500 uppercase">
              Tempo
              <input
                id="jam-custom-tempo-main"
                type="number"
                bind:value={customTempo}
                on:change={persistJamState}
                min="60"
                max="200"
                class="bg-stone-800 text-white px-3 py-2 rounded border border-stone-700 outline-none focus:border-shed-orange"
                placeholder="120">
            </label>

            <label for="jam-builder-chord" class="flex flex-col gap-1 text-xs font-bold text-stone-500 uppercase">
              Add Chord
              <input
                id="jam-builder-chord"
                type="text"
                bind:value={builderChordName}
                placeholder="Dm7"
                class="bg-stone-800 text-white px-3 py-2 rounded border border-stone-700 outline-none focus:border-shed-orange">
            </label>

            <label for="jam-builder-insert" class="flex flex-col gap-1 text-xs font-bold text-stone-500 uppercase">
              Insert Position
              <select id="jam-builder-insert" bind:value={builderInsertIndex} class="bg-stone-800 text-white px-3 py-2 rounded border border-stone-700 outline-none">
                <option value={-1}>At start</option>
                {#each customBuilderChords as chordName, i}
                  <option value={i}>After {i + 1}: {chordName}</option>
                {/each}
              </select>
            </label>

            <div class="flex flex-wrap gap-2">
              <button on:click={addBuilderChord} class="px-4 py-2 rounded bg-cyan-600 text-black font-bold text-xs uppercase hover:bg-cyan-500">
                Insert Chord
              </button>
              <button on:click={loadActiveTrackIntoCustomBuilder} class="px-4 py-2 rounded bg-stone-700 text-stone-300 hover:text-white font-bold text-xs uppercase">
                Use Current
              </button>
            </div>
          </div>

          <div class="min-w-0">
            <div class="flex flex-wrap gap-2 mb-3">
              {#each customBuilderChords as chordName, i}
                <div class="flex items-center gap-2 rounded border border-stone-700 bg-stone-800 px-2 py-1">
                  <span class="text-[10px] font-mono text-stone-500">{i + 1}</span>
                  <span class="text-sm font-black text-white">{chordName}</span>
                  <button on:click={() => removeBuilderChord(i)} class="text-xs font-black text-red-300 hover:text-red-100">X</button>
                </div>
              {:else}
                <span class="text-xs text-stone-500">Insert chords to build a progression.</span>
              {/each}
            </div>

            <label for="jam-custom-raw" class="text-xs font-bold text-stone-500 uppercase block mb-1">Raw Progression</label>
            <input
              id="jam-custom-raw"
              type="text"
              value={customParts[0] || ""}
              on:input={(event) => updateCustomRawProgression(event.currentTarget.value)}
              placeholder="Am F C G"
              class="w-full bg-stone-800 text-white px-3 py-2 rounded border border-stone-700 outline-none focus:border-shed-orange">

            <div class="flex flex-wrap gap-2 mt-3">
              <button on:click={clearCustomProgression} class="px-4 py-2 rounded bg-stone-700 text-stone-300 hover:text-white font-bold text-xs uppercase">
                Clear
              </button>
              <button on:click={playCustomSong} class="px-4 py-2 rounded bg-shed-orange text-black font-bold text-xs uppercase hover:bg-orange-500">
                Play Custom
              </button>
            </div>

            {#if customChordErrors.length}
              <div role="alert" class="mt-3 rounded border border-red-500/50 bg-red-950/40 px-3 py-2 text-xs font-bold text-red-200">
                Unsupported chord token{customChordErrors.length === 1 ? "" : "s"}: {customChordErrors.join(", ")}.
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </section>
</div>
