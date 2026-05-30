// SHED POWER: Scale Shape & Position Engine
// Provides fretboard positions, diatonic chords, and degree maps for any key/scale.

import { NOTES, SCALES, getScaleNotes, getNoteIndex, getChordNotes, normalizeNoteName } from "./data.js";

// Standard guitar tuning: open note index in NOTES for each string (low E → high e)
export const STRING_TUNING = [4, 9, 2, 7, 11, 4]; // E A D G B e
export const STRING_LABELS = ["E (low)", "A", "D", "G", "B", "e (high)"];
export const FRET_COUNT = 15;

// Triad quality labels based on interval pattern
const TRIAD_QUALITIES = {
    "4-3": "maj",   // major 3rd + minor 3rd = major triad
    "3-4": "min",   // minor 3rd + major 3rd = minor triad
    "3-3": "dim",   // minor 3rd + minor 3rd = diminished triad
    "4-4": "aug"    // major 3rd + major 3rd = augmented triad
};

// Scale degree Roman numeral labels per triad quality
const ROMAN = ["I", "II", "III", "IV", "V", "VI", "VII"];
function romanLabel(degree, quality) {
    const r = ROMAN[degree] || `${degree + 1}`;
    if (quality === "min") return r.toLowerCase();
    if (quality === "dim") return r.toLowerCase() + "°";
    if (quality === "aug") return r + "+";
    return r;
}

export function getScaleDegreeMap(root, scaleType) {
    const notes = getScaleNotes(normalizeNoteName(root) || "C", scaleType);
    const map = {};
    notes.forEach((note, i) => { map[note] = i + 1; });
    return map;
}

export function getDiatonicChords(root, scaleType) {
    const scaleNotes = getScaleNotes(normalizeNoteName(root) || "C", scaleType);
    const n = scaleNotes.length;
    if (n < 3) return [];

    return scaleNotes.map((noteName, i) => {
        const third = scaleNotes[(i + 2) % n];
        const fifth = scaleNotes[(i + 4) % n];

        const rootIdx = getNoteIndex(noteName);
        const thirdIdx = getNoteIndex(third);
        const fifthIdx = getNoteIndex(fifth);

        const int3 = ((thirdIdx - rootIdx) + 12) % 12;
        const int53 = ((fifthIdx - thirdIdx) + 12) % 12;
        const quality = TRIAD_QUALITIES[`${int3}-${int53}`] || "maj";

        const suffix = quality === "min" ? "m" : quality === "dim" ? "dim" : quality === "aug" ? "aug" : "";
        const chordName = `${noteName}${suffix}`;

        return {
            degree: i + 1,
            roman: romanLabel(i, quality),
            name: chordName,
            quality,
            root: noteName,
            notes: getChordNotes(chordName)
        };
    });
}

// Returns the fret where `note` first appears on `stringIndex` (frets 0-11, then +12 for higher octave)
function noteOnString(noteIndex, stringTuning) {
    return ((noteIndex - stringTuning) + 12) % 12;
}

// All frets where a given note appears on a string within 0..FRET_COUNT
function allFretsForNoteOnString(noteIndex, stringTuning) {
    const first = noteOnString(noteIndex, stringTuning);
    const frets = [first];
    if (first + 12 <= FRET_COUNT) frets.push(first + 12);
    return frets;
}

// Full fretboard: for each (string, fret) return note info
export function buildFretboard(root, scaleType) {
    const normalized = normalizeNoteName(root) || "C";
    const rootIdx = getNoteIndex(normalized);
    const scaleNotes = getScaleNotes(normalized, scaleType);
    const degreeMap = getScaleDegreeMap(normalized, scaleType);

    const fretboard = STRING_TUNING.map((stringTuning, stringIndex) => {
        const frets = [];
        for (let fret = 0; fret <= FRET_COUNT; fret++) {
            const noteIdx = (stringTuning + fret) % 12;
            const noteName = NOTES[noteIdx];
            const inScale = scaleNotes.includes(noteName);
            const isRoot = noteIdx === rootIdx;
            const degree = degreeMap[noteName] || null;
            frets.push({ fret, noteName, inScale, isRoot, degree });
        }
        return { stringIndex, label: STRING_LABELS[stringIndex], frets };
    });

    return fretboard;
}

