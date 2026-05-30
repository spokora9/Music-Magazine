import { writable } from 'svelte/store';

export const isMidiLearnMode = writable(false);
export const isMetronomeEnabled = writable(false);
export const metronomeBpm = writable(120);
export const jamHandoffTrack = writable(null);
export const jamVisualizerState = writable({
  track: null,
  isPlaying: false,
  currentChordIndex: 0,
  currentChordNotes: [],
  currentChordLabel: "",
  rootKey: "C",
  scaleType: "minor",
  viewMode: "guitar"
});
