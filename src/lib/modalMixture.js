import { NOTES, SCALES, getChordNotes, getNoteIndex, getScaleNotes, normalizeNoteName } from "./data.js";

export const SCALE_LABELS = {
    lydian: "Lydian",
    major: "Ionian",
    mixolydian: "Mixolydian",
    dorian: "Dorian",
    minor: "Aeolian",
    phrygian: "Phrygian",
    locrian: "Locrian",
    harmonic_major: "Harmonic Major",
    mixolydian_b6: "Mixolydian b6",
    harmonic_minor: "Harmonic Minor",
    melodic_minor: "Melodic Minor",
    pentatonic_maj: "Major Pentatonic",
    pentatonic_min: "Minor Pentatonic",
    blues: "Blues"
};

const SCALE_PRIORITY = [
    "major",
    "mixolydian",
    "lydian",
    "harmonic_major",
    "mixolydian_b6",
    "dorian",
    "minor",
    "melodic_minor",
    "harmonic_minor",
    "pentatonic_maj",
    "pentatonic_min",
    "blues",
    "phrygian",
    "locrian"
];

const SCALE_TYPES = SCALE_PRIORITY.filter(type => SCALES[type]);

function unique(values) {
    return [...new Set(values)];
}

function accidentalFromTail(tail) {
    if (tail.startsWith("#")) return "#";
    if (tail.toLowerCase().startsWith("b")) return "b";
    if (tail.startsWith("\u266f") || tail.includes("\u00af")) return "#";
    if (tail.startsWith("\u266d") || tail.includes("\u00ad")) return "b";
    return "";
}

function noteNameFrom(value) {
    const text = String(value || "").trim();
    const match = text.match(/^([A-Ga-g])/);
    if (!match) return "";
    return normalizeNoteName(`${match[1]}${accidentalFromTail(text.slice(1, 8))}`);
}

function noteToPitchClass(note) {
    if (typeof note === "number") return ((note % 12) + 12) % 12;
    const name = noteNameFrom(note);
    const index = getNoteIndex(name);
    return index === -1 ? null : index;
}

function pitchClassName(pitchClass) {
    return NOTES[((pitchClass % 12) + 12) % 12];
}

export function chordRootName(chordName) {
    return noteNameFrom(chordName);
}

export function chordPitchClasses(chord) {
    if (Array.isArray(chord?.notes) && chord.notes.length) {
        return unique(chord.notes.map(noteToPitchClass).filter(value => value != null));
    }

    const chordName = typeof chord === "string" ? chord : chord?.name;
    if (!chordRootName(chordName)) return [];

    return unique(getChordNotes(chordName).map(noteToPitchClass).filter(value => value != null));
}

export function chordToneNames(chord) {
    return chordPitchClasses(chord).map(pitchClassName);
}

function scalePitchClasses(root, type) {
    return getScaleNotes(root, type)
        .map(noteToPitchClass)
        .filter(value => value != null);
}

function scaleNotes(root, type) {
    return scalePitchClasses(root, type).map(pitchClassName);
}

function coveredPitchClasses(scalePitches, requiredPitches) {
    return requiredPitches.filter(pitch => scalePitches.includes(pitch));
}

function missingPitchClasses(scalePitches, requiredPitches) {
    return requiredPitches.filter(pitch => !scalePitches.includes(pitch));
}

function coverageScore(scalePitches, requiredPitches) {
    if (!requiredPitches.length) return 1;
    return coveredPitchClasses(scalePitches, requiredPitches).length / requiredPitches.length;
}

function semitoneClashes(scalePitches, chordPitches) {
    return unique(scalePitches.filter(scalePitch =>
        chordPitches.some(chordPitch => {
            const distance = Math.abs(scalePitch - chordPitch) % 12;
            return distance === 1 || distance === 11;
        }) && !chordPitches.includes(scalePitch)
    ));
}

function scalePriority(type) {
    const index = SCALE_PRIORITY.indexOf(type);
    return index === -1 ? SCALE_PRIORITY.length : index;
}

function scaleId(root, type) {
    const safeRoot = root.toLowerCase().replace("#", "-sharp");
    const safeType = type.replace(/_/g, "-");
    return `scale-${safeRoot}-${safeType}`;
}

function candidateScore(candidate, homeRoot, chordRoot) {
    let score = candidate.coveredCount * 100000 + Math.round(candidate.coverage * 10000);
    if (candidate.exact) score += 500000;
    if (candidate.root === homeRoot) score += 70;
    if (candidate.root === chordRoot) score += 35;
    if (candidate.root === homeRoot && candidate.root === chordRoot) score += 20;
    score += (SCALE_PRIORITY.length - scalePriority(candidate.type)) * 10;
    score -= candidate.clashes.length * 3;
    return score;
}

function buildCandidates(requiredPitches, homeRoot, chordRoot = "") {
    const candidates = [];

    for (const root of NOTES) {
        for (const type of SCALE_TYPES) {
            const pitches = scalePitchClasses(root, type);
            const covered = coveredPitchClasses(pitches, requiredPitches);
            const missing = missingPitchClasses(pitches, requiredPitches);
            const clashes = semitoneClashes(pitches, requiredPitches);
            const coverage = coverageScore(pitches, requiredPitches);

            candidates.push({
                id: scaleId(root, type),
                label: `${root} ${SCALE_LABELS[type] || type}`,
                root,
                type,
                notes: scaleNotes(root, type),
                exact: missing.length === 0,
                coverage,
                missing: missing.map(pitchClassName),
                clashes: clashes.map(pitchClassName),
                covered: covered.map(pitchClassName),
                coveredCount: covered.length,
                score: 0
            });
        }
    }

    return candidates
        .map(candidate => ({
            ...candidate,
            score: candidateScore(candidate, homeRoot, chordRoot)
        }))
        .sort((a, b) => b.score - a.score || a.label.localeCompare(b.label));
}

function withSuggestedOrder(candidates) {
    return candidates.map((candidate, index) => ({
        ...candidate,
        suggested: index + 1
    }));
}

export function chordScaleOptions(chord, homeRoot = "C", limit = 8) {
    const requiredPitches = chordPitchClasses(chord);
    if (!requiredPitches.length) return [];

    const chordRoot = chordRootName(typeof chord === "string" ? chord : chord?.name);
    const normalizedHomeRoot = noteNameFrom(homeRoot) || "C";

    return withSuggestedOrder(buildCandidates(requiredPitches, normalizedHomeRoot, chordRoot)
        .filter(candidate => candidate.exact)
        .slice(0, limit));
}

export function progressionScaleLinks(track, homeRoot = "C", limit = 8) {
    const requiredPitches = unique((track?.progression || []).flatMap(chordPitchClasses));
    if (!requiredPitches.length) return [];

    const normalizedHomeRoot = noteNameFrom(homeRoot) || "C";
    const candidates = buildCandidates(requiredPitches, normalizedHomeRoot)
        .filter(candidate => candidate.coverage > 0);
    const exact = candidates.filter(candidate => candidate.exact);
    const source = exact.length
        ? exact
        : candidates.filter(candidate => candidate.coveredCount === Math.max(...candidates.map(item => item.coveredCount)));

    return withSuggestedOrder(source.slice(0, limit));
}