// The 5 positional box patterns for any scale.
// Each position is anchored to where the root appears on a different string.
// Returns array of 5 position objects sorted by startFret.
export function getScalePositions(root, scaleType) {
    const normalized = normalizeNoteName(root) || "C";
    const rootIdx = getNoteIndex(normalized);
    const scaleNotes = getScaleNotes(normalized, scaleType);
    const degreeMap = getScaleDegreeMap(normalized, scaleType);

    // Root fret on each of the first 5 strings (frets 0-11)
    const rootFretsPerString = STRING_TUNING.slice(0, 5).map(tuning =>
        ((rootIdx - tuning) + 12) % 12
    );

    // Build a position window centered 2 frets below the root on that string
    const raw = rootFretsPerString.map((rootFret, anchorString) => {
        // Prefer roots that are closer to the middle of the neck
        const adjustedRootFret = rootFret === 0 ? 12 : rootFret;
        const startFret = Math.max(0, adjustedRootFret - 2);
        const endFret = startFret + 4;

        // Get all notes in this window for each string
        const windowNotes = STRING_TUNING.map((tuning, stringIndex) => {
            const notes = [];
            for (let fret = startFret; fret <= endFret; fret++) {
                const noteIdx = (tuning + fret) % 12;
                const noteName = NOTES[noteIdx];
                if (scaleNotes.includes(noteName)) {
                    notes.push({
                        fret,
                        noteName,
                        isRoot: noteIdx === rootIdx,
                        degree: degreeMap[noteName] || null
                    });
                }
            }
            return notes;
        });

        // Find the lowest-fret scale note on low E — this names the position
        const lowestENote = windowNotes[0][0] || null;
        const lowestNoteName = lowestENote?.noteName || normalized;
        const lowestDegree = lowestENote?.degree || 1;
        const degreeNames = ["Root", "2nd", "3rd", "4th", "5th", "6th", "7th"];

        return {
            position: anchorString + 1,
            label: `Position ${anchorString + 1}`,
            startFret,
            endFret,
            anchorString,
            rootFretInWindow: adjustedRootFret <= endFret && adjustedRootFret >= startFret ? adjustedRootFret : null,
            startNote: lowestNoteName,
            startDegree: lowestDegree,
            startDegreeLabel: degreeNames[(lowestDegree - 1) % 7] || `${lowestDegree}`,
            windowNotes
        };
    });

    // Sort low-to-high on the neck, then renumber 1–5 consistently
    return raw
        .sort((a, b) => a.startFret - b.startFret)
        .map((pos, i) => ({ ...pos, position: i + 1, label: `Position ${i + 1}` }));
}

// Common progressions per scale type for the "Try it on" section
const PROGRESSION_TEMPLATES = {
    major: [
        { label: "I – IV – V", degrees: [1, 4, 5], beats: 4 },
        { label: "I – V – vi – IV", degrees: [1, 5, 6, 4], beats: 4 },
        { label: "ii – V – I", degrees: [2, 5, 1], beats: 4 },
        { label: "I – vi – IV – V", degrees: [1, 6, 4, 5], beats: 4 }
    ],
    minor: [
        { label: "i – bVI – bVII", degrees: [1, 6, 7], beats: 4 },
        { label: "i – iv – v", degrees: [1, 4, 5], beats: 4 },
        { label: "i – bVI – bIII – bVII", degrees: [1, 6, 3, 7], beats: 4 },
        { label: "ii° – V – i", degrees: [2, 5, 1], beats: 4 }
    ],
    dorian: [
        { label: "i – IV (Dorian vamp)", degrees: [1, 4], beats: 8 },
        { label: "i – ii – IV – i", degrees: [1, 2, 4, 1], beats: 4 }
    ],
    mixolydian: [
        { label: "I – bVII – IV", degrees: [1, 7, 4], beats: 4 },
        { label: "I – bVII – I", degrees: [1, 7, 1], beats: 8 }
    ],
    pentatonic_maj: [
        { label: "I – IV – V", degrees: [1, 4, 5], beats: 4 }
    ],
    pentatonic_min: [
        { label: "i – iv – v", degrees: [1, 4, 5], beats: 4 },
        { label: "i – bVII – bVI", degrees: [1, 5, 4], beats: 4 }
    ],
    blues: [
        { label: "12-Bar I – IV – V", degrees: [1, 4, 5], beats: 4 }
    ],
    lydian: [
        { label: "I – II – I (Lydian shimmer)", degrees: [1, 2, 1], beats: 8 }
    ],
    harmonic_minor: [
        { label: "i – V (Harmonic pull)", degrees: [1, 5], beats: 4 },
        { label: "i – iv – V", degrees: [1, 4, 5], beats: 4 }
    ],
    phrygian: [
        { label: "i – bII (Phrygian vamp)", degrees: [1, 2], beats: 8 },
        { label: "i – bII – bVII – i", degrees: [1, 2, 7, 1], beats: 4 }
    ]
};

export function getProgressionsForScale(root, scaleType) {
    const diatonic = getDiatonicChords(root, scaleType);
    const templates = PROGRESSION_TEMPLATES[scaleType] || PROGRESSION_TEMPLATES.major;

    return templates.map(template => {
        const chords = template.degrees.map(d => {
            const chord = diatonic[(d - 1) % diatonic.length];
            return chord
                ? { name: chord.name, beats: template.beats, notes: chord.notes, theory: chord.roman }
                : null;
        }).filter(Boolean);

        return {
            label: template.label,
            chords,
            track: chords.length ? {
                id: `scale-prog-${root}-${scaleType}-${template.degrees.join("-")}`,
                title: `${root} ${scaleType.replace(/_/g, " ")} — ${template.label}`,
                genre: "Scale Practice",
                bpm: 80,
                key: scaleType.includes("min") || scaleType === "dorian" || scaleType === "phrygian" || scaleType === "blues"
                    ? `${root}m` : root,
                progression: chords
            } : null
        };
    }).filter(p => p.track);
}
