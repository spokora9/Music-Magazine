// SHED POWER: Music Theory & Data Library

export const NOTES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

const ENHARMONIC_NOTES = {
    "B#": "C",
    "DB": "C#",
    "D♭": "C#",
    "EB": "D#",
    "E♭": "D#",
    "FB": "E",
    "GB": "F#",
    "G♭": "F#",
    "AB": "G#",
    "A♭": "G#",
    "BB": "A#",
    "B♭": "A#",
    "CB": "B",
    "E#": "F"
};

export function normalizeNoteName(note) {
    if (!note) return note;
    const trimmed = String(note).trim();
    const normalized = trimmed[0].toUpperCase() + trimmed.slice(1);
    return ENHARMONIC_NOTES[normalized.toUpperCase()] || normalized;
}

export function getNoteIndex(note) {
    return NOTES.indexOf(normalizeNoteName(note));
}

export const SCALES = {
    lydian: [0, 2, 4, 6, 7, 9, 11],
    major: [0, 2, 4, 5, 7, 9, 11],
    mixolydian: [0, 2, 4, 5, 7, 9, 10],
    dorian: [0, 2, 3, 5, 7, 9, 10],
    minor: [0, 2, 3, 5, 7, 8, 10],
    phrygian: [0, 1, 3, 5, 7, 8, 10],
    locrian: [0, 1, 3, 5, 6, 8, 10],
    harmonic_major: [0, 2, 4, 5, 7, 8, 11],
    mixolydian_b6: [0, 2, 4, 5, 7, 8, 10],
    harmonic_minor: [0, 2, 3, 5, 7, 8, 11],
    melodic_minor: [0, 2, 3, 5, 7, 9, 11],
    pentatonic_maj: [0, 2, 4, 7, 9],
    pentatonic_min: [0, 3, 5, 7, 10],
    blues: [0, 3, 5, 6, 7, 10]
};

// Helper: Get notes for a scale
export function getScaleNotes(root, type) {
    const rootIdx = getNoteIndex(root);
    if (rootIdx === -1) return [];

    const intervals = SCALES[type] || SCALES.major;
    return intervals.map(i => NOTES[(rootIdx + i) % 12]);
}

// Helper: Transpose logic
export function transposeNote(note, semitones) {
    const hasOctave = /\d/.test(note);
    const name = normalizeNoteName(hasOctave ? note.slice(0, -1) : note);
    const octave = hasOctave ? parseInt(note.slice(-1)) : 4;

    let idx = getNoteIndex(name);
    if (idx === -1) return note;

    let newIdx = idx + semitones;
    let octaveShift = Math.floor(newIdx / 12);
    newIdx = ((newIdx % 12) + 12) % 12;

    return NOTES[newIdx] + (hasOctave ? (octave + octaveShift) : "");
}

export function getTrackKeyRoot(key) {
    const rootMatch = String(key || "C").match(/^([A-Ga-g](?:#|b|â™­)?)/);
    return normalizeNoteName(rootMatch?.[1] || "C");
}

export function keyFromRootAndScale(root, scaleType) {
    const normalizedRoot = normalizeNoteName(root) || "C";
    const minorLike = scaleType === "minor" || scaleType === "pentatonic_min" || scaleType === "blues";
    return minorLike ? `${normalizedRoot}m` : normalizedRoot;
}

export function transposeChordName(chordName, semitones) {
    const raw = String(chordName || "").trim();
    const compact = raw.replace(/\s+/g, "");
    const rootMatch = compact.match(/^([A-Ga-g](?:#|b|â™­)?)/);
    if (!rootMatch) return raw;

    const transposedRoot = transposeNote(normalizeNoteName(rootMatch[1]), semitones);
    return `${transposedRoot}${compact.slice(rootMatch[1].length)}`;
}

export function transposeTrackToKey(track, root, scaleType = "major") {
    if (!track) return track;

    const targetRoot = normalizeNoteName(root) || "C";
    const sourceRoot = getTrackKeyRoot(track.key);
    const sourceIndex = getNoteIndex(sourceRoot);
    const targetIndex = getNoteIndex(targetRoot);
    const semitones = sourceIndex === -1 || targetIndex === -1 ? 0 : targetIndex - sourceIndex;

    return {
        ...track,
        key: keyFromRootAndScale(targetRoot, scaleType),
        progression: (track.progression || []).map(chord => {
            const name = transposeChordName(chord.name, semitones);
            return {
                ...chord,
                name,
                notes: Array.isArray(chord.notes)
                    ? chord.notes.map(note => transposeNote(note, semitones))
                    : getChordNotes(name)
            };
        })
    };
}

export function getMidiNumber(noteName) {
    const hasOctave = /\d/.test(noteName);
    if (!hasOctave) return 60;
    const name = normalizeNoteName(noteName.slice(0, -1));
    const octave = parseInt(noteName.slice(-1));
    const idx = getNoteIndex(name);
    if (idx === -1 || Number.isNaN(octave)) return 60;
    return idx + (octave + 1) * 12;
}

export function getChordNotes(chordName, octave = 3) {
    const raw = String(chordName || "").trim();
    const compact = raw.replace(/\s+/g, "");
    const rootMatch = compact.match(/^([A-Ga-g](?:#|b|♭)?)/);
    if (!rootMatch) return ["C3", "E3", "G3"];

    const rootName = normalizeNoteName(rootMatch[1]);
    const rootIdx = getNoteIndex(rootName);
    if (rootIdx === -1) return ["C3", "E3", "G3"];

    const quality = raw.toLowerCase();
    const suffix = compact.slice(rootMatch[1].length).toLowerCase();
    const isMaj7 = suffix.includes("maj7") || quality.includes("major 7");
    const isMinor = (suffix.startsWith("m") && !suffix.startsWith("maj")) || quality.includes("minor");
    const isDominant = quality.includes("dominant") || (suffix.includes("7") && !isMaj7 && !isMinor);

    let intervals;
    if (suffix.includes("m7b5") || suffix.includes("ø")) {
        intervals = [0, 3, 6, 10];
    } else if (suffix.includes("dim")) {
        intervals = [0, 3, 6];
    } else if (suffix.includes("sus2")) {
        intervals = [0, 2, 7];
    } else if (suffix.includes("sus4") || suffix.includes("sus")) {
        intervals = [0, 5, 7];
    } else if (isMinor) {
        intervals = [0, 3, 7];
        if (suffix.includes("7") || suffix.includes("9")) intervals.push(10);
        if (suffix.includes("9")) intervals.push(14);
    } else {
        intervals = [0, 4, 7];
        if (isMaj7) intervals.push(11);
        else if (isDominant) intervals.push(10);
        if (suffix.includes("9")) intervals.push(14);
    }

    const baseOctave = rootIdx >= NOTES.indexOf("F") ? octave - 1 : octave;
    return intervals.map(interval => {
        const absolute = rootIdx + interval;
        const note = NOTES[absolute % 12];
        const noteOctave = baseOctave + Math.floor(absolute / 12);
        return `${note}${noteOctave}`;
    });
}

export const BACKING_TRACKS = [
    // --- MODAL MIXTURE / BORROWED CHORD PRACTICE ---
    {
        id: "modal-mixture-mixolydian",
        title: "Modal Mixture: Major + Mixolydian",
        genre: "Modal Mixture",
        bpm: 96,
        key: "C",
        progression: [
            { name: "Cmaj7", beats: 8, notes: ["C2", "E3", "G3", "B3"], theory: "Imaj7 - Major home" },
            { name: "Bbmaj7", beats: 8, notes: ["Bb2", "D3", "F3", "A3"], theory: "bVIImaj7 - Mixolydian color" },
            { name: "F", beats: 4, notes: ["F2", "A3", "C4"], theory: "IV - Shared tone landing" },
            { name: "Cmaj7", beats: 4, notes: ["C2", "E3", "G3", "B3"], theory: "Imaj7 - Resolve" }
        ]
    },
    {
        id: "modal-mixture-lydian",
        title: "Modal Mixture: Borrow From Lydian",
        genre: "Modal Mixture",
        bpm: 92,
        key: "C",
        progression: [
            { name: "Cmaj7", beats: 8, notes: ["C2", "E3", "G3", "B3"], theory: "Imaj7 - Major home" },
            { name: "Dmaj7", beats: 8, notes: ["D2", "F#3", "A3", "C#4"], theory: "IImaj7 - Lydian lift" },
            { name: "G", beats: 4, notes: ["G2", "B3", "D4"], theory: "V - Back to gravity" },
            { name: "Cmaj7", beats: 4, notes: ["C2", "E3", "G3", "B3"], theory: "Imaj7 - Resolve" }
        ]
    },
    {
        id: "modal-mixture-major-minor",
        title: "Modal Mixture: Major + Minor",
        genre: "Modal Mixture",
        bpm: 84,
        key: "C",
        progression: [
            { name: "C", beats: 4, notes: ["C2", "E3", "G3"], theory: "I - Major home" },
            { name: "Fm", beats: 4, notes: ["F2", "Ab3", "C4"], theory: "iv - Borrowed from minor" },
            { name: "Ab", beats: 4, notes: ["Ab2", "C3", "Eb3"], theory: "bVI - Minor color" },
            { name: "Bb", beats: 4, notes: ["Bb2", "D3", "F3"], theory: "bVII - Neighbor color" },
            { name: "C", beats: 8, notes: ["C2", "E3", "G3"], theory: "I - Resolve" }
        ]
    },
    {
        id: "modal-mixture-amaj-gmaj",
        title: "Modal Mixture: Amaj7 to Gmaj7",
        genre: "Modal Mixture",
        bpm: 88,
        key: "A",
        progression: [
            { name: "Amaj7", beats: 8, notes: ["A2", "C#3", "E3", "G#3"], theory: "Imaj7 - Major home" },
            { name: "Gmaj7", beats: 8, notes: ["G2", "B3", "D4", "F#4"], theory: "bVIImaj7 - Borrowed color" },
            { name: "Dmaj7", beats: 4, notes: ["D2", "F#3", "A3", "C#4"], theory: "IVmaj7 - Shared tones" },
            { name: "Amaj7", beats: 4, notes: ["A2", "C#3", "E3", "G#3"], theory: "Imaj7 - Resolve" }
        ]
    },

    // --- TOP JAZZ STANDARDS ---
    {
        id: "jazz-so-what",
        title: "So What",
        genre: "Modal Jazz",
        bpm: 136,
        key: "D",
        progression: [
            { name: "Dm7", beats: 16, notes: ["D2", "F3", "A3", "C4"], theory: "i7 - Dorian Mode" },
            { name: "Ebm7", beats: 8, notes: ["Eb2", "Gb3", "Bb3", "Db4"], theory: "bii7 - Shift Up" },
            { name: "Dm7", beats: 8, notes: ["D2", "F3", "A3", "C4"], theory: "i7 - Back Home" }
        ]
    },
    {
        id: "jazz-autumn",
        title: "Autumn Leaves",
        genre: "Jazz Standard",
        bpm: 120,
        key: "Em",
        progression: [
            { name: "Am7", beats: 4, notes: ["A2", "C3", "E3", "G3"], theory: "iv7 - Circle" },
            { name: "D7", beats: 4, notes: ["D2", "F#3", "A3", "C4"], theory: "VII7 - Circle" },
            { name: "Gmaj7", beats: 4, notes: ["G2", "B3", "D4", "F#4"], theory: "IIImaj7 - Circle" },
            { name: "Cmaj7", beats: 4, notes: ["C3", "E3", "G3", "B3"], theory: "VImaj7 - Circle" },
            { name: "F#m7b5", beats: 4, notes: ["F#2", "A3", "C4", "E4"], theory: "ii7b5 - Minor 2-5" },
            { name: "B7", beats: 4, notes: ["B2", "D#3", "F#3", "A3"], theory: "V7 - Minor 2-5" },
            { name: "Em7", beats: 4, notes: ["E2", "G3", "B3", "D4"], theory: "i7 - Tonic" }
        ]
    },
    {
        id: "jazz-blue-bossa",
        title: "Blue Bossa",
        genre: "Latin Jazz",
        bpm: 140,
        key: "Cm",
        progression: [
            { name: "Cm7", beats: 8, notes: ["C2", "Eb3", "G3", "Bb3"], theory: "i7 - Tonic" },
            { name: "Fm7", beats: 4, notes: ["F2", "Ab3", "C4", "Eb4"], theory: "iv7" },
            { name: "Dm7b5", beats: 2, notes: ["D2", "F3", "Ab3", "C4"], theory: "ii7b5" },
            { name: "G7alt", beats: 2, notes: ["G2", "B3", "F3", "Ab3"], theory: "V7alt" },
            { name: "Cm7", beats: 4, notes: ["C2", "Eb3", "G3", "Bb3"], theory: "i7" },
            { name: "Ebm7", beats: 2, notes: ["Eb2", "Gb3", "Bb3", "Db4"], theory: "ii7 - Modulation" },
            { name: "Ab7", beats: 2, notes: ["Ab2", "C3", "Gb3", "Bb3"], theory: "V7 - Modulation" },
            { name: "Dbmaj7", beats: 4, notes: ["Db2", "F3", "Ab3", "C4"], theory: "Imaj7 - New Key" }
        ]
    },
    {
        id: "jazz-fly-me",
        title: "Fly Me To The Moon",
        genre: "Swing",
        bpm: 120,
        key: "C",
        progression: [
            { name: "Am7", beats: 4, notes: ["A2", "C3", "E3", "G3"], theory: "vi7" },
            { name: "Dm7", beats: 4, notes: ["D2", "F3", "A3", "C4"], theory: "ii7" },
            { name: "G7", beats: 4, notes: ["G2", "B3", "D4", "F4"], theory: "V7" },
            { name: "Cmaj7", beats: 4, notes: ["C2", "E3", "G3", "B3"], theory: "Imaj7" },
            { name: "Fmaj7", beats: 4, notes: ["F2", "A3", "C4", "E4"], theory: "IVmaj7" },
            { name: "Bm7b5", beats: 4, notes: ["B1", "D3", "F3", "A3"], theory: "vii7b5" },
            { name: "E7", beats: 4, notes: ["E2", "G#3", "B3", "D4"], theory: "III7" },
            { name: "Am7", beats: 4, notes: ["A2", "C3", "E3", "G3"], theory: "vi7" }
        ]
    },
    {
        id: "jazz-cantaloupe",
        title: "Cantaloupe Island",
        genre: "Jazz Funk",
        bpm: 110,
        key: "Fm",
        progression: [
            { name: "Fm7", beats: 16, notes: ["F2", "Ab3", "C4", "Eb4"], theory: "i7 - The Groove" },
            { name: "Db7", beats: 16, notes: ["Db2", "F3", "Ab3", "Cb4"], theory: "VI7 - Shift" },
            { name: "Dm7", beats: 16, notes: ["D2", "F3", "A3", "C4"], theory: "vi7 - Shift" },
            { name: "Fm7", beats: 16, notes: ["F2", "Ab3", "C4", "Eb4"], theory: "i7 - Return" }
        ]
    },
    
    // --- TOP POP/ROCK PROGRESSIONS ---
    {
        id: "prog-canon",
        title: "Canon in D (Full)",
        genre: "Classical / Pop",
        bpm: 75,
        key: "D",
        progression: [
            { name: "D", beats: 4, notes: ["D2", "F#3", "A3"], theory: "I" },
            { name: "A", beats: 4, notes: ["A2", "C#3", "E3"], theory: "V" },
            { name: "Bm", beats: 4, notes: ["B2", "D3", "F#3"], theory: "vi" },
            { name: "F#m", beats: 4, notes: ["F#2", "A3", "C#4"], theory: "iii" },
            { name: "G", beats: 4, notes: ["G2", "B3", "D4"], theory: "IV" },
            { name: "D", beats: 4, notes: ["D2", "F#3", "A3"], theory: "I" },
            { name: "G", beats: 4, notes: ["G2", "B3", "D4"], theory: "IV" },
            { name: "A", beats: 4, notes: ["A2", "C#3", "E3"], theory: "V" }
        ]
    },
    {
        id: "prog-axis",
        title: "4-Chord Axis",
        genre: "Pop Anthem",
        bpm: 120,
        key: "C",
        progression: [
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "I" },
            { name: "G", beats: 4, notes: ["G2", "B3", "D4"], theory: "V" },
            { name: "Am", beats: 4, notes: ["A2", "C3", "E3"], theory: "vi" },
            { name: "F", beats: 4, notes: ["F2", "A3", "C4"], theory: "IV" }
        ]
    },
    {
        id: "prog-blues",
        title: "12-Bar Blues",
        genre: "Blues",
        bpm: 100,
        key: "A",
        progression: [
            { name: "A7", beats: 16, notes: ["A2", "C#3", "G3"], theory: "I7" },
            { name: "D7", beats: 8, notes: ["D2", "F#3", "C4"], theory: "IV7" },
            { name: "A7", beats: 8, notes: ["A2", "C#3", "G3"], theory: "I7" },
            { name: "E7", beats: 4, notes: ["E2", "G#3", "D4"], theory: "V7" },
            { name: "D7", beats: 4, notes: ["D2", "F#3", "C4"], theory: "IV7" },
            { name: "A7", beats: 4, notes: ["A2", "C#3", "G3"], theory: "I7" },
            { name: "E7", beats: 4, notes: ["E2", "G#3", "D4"], theory: "V7" }
        ]
    },
    {
        id: "prog-andalusian",
        title: "Andalusian Cadence",
        genre: "Flamenco / Rock",
        bpm: 110,
        key: "Am",
        progression: [
            { name: "Am", beats: 4, notes: ["A2", "C3", "E3"], theory: "i" },
            { name: "G", beats: 4, notes: ["G2", "B3", "D4"], theory: "VII" },
            { name: "F", beats: 4, notes: ["F2", "A3", "C4"], theory: "VI" },
            { name: "E", beats: 4, notes: ["E2", "G#3", "B3"], theory: "V" }
        ]
    },
    {
        id: "prog-doowop",
        title: "50s Doo-Wop",
        genre: "Oldies",
        bpm: 90,
        key: "C",
        progression: [
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "I" },
            { name: "Am", beats: 4, notes: ["A2", "C3", "E3"], theory: "vi" },
            { name: "F", beats: 4, notes: ["F2", "A3", "C4"], theory: "IV" },
            { name: "G", beats: 4, notes: ["G2", "B3", "D4"], theory: "V" }
        ]
    },

    // --- GUITAR CHORD PROGRESSION LIBRARY (genre-spanning practice loops) ---
    {
        id: "rock-axis-g",
        title: "I–V–vi–IV Pop Axis (G)",
        genre: "Pop / Rock",
        bpm: 120,
        key: "G",
        progression: [
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "I - Home" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "V - Tension" },
            { name: "Em", beats: 4, notes: ["E3", "G3", "B3"], theory: "vi - Relative minor" },
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "IV - Lift" }
        ]
    },
    {
        id: "rock-pop-punk",
        title: "vi–IV–I–V Pop-Punk (G)",
        genre: "Pop Punk",
        bpm: 150,
        key: "G",
        progression: [
            { name: "Em", beats: 4, notes: ["E3", "G3", "B3"], theory: "vi - Drive" },
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "IV - Build" },
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "I - Anthem" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "V - Push" }
        ]
    },
    {
        id: "rock-three-chord",
        title: "I–IV–V Three-Chord Rock (A)",
        genre: "Classic Rock",
        bpm: 132,
        key: "A",
        progression: [
            { name: "A", beats: 8, notes: ["A2", "C#3", "E3"], theory: "I - Riff home" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "IV - Move" },
            { name: "E", beats: 4, notes: ["E3", "G#3", "B3"], theory: "V - Drive home" }
        ]
    },
    {
        id: "rock-mixolydian",
        title: "I–bVII–IV Mixolydian Rock (D)",
        genre: "Rock",
        bpm: 118,
        key: "D",
        progression: [
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "I - Tonic" },
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "bVII - Mixolydian color" },
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "IV - Open lift" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "I - Resolve" }
        ]
    },
    {
        id: "rock-50s-g",
        title: "I–vi–IV–V Doo-Wop (G)",
        genre: "Oldies",
        bpm: 96,
        key: "G",
        progression: [
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "I - Home" },
            { name: "Em", beats: 4, notes: ["E3", "G3", "B3"], theory: "vi - Sentiment" },
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "IV - Sweet" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "V - Turnaround" }
        ]
    },
    {
        id: "rock-epic-minor",
        title: "i–bVI–bVII Epic Minor (Em)",
        genre: "Cinematic Rock",
        bpm: 128,
        key: "Em",
        progression: [
            { name: "Em", beats: 4, notes: ["E3", "G3", "B3"], theory: "i - Minor home" },
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "bVI - Lift" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "bVII - Rise" },
            { name: "Em", beats: 4, notes: ["E3", "G3", "B3"], theory: "i - Return" }
        ]
    },
    {
        id: "rock-power-descent",
        title: "i–bVII–bVI Power Descent (Em)",
        genre: "Hard Rock",
        bpm: 138,
        key: "Em",
        progression: [
            { name: "Em", beats: 4, notes: ["E3", "G3", "B3"], theory: "i - Tonic" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "bVII - Step down" },
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "bVI - Weight" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "bVII - Turn" }
        ]
    },
    {
        id: "folk-country-shuffle",
        title: "I–IV–I–V Country Shuffle (G)",
        genre: "Country",
        bpm: 110,
        key: "G",
        progression: [
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "I - Home" },
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "IV - Move" },
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "I - Home" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "V - Turnaround" }
        ]
    },
    {
        id: "folk-ballad",
        title: "I–IV–vi–V Folk Ballad (C)",
        genre: "Folk",
        bpm: 92,
        key: "C",
        progression: [
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "I - Home" },
            { name: "F", beats: 4, notes: ["F2", "A2", "C3"], theory: "IV - Open" },
            { name: "Am", beats: 4, notes: ["A2", "C3", "E3"], theory: "vi - Reflective" },
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "V - Resolve" }
        ]
    },
    {
        id: "folk-campfire",
        title: "Campfire G–C–D–Em (G)",
        genre: "Acoustic Folk",
        bpm: 100,
        key: "G",
        progression: [
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "I - Strum" },
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "IV - Open" },
            { name: "D", beats: 4, notes: ["D3", "F#3", "A3"], theory: "V - Drive" },
            { name: "Em", beats: 4, notes: ["E3", "G3", "B3"], theory: "vi - Color" }
        ]
    },
    {
        id: "blues-e-12bar",
        title: "12-Bar Blues (E)",
        genre: "Blues",
        bpm: 96,
        key: "E",
        progression: [
            { name: "E7", beats: 16, notes: ["E3", "G#3", "B3", "D4"], theory: "I7 - Home" },
            { name: "A7", beats: 8, notes: ["A2", "C#3", "E3", "G3"], theory: "IV7 - Move" },
            { name: "E7", beats: 8, notes: ["E3", "G#3", "B3", "D4"], theory: "I7 - Home" },
            { name: "B7", beats: 4, notes: ["B2", "D#3", "F#3", "A3"], theory: "V7 - Tension" },
            { name: "A7", beats: 4, notes: ["A2", "C#3", "E3", "G3"], theory: "IV7 - Step down" },
            { name: "E7", beats: 4, notes: ["E3", "G#3", "B3", "D4"], theory: "I7 - Resolve" },
            { name: "B7", beats: 4, notes: ["B2", "D#3", "F#3", "A3"], theory: "V7 - Turnaround" }
        ]
    },
    {
        id: "blues-minor-am",
        title: "Slow Minor Blues (Am)",
        genre: "Blues",
        bpm: 66,
        key: "Am",
        progression: [
            { name: "Am", beats: 8, notes: ["A2", "C3", "E3"], theory: "i - Aching home" },
            { name: "Dm", beats: 8, notes: ["D3", "F3", "A3"], theory: "iv - Move" },
            { name: "Am", beats: 8, notes: ["A2", "C3", "E3"], theory: "i - Home" },
            { name: "E7", beats: 4, notes: ["E3", "G#3", "B3", "D4"], theory: "V7 - Tension" },
            { name: "Am", beats: 4, notes: ["A2", "C3", "E3"], theory: "i - Resolve" }
        ]
    },
    {
        id: "blues-quickchange-g",
        title: "Quick-Change Shuffle (G)",
        genre: "Blues Shuffle",
        bpm: 116,
        key: "G",
        progression: [
            { name: "G7", beats: 4, notes: ["G2", "B2", "D3", "F3"], theory: "I7 - Home" },
            { name: "C7", beats: 4, notes: ["C3", "E3", "G3", "A#3"], theory: "IV7 - Quick change" },
            { name: "G7", beats: 8, notes: ["G2", "B2", "D3", "F3"], theory: "I7 - Home" },
            { name: "D7", beats: 4, notes: ["D3", "F#3", "A3", "C4"], theory: "V7 - Tension" },
            { name: "C7", beats: 4, notes: ["C3", "E3", "G3", "A#3"], theory: "IV7 - Step down" },
            { name: "G7", beats: 4, notes: ["G2", "B2", "D3", "F3"], theory: "I7 - Resolve" }
        ]
    },
    {
        id: "jazz-2-5-1-major",
        title: "ii–V–I Major (C)",
        genre: "Jazz",
        bpm: 120,
        key: "C",
        progression: [
            { name: "Dm7", beats: 4, notes: ["D3", "F3", "A3", "C4"], theory: "ii7 - Setup" },
            { name: "G7", beats: 4, notes: ["G2", "B2", "D3", "F3"], theory: "V7 - Tension" },
            { name: "Cmaj7", beats: 8, notes: ["C3", "E3", "G3", "B3"], theory: "Imaj7 - Resolve" }
        ]
    },
    {
        id: "jazz-2-5-1-minor",
        title: "ii–V–i Minor (Am)",
        genre: "Jazz",
        bpm: 116,
        key: "Am",
        progression: [
            { name: "Bm7b5", beats: 4, notes: ["B2", "D3", "F3", "A3"], theory: "ii7b5 - Setup" },
            { name: "E7", beats: 4, notes: ["E3", "G#3", "B3", "D4"], theory: "V7 - Tension" },
            { name: "Am7", beats: 8, notes: ["A2", "C3", "E3", "G3"], theory: "i7 - Resolve" }
        ]
    },
    {
        id: "jazz-turnaround",
        title: "I–vi–ii–V Turnaround (C)",
        genre: "Jazz",
        bpm: 130,
        key: "C",
        progression: [
            { name: "Cmaj7", beats: 4, notes: ["C3", "E3", "G3", "B3"], theory: "Imaj7 - Home" },
            { name: "Am7", beats: 4, notes: ["A2", "C3", "E3", "G3"], theory: "vi7 - Pivot" },
            { name: "Dm7", beats: 4, notes: ["D3", "F3", "A3", "C4"], theory: "ii7 - Setup" },
            { name: "G7", beats: 4, notes: ["G2", "B2", "D3", "F3"], theory: "V7 - Turnaround" }
        ]
    },
    {
        id: "jazz-rhythm-changes",
        title: "Rhythm Changes A-Section (Bb)",
        genre: "Bebop",
        bpm: 160,
        key: "Bb",
        progression: [
            { name: "Bbmaj7", beats: 4, notes: ["A#2", "D3", "F3", "A3"], theory: "Imaj7 - Home" },
            { name: "Gm7", beats: 4, notes: ["G2", "A#2", "D3", "F3"], theory: "vi7 - Pivot" },
            { name: "Cm7", beats: 4, notes: ["C3", "D#3", "G3", "A#3"], theory: "ii7 - Setup" },
            { name: "F7", beats: 4, notes: ["F2", "A2", "C3", "D#3"], theory: "V7 - Turnaround" }
        ]
    },
    {
        id: "soul-1-3-4",
        title: "I–iii–IV Soul (C)",
        genre: "Soul",
        bpm: 100,
        key: "C",
        progression: [
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "I - Home" },
            { name: "Em", beats: 4, notes: ["E3", "G3", "B3"], theory: "iii - Warmth" },
            { name: "F", beats: 4, notes: ["F2", "A2", "C3"], theory: "IV - Lift" },
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "V - Resolve" }
        ]
    },
    {
        id: "neosoul-2-5-loop",
        title: "Neo-Soul ii–V Loop (C)",
        genre: "Neo-Soul",
        bpm: 84,
        key: "C",
        progression: [
            { name: "Dm7", beats: 4, notes: ["D3", "F3", "A3", "C4"], theory: "ii7 - Float" },
            { name: "G7", beats: 4, notes: ["G2", "B2", "D3", "F3"], theory: "V7 - Suspend" }
        ]
    },
    {
        id: "gospel-1-4-iv",
        title: "Gospel I–I7–IV–iv (C)",
        genre: "Gospel",
        bpm: 88,
        key: "C",
        progression: [
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "I - Home" },
            { name: "C7", beats: 4, notes: ["C3", "E3", "G3", "A#3"], theory: "I7 - Secondary dominant" },
            { name: "F", beats: 4, notes: ["F2", "A2", "C3"], theory: "IV - Lift" },
            { name: "Fm", beats: 4, notes: ["F2", "G#2", "C3"], theory: "iv - Borrowed minor" }
        ]
    },
    {
        id: "funk-dorian-vamp",
        title: "Dorian Funk Vamp (Em7–A7)",
        genre: "Funk",
        bpm: 108,
        key: "Em",
        progression: [
            { name: "Em7", beats: 8, notes: ["E3", "G3", "B3", "D4"], theory: "i7 - Groove" },
            { name: "A7", beats: 8, notes: ["A2", "C#3", "E3", "G3"], theory: "IV7 - Dorian color" }
        ]
    },
    {
        id: "reggae-skank",
        title: "Reggae I–V–vi–IV (C)",
        genre: "Reggae",
        bpm: 74,
        key: "C",
        progression: [
            { name: "C", beats: 4, notes: ["C3", "E3", "G3"], theory: "I - Skank" },
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "V - Lift" },
            { name: "Am", beats: 4, notes: ["A2", "C3", "E3"], theory: "vi - Roots minor" },
            { name: "F", beats: 4, notes: ["F2", "A2", "C3"], theory: "IV - Easy" }
        ]
    },
    {
        id: "spanish-phrygian",
        title: "Phrygian Flamenco Vamp (E)",
        genre: "Spanish",
        bpm: 120,
        key: "E",
        progression: [
            { name: "E", beats: 4, notes: ["E3", "G#3", "B3"], theory: "i - Phrygian home" },
            { name: "F", beats: 4, notes: ["F2", "A2", "C3"], theory: "bII - Spanish color" },
            { name: "G", beats: 4, notes: ["G2", "B2", "D3"], theory: "bIII - Rise" },
            { name: "F", beats: 4, notes: ["F2", "A2", "C3"], theory: "bII - Return" }
        ]
    }
];

export const PRODUCER_VOLUMES = [
    {
        id: "prod_vol1",
        title: "Volume I: The Architects",
        subtitle: "Structure, Melody, and The Math",
        musicians: [
            {
                id: "maxmartin",
                name: "Max Martin",
                archetype: "The Scientist",
                quote: "You can have the best lyrics in the world, but if the melody isn't catchy, no one will hear them.",
                lessons: [
                    {
                        id: "maxmartin-1",
                        title: "Melodic Math",
                        theory: "Syllables must mirror the rhythm. If a line feels clunky, rewrite the words to fit the melody, not the other way around.",
                        drill: "Take a 4-bar loop. Hum a melody using only 'Da-Da-Da'. Once it's catchy, write lyrics that match that exact rhythm.",
                        duration: 15
                    },
                    {
                        id: "maxmartin-2",
                        title: "The Chorus Payoff",
                        theory: "The chorus must open up. Higher notes, wider stereo image, more energy. It is the reward for the listener.",
                        drill: "Write a verse in a lower register. When the chorus hits, jump up an octave and add a harmony layer.",
                        duration: 20
                    }
                ]
            },
            {
                id: "rickrubin",
                name: "Rick Rubin",
                archetype: "The Reducer",
                quote: "I have no technical ability. I have no idea how to work the board. I know what I like and what I don't like.",
                lessons: [
                    {
                        id: "rubin-1",
                        title: "Reduction",
                        theory: "The best production is often what you remove. If an element isn't essential to the groove or the emotion, mute it.",
                        drill: "Play your full loop. Mute one track every 4 bars until only the absolute core remains. Build back from there.",
                        duration: 10
                    },
                    {
                        id: "rubin-2",
                        title: "Dry & In Your Face",
                        theory: "Reverb pushes sounds away. To make something intimate and powerful, keep it completely dry.",
                        drill: "Remove all reverb and delay from your vocal and snare. Adjust the volume to make it sit right without effects.",
                        duration: 10
                    }
                ]
            },
            {
                id: "drdre",
                name: "Dr. Dre",
                archetype: "The Sonic Architect",
                quote: "People hear the music, but they feel the drums.",
                lessons: [
                    {
                        id: "dre-1",
                        title: "The Perfect Kick",
                        theory: "The kick drum shouldn't just be heard, it should be physical. It needs its own frequency space, usually cleared out by EQ.",
                        drill: "Layer two kicks: one for sub (40-60Hz) and one for punch (100Hz+). Low-cut everything else below 100Hz.",
                        duration: 25
                    }
                ]
            }
        ]
    },
    {
        id: "prod_vol2",
        title: "Volume II: The Timekeepers",
        subtitle: "Groove, Swing, and Feel",
        musicians: [
            {
                id: "jdilla",
                name: "J Dilla",
                archetype: "The Humanizer",
                quote: "I don't use the grid. I use my ears.",
                lessons: [
                    {
                        id: "dilla-1",
                        title: "Drunk Funk",
                        theory: "Quantization kills the vibe. By disabling the grid, you create a push-pull tension that feels human.",
                        drill: "Turn off quantization. Record your hi-hats straight. Then record the kick slightly late, and the snare slightly early.",
                        duration: 30
                    },
                    {
                        id: "dilla-2",
                        title: "Sample Chopping",
                        theory: "Don't just loop the break. Chop it into individual hits and replay them in a new pattern.",
                        drill: "Load a drum break. Slice it into Kick, Snare, and Hat. Re-sequence them into a completely new beat.",
                        duration: 45
                    }
                ]
            },
            {
                id: "prince",
                name: "Prince",
                archetype: "The Auteur",
                quote: "Cool means being able to hang with yourself.",
                lessons: [
                    {
                        id: "prince-1",
                        title: "The Minneapolis Sound",
                        theory: "Replace the bass guitar with a synth. Use a drum machine for the beat, but play live percussion on top.",
                        drill: "Program a stiff drum machine beat (Linndrum style). Play a loose, funky shaker or tambourine loop over it.",
                        duration: 20
                    }
                ]
            }
        ]
    },
    {
        id: "prod_vol3",
        title: "Volume III: The Sound Architects",
        subtitle: "Space, Texture, and Atmosphere",
        musicians: [
            {
                id: "quincyjones",
                name: "Quincy Jones",
                archetype: "The Master",
                quote: "The song is the power; the singer is the messenger. The greatest singer in the world cannot save a bad song.",
                lessons: [
                    {
                        id: "quincy-1",
                        title: "The Material First",
                        theory: "A producer's biggest job is to get the right material. If you don't have a great song, it doesn't matter what else you put around it.",
                        drill: "Before adding any production, play your song on just piano and voice. If it doesn't work there, start over.",
                        duration: 20
                    },
                    {
                        id: "quincy-2", 
                        title: "Loving Honesty",
                        theory: "You have to love them to honestly evaluate them. This is a very close relationship between producer and artist.",
                        drill: "Record 3 takes of the same vocal line. Pick the best one based on emotion, not technical perfection.",
                        duration: 15
                    }
                ]
            },
            {
                id: "daniellanois",
                name: "Daniel Lanois",
                archetype: "The Ambient Architect", 
                quote: "All kinds of curious automatic placement of instruments happen naturally when you open a lot of mics in a room.",
                lessons: [
                    {
                        id: "lanois-1",
                        title: "Room as Instrument",
                        theory: "The studio is an instrument, not a technical facility. Spill is part of the formula, not a problem to solve.",
                        drill: "Set up multiple mics in a room. Record a simple guitar chord and embrace the natural reverb and reflections.",
                        duration: 25
                    },
                    {
                        id: "lanois-2",
                        title: "Live Manipulation",
                        theory: "Take a feed off instruments and start sampling and manipulating the sound in real-time as part of the performance.",
                        drill: "Route your guitar through a delay pedal, then sample the delayed signal and play it back while still playing.",
                        duration: 30
                    }
                ]
            }
        ]
    },
    {
        id: "prod_vol4",
        title: "Volume IV: The Feel Masters", 
        subtitle: "Groove, Vibe, and Human Touch",
        musicians: [
            {
                id: "pharrell",
                name: "Pharrell Williams",
                archetype: "The Minimalist",
                quote: "The feeling directs all creativity. The beat comes first. My job is just to listen to it, and let it tell me what should be fed.",
                lessons: [
                    {
                        id: "pharrell-1",
                        title: "Skeleton Songs",
                        theory: "The least amount of sounds we could use, the better. Create tracks that are more than the sum of their parts.",
                        drill: "Make a full song using only 4 elements: kick, snare, one melodic element, and vocals. Nothing else.",
                        duration: 45
                    },
                    {
                        id: "pharrell-2",
                        title: "The Four-Count Start",
                        theory: "Every song needs a signature intro that immediately grabs attention. The hook is everything.",
                        drill: "Create a 4-beat intro that instantly identifies your song. Test it: would someone know this track from just those 4 beats?",
                        duration: 20
                    }
                ]
            },
            {
                id: "jackwhite",
                name: "Jack White", 
                archetype: "The Analog Warrior",
                quote: "There are no safety nets. It's obviously harder to work with a 1964 plastic guitar, but that's the point.",
                lessons: [
                    {
                        id: "white-1",
                        title: "Limitations Breed Creativity",
                        theory: "Using old, imperfect gear forces you to be creative. The struggle makes the music more authentic.",
                        drill: "Record a song using only the most basic, low-fi gear you have. No perfect takes, no fixing in post.",
                        duration: 35
                    },
                    {
                        id: "white-2", 
                        title: "Sound Like The Streets",
                        theory: "Make your music sound like your environment. If you're from the city, it should sound like metal clanging.",
                        drill: "Go outside and record 30 seconds of ambient sound from your neighborhood. Use it as the foundation for a track.",
                        duration: 25
                    }
                ]
            }
        ]
    },
    {
        id: "prod_vol6",
        title: "Volume VI: The Sound Designers",
        subtitle: 'Synthesis, Glitch, and "The Growl"',
        musicians: [
            {
                id: "skrillex",
                name: "Skrillex",
                archetype: "The FM Alchemist",
                theme: "purple",
                quote: "I treat sounds like characters. They have to speak, scream, and breathe.",
                origin: "Revolutionized bass music with FM synthesis and vocal-like growls.",
                lessons: [
                    {
                        id: "skrillex-1",
                        title: 'The "Talking" Bass',
                        theoryLabel: "The Concept",
                        theory: "Use formant filters to make synths mimic human vowels: A, E, I, O, U.",
                        drillLabel: "The Drill",
                        drill: "Map a formant filter cutoff to an LFO on your bass track to create vowel-like movement.",
                        duration: 15
                    },
                    {
                        id: "skrillex-2",
                        title: "Resampling to Audio",
                        theoryLabel: "The Technique",
                        theory: "Record your synth tweaking to raw audio, then chop up the performance.",
                        drillLabel: "The Edit",
                        drill: "Record a wild synth jam to audio. Find the best split-second slice and use that instead of keeping the MIDI.",
                        duration: 15
                    }
                ]
            },
            {
                id: "flume",
                name: "Flume",
                archetype: "The Granular Architect",
                theme: "pink",
                quote: "I got bored of standard drums. I wanted to make beats out of glass and metal.",
                origin: "Known for granular synthesis and future-bass textures.",
                lessons: [
                    {
                        id: "flume-1",
                        title: "Granular Synthesis",
                        theoryLabel: "The Concept",
                        theory: "Break samples into tiny grains to create clouds of sound.",
                        drillLabel: "The Drill",
                        drill: "Put a vocal sample in a granular sampler. Freeze one moment to create a metallic synth pad.",
                        duration: 15
                    },
                    {
                        id: "flume-2",
                        title: "Silence as Heavy Drop",
                        theoryLabel: "The Arrange",
                        theory: "Heaviness comes from contrast. Use absolute silence right before the bass hits.",
                        drillLabel: "The Lesson",
                        drill: "Insert a bar of silence before your chorus drop.",
                        duration: 5
                    }
                ]
            },
            {
                id: "sophie",
                name: "SOPHIE",
                archetype: "The Materialist",
                theme: "cyan",
                quote: "I try to make sounds that resemble materials: latex, metal, bubbles.",
                origin: "Pioneer of Hyperpop and physical modeling synthesis.",
                lessons: [
                    {
                        id: "sophie-1",
                        title: "Physical Modeling",
                        theoryLabel: "The Concept",
                        theory: "Use synths that mathematically model how objects vibrate, like metal or glass.",
                        drillLabel: "The Drill",
                        drill: "Create a percussion sound that mimics water using pitch envelopes and sine waves.",
                        duration: 15
                    },
                    {
                        id: "sophie-2",
                        title: "The Hyper-Real Transient",
                        theoryLabel: "The Technique",
                        theory: "Use a transient shaper to maximize attack and reduce sustain for snapping sounds.",
                        drillLabel: "The Drill",
                        drill: "Make a snare sound impossibly short, measured in milliseconds, but still punchy.",
                        duration: 10
                    }
                ]
            }
        ]
    },
    {
        id: "prod_vol8",
        title: "Volume VIII: The Vocal Architects",
        subtitle: "Tuning, Stacking, and Booth Psychology",
        musicians: [
            {
                id: "lange",
                name: "Mutt Lange",
                archetype: "The Stack Master",
                theme: "red",
                quote: "The background vocals should sound like a synthesizer made of human voices.",
                origin: "Producer for Def Leppard and Shania Twain, known for massive vocal stacks.",
                lessons: [
                    {
                        id: "lange-1",
                        title: 'The "Mutt" Stack',
                        theoryLabel: "The Recipe",
                        theory: "Record two melody doubles, two high harmonies, two low harmonies, and two whisper layers.",
                        drillLabel: "The Result",
                        drill: "Record a chorus with at least eight layers of vocals. Align the timing until it feels like one instrument.",
                        duration: 20
                    },
                    {
                        id: "lange-2",
                        title: "EQing the Stack",
                        theoryLabel: "The Mix",
                        theory: "Backing vocals do not need body. They should support the lead without clouding it.",
                        drillLabel: "The Drill",
                        drill: "High-pass every backing vocal up toward 500Hz so the stack floats above the lead.",
                        duration: 5
                    }
                ]
            },
            {
                id: "eilish",
                name: "Billie Eilish (Finneas)",
                archetype: "The ASMR Producer",
                theme: "green",
                quote: "I want the vocal to sound like she is sitting next to you on the bed.",
                origin: "Known for intimate, dry, whisper-quiet vocals.",
                lessons: [
                    {
                        id: "eilish-1",
                        title: "The Dry Vocal",
                        theoryLabel: "The Technique",
                        theory: "Record in a dead room. Use no reverb. Let compression bring the quiet detail forward.",
                        drillLabel: "The Mix",
                        drill: "Remove all reverb from a vocal and compress it aggressively. Balance level until the intimacy feels intentional.",
                        duration: 10
                    },
                    {
                        id: "eilish-2",
                        title: "Comping Breaths",
                        theoryLabel: "The Edit",
                        theory: "Breaths are emotional information. Do not automatically delete them.",
                        drillLabel: "The Drill",
                        drill: "Cut vocal breaths to a separate track and make selected breaths louder for emotional emphasis.",
                        duration: 10
                    }
                ]
            },
            {
                id: "harrell",
                name: "Kuk Harrell",
                archetype: "The Vocal Coach",
                theme: "blue",
                quote: "It is not about pitch. It is about attitude.",
                origin: "Vocal producer for Rihanna and Beyonce.",
                lessons: [
                    {
                        id: "harrell-1",
                        title: 'The "Money" Take',
                        theoryLabel: "The Psychology",
                        theory: "Do not sing the whole song when one phrase is weak. Loop the phrase and build muscle memory.",
                        drillLabel: "The Method",
                        drill: "Loop a difficult phrase. Sing it repeatedly until the rhythm, vowel shape, and attitude lock in.",
                        duration: 15
                    },
                    {
                        id: "harrell-2",
                        title: "Melodyne as Instrument",
                        theoryLabel: "The Tool",
                        theory: "Use pitch editing to fix timing and note length, not just tuning.",
                        drillLabel: "The Drill",
                        drill: "Tighten the rhythm of a vocal performance as if it were a drum track.",
                        duration: 10
                    }
                ]
            }
        ]
    },
    {
        id: "prod_vol9",
        title: "Volume IX: The Workflow Warriors",
        subtitle: "Speed, Templates, and Executive Decision",
        musicians: [
            {
                id: "kennybeats",
                name: "Kenny Beats",
                archetype: "The Sprinter",
                theme: "orange",
                quote: "Don't overthink it. If you spend 20 minutes on a hi-hat, you've lost the vibe.",
                origin: "Modern hip-hop producer known for speed and The Cave.",
                lessons: [
                    {
                        id: "kennybeats-1",
                        title: "The 10-Minute Timer",
                        theoryLabel: "The Challenge",
                        theory: "Creativity is a faucet. You have to run the brown water to get clear water.",
                        drillLabel: "The Rules",
                        drill: "Set a timer for 10 minutes. Make a beat using loops or presets. Stop when time is up.",
                        duration: 10
                    },
                    {
                        id: "kennybeats-2",
                        title: '"Type Beat" Psychology',
                        theoryLabel: "The Strategy",
                        theory: "Limit choices before you begin. A clear style target narrows the sound palette.",
                        drillLabel: "The Drill",
                        drill: "Pick a specific style before starting. Use only sounds that fit that style.",
                        duration: 5
                    }
                ]
            },
            {
                id: "kanye",
                name: "Kanye West",
                archetype: "The Executive",
                theme: "stone",
                quote: "I'm not a rapper; I'm a curator. I put the pieces together.",
                origin: "Hip-hop producer known for sampling and curation.",
                lessons: [
                    {
                        id: "kanye-1",
                        title: "Sampling the Room",
                        theoryLabel: "The Method",
                        theory: "Layer imperfect human sounds over digital drums to make a track feel social and alive.",
                        drillLabel: "The Texture",
                        drill: "Record friends clapping or humming. Layer it over your beat for a choir-like human texture.",
                        duration: 10
                    },
                    {
                        id: "kanye-2",
                        title: "Voice Memos as Samples",
                        theoryLabel: "The Source",
                        theory: "Phone microphones add natural grit and urgency.",
                        drillLabel: "The Drill",
                        drill: "Record an idea on your phone. Import it and use it as the main sample source.",
                        duration: 10
                    }
                ]
            },
            {
                id: "grimes",
                name: "Grimes",
                archetype: "The DIY Icon",
                theme: "pink",
                quote: "Limitations force you to be creative.",
                origin: "Produced acclaimed albums in her bedroom.",
                lessons: [
                    {
                        id: "grimes-1",
                        title: "Learn One Synth Perfectly",
                        theoryLabel: "The Philosophy",
                        theory: "Do not collect 50 plugins. Master one tool deeply enough to make it do many jobs.",
                        drillLabel: "The Drill",
                        drill: "Make a kick, bass, pad, and lead using only one synth plugin.",
                        duration: 20
                    },
                    {
                        id: "grimes-2",
                        title: '"Bedroom" Acoustics',
                        theoryLabel: "The Reality",
                        theory: "Performance matters more than fidelity when the idea is strong.",
                        drillLabel: "The Drill",
                        drill: "Record vocals under a blanket. Embrace the lo-fi texture instead of hiding it.",
                        duration: 10
                    }
                ]
            }
        ]
    },
    {
        id: "prod_vol10",
        title: "Volume X: The Science Lab",
        subtitle: "Mastering, Acoustics, and Physics",
        musicians: [
            {
                id: "mastering",
                name: "Mastering Engineers",
                archetype: "The Scientists",
                theme: "slate",
                quote: "Loudness is the difference between the peak and the RMS.",
                origin: "Techniques from mastering engineers such as Bob Ludwig and Emily Lazar.",
                lessons: [
                    {
                        id: "mastering-1",
                        title: 'The "Crest Factor"',
                        theoryLabel: "The Concept",
                        theory: "To get a loud track, control short peaks before the master limiter has to react.",
                        drillLabel: "The Drill",
                        drill: "Use a clipper on your snare track to shave off 2dB of peaks before the master bus.",
                        duration: 10
                    },
                    {
                        id: "mastering-2",
                        title: "Mono Compatibility",
                        theoryLabel: "The Science",
                        theory: "If left and right channels are out of phase, important sounds can vanish in mono playback.",
                        drillLabel: "The Check",
                        drill: "Check your mix with a correlation meter. Adjust stereo effects until the reading stays positive.",
                        duration: 5
                    }
                ]
            },
            {
                id: "acoustics",
                name: "Acoustic Scientists",
                archetype: "The Physicists",
                theme: "stone",
                quote: "The room lies to you.",
                origin: "A practical view of how sound behaves in physical spaces.",
                lessons: [
                    {
                        id: "acoustics-1",
                        title: 'The "Bass Trap" Myth',
                        theoryLabel: "The Truth",
                        theory: "Egg cartons and thin foam do not stop bass. Low frequencies pass through lightweight treatment.",
                        drillLabel: "The Fix",
                        drill: "Mix at low volume to reduce room influence when you do not have proper treatment.",
                        duration: 5
                    },
                    {
                        id: "acoustics-2",
                        title: 'The "Car Test" Physics',
                        theoryLabel: "The Why",
                        theory: "Cars exaggerate bass and treble, which makes translation problems obvious.",
                        drillLabel: "The Drill",
                        drill: "Check your mix in a car. If vocals hurt, cut 2k-4kHz. If doors rattle, reduce sub bass.",
                        duration: 10
                    }
                ]
            }
        ]
    }
];

export const GUITAR_VOLUMES = [
    {
        id: "legacy_musician_vol1",
        title: "Volume I: The Masters of Self-Invention",
        subtitle: "The Foundations: Rock, Jazz, and the Oral Tradition",
        musicians: [
            {
                id: "clapton",
                name: "Eric Clapton",
                archetype: "The Disciple",
                theme: "amber",
                quote: "I would practice blues chords for hours, noting weak spots until the performance was perfect.",
                origin: "Clapton treated guitar as a vocal substitute, learning to breathe with his hands and make phrases feel sung instead of typed.",
                lessons: [
                    {
                        id: "clapton-1",
                        title: 'The "Woman Tone" (Sonic Engineering)',
                        theoryLabel: "The Setup",
                        theory: "Cream-era sustain came from a thick, rounded tone that mimicked a cello or voice.",
                        drillLabel: "The Hack",
                        drill: "Turn the guitar tone knob all the way down on the neck pickup. Raise the amp volume to compensate and practice legato lines.",
                        duration: 5
                    },
                    {
                        id: "clapton-2",
                        title: 'The "Vocal" Breath (Phrasing)',
                        theoryLabel: "The Concept",
                        theory: "Singers have to stop to breathe. Instrumentalists need to create that space on purpose.",
                        drillLabel: "The Practice",
                        drill: 'Play a short lick, say "answer me" out loud, then play a responding lick. No breath means the phrase does not count.',
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "Tape Loop" Critic',
                    description: "Record yourself improvising for 2 minutes. Listen back immediately. Identify one specific weakness. Record the same 2 minutes again, fixing only that one thing."
                }
            },
            {
                id: "hendrix",
                name: "Jimi Hendrix",
                archetype: "The Voodoo Child",
                theme: "purple",
                quote: "I could remember notes I heard on the radio and find them on the guitar.",
                origin: "Hendrix learned by watching and listening. In trio settings he had to sound like rhythm and lead at the same time, merging chord movement with melodic fills.",
                lessons: [
                    {
                        id: "hendrix-1",
                        title: "The Thumb-Over Grip",
                        theoryLabel: "The Mechanics",
                        theory: "Wrap your thumb over the top of the neck to fret the root on the low E string.",
                        drillLabel: "The Freedom",
                        drill: "Hold a bass note with your thumb while your fingers add hammer-ons and pull-offs on the high strings.",
                        duration: 5
                    },
                    {
                        id: "hendrix-2",
                        title: "Double Stops",
                        theoryLabel: "The Texture",
                        theory: "Two adjacent strings played together can turn a single-line lick into an R&B horn-like phrase.",
                        drillLabel: "The Riff",
                        drill: "In A minor pentatonic at the 5th fret, flatten your index across B and E, then hammer the G string at the 7th fret.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "Radio" Memory',
                    description: "Turn on a random playlist, pause a melody, and try to find it on your instrument within three tries."
                }
            },
            {
                id: "page",
                name: "Jimmy Page",
                archetype: "The Session Master",
                theme: "zinc",
                quote: "I slowed records down to figure out what they were doing.",
                origin: "Before Zeppelin, Page treated records like blueprints, analyzing riff construction, studio layers, timing, and dynamics.",
                lessons: [
                    {
                        id: "page-1",
                        title: "Rhythmic Displacement",
                        theoryLabel: "The Groove",
                        theory: "A simple blues lick can gain swagger when it starts on an offbeat instead of beat 1.",
                        drillLabel: "The Shift",
                        drill: "Take one pentatonic lick. Play it on beat 1, then force the exact same lick to start on the and of beat 1.",
                        duration: 10
                    },
                    {
                        id: "page-2",
                        title: "Light and Shade",
                        theoryLabel: "The Contrast",
                        theory: "Heavy music sounds heavier when it is contrasted with something quiet.",
                        drillLabel: "The Application",
                        drill: "Never play at 10 for the whole song. Use a clean verse, volume swell, or sparse texture before the heavy section.",
                        duration: 5
                    }
                ],
                artistChallenge: {
                    title: 'The "33 RPM" Microscope',
                    description: 'Find a fast riff you love. Slow it to 0.5x. Learn not just the notes, but the micro-timing and articulation of each note.'
                }
            },
            {
                id: "scofield",
                name: "John Scofield",
                archetype: "The Modernist",
                theme: "teal",
                quote: "If you play a wrong note, it is only wrong if you do not resolve it.",
                origin: "Scofield bridged funk and intellectual jazz by turning outside notes into tension that slips back into safety.",
                lessons: [
                    {
                        id: "scofield-1",
                        title: '"Outside" Playing',
                        theoryLabel: "The Tension",
                        theory: "You can approach a target note or phrase from a half-step above or below.",
                        drillLabel: "The Resolution",
                        drill: "Solo in A minor, repeat one lick in Bb minor for tension, then repeat it in A minor for resolution.",
                        duration: 15
                    },
                    {
                        id: "scofield-2",
                        title: "Legato Phrasing",
                        theoryLabel: "The Articulation",
                        theory: "Avoid picking every note so the line sounds less like a guitar exercise.",
                        drillLabel: "The Flow",
                        drill: "Pick only the first note on each string, then use hammer-ons and pull-offs for the rest.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "Wrong Note" Resolution',
                    description: 'Play a backing track. Intentionally play a wrong note outside the scale. Hold it, then slide up or down by one fret to make it right.'
                }
            },
            {
                id: "liebman",
                name: "David Liebman",
                archetype: "The Sage",
                theme: "indigo",
                quote: "Technique is the ability to do what you want to do, when you want to do it.",
                origin: "A saxophone educator who framed practice around tone, technique, spirit, and deep listening.",
                lessons: [
                    {
                        id: "liebman-1",
                        title: "The Drone",
                        theoryLabel: "The Reference",
                        theory: "Deep listening needs a reference point. You cannot tune if you do not know what you are tuning against.",
                        drillLabel: "The Ear Training",
                        drill: "Play a continuous low drone. Move through a slow scale and notice how each interval feels against the bass note.",
                        duration: 20
                    },
                    {
                        id: "liebman-2",
                        title: "Chromatic Enclosure",
                        theoryLabel: "The Approach",
                        theory: "Surrounding a target note creates gravity before the landing.",
                        drillLabel: "The Target",
                        drill: "To land on C, play Db, then B, then C. Move that upper-neighbor, lower-neighbor, target shape through a scale.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "Singing" Drone',
                    description: "Play a drone note. Sing a specific interval above it. Only after you sing it and lock the pitch, play the note on your instrument to verify."
                }
            },
            {
                id: "vai",
                name: "Steve Vai",
                archetype: "The Scientist",
                theme: "lime",
                quote: "For me it was every waking moment.",
                origin: "Vai is the patron saint of the grid: any physical motion can be mastered if it is slowed down, measured, and repeated deliberately.",
                lessons: [
                    {
                        id: "vai-1",
                        title: 'The "Finger Independence" Grid',
                        theoryLabel: "The Physics",
                        theory: "Awkward movements become smooth when you remove the hand's bias toward easy finger combinations.",
                        drillLabel: "The Permutation",
                        drill: "Play 1-2-3-4, then 1-3-2-4, then 4-1-3-2 on every string without stopping.",
                        duration: 15
                    },
                    {
                        id: "vai-2",
                        title: "Audiation",
                        theoryLabel: "The Mind",
                        theory: "If you cannot hear it internally, your hands are guessing.",
                        drillLabel: "The Visualization",
                        drill: "Lie down, visualize the fretboard, and hear each pitch in your mind before imagining the finger moving.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: "The 10-Hour Mental Grid",
                    description: "Set a timer for 5 minutes. Close your eyes. Visualize your hardest song perfectly. If your mind wanders, restart the timer."
                }
            },
            {
                id: "django",
                name: "Django Reinhardt",
                archetype: "The Survivor",
                theme: "red",
                quote: "Practice implies that you are struggling with something. I just play.",
                origin: "After a fire damaged his hand, Django reinvented fretboard geometry around vertical movement and a new jazz vocabulary.",
                lessons: [
                    {
                        id: "django-1",
                        title: "Vertical Arpeggios",
                        theoryLabel: "The Constraint",
                        theory: "Constraint breeds creativity. A physical limitation can reveal a new sound.",
                        drillLabel: "The Slide",
                        drill: "Play arpeggios up and down the neck on only the top two strings to force rapid position shifts.",
                        duration: 10
                    },
                    {
                        id: "django-2",
                        title: "The Rest Stroke",
                        theoryLabel: "The Attack",
                        theory: "Acoustic projection needs maximum energy transfer through the string.",
                        drillLabel: "The Technique",
                        drill: "Pick through the string so the pick comes to rest on the next string below it.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "Two-Finger" Constraint',
                    description: "Tape your ring and pinky fingers together, or avoid using them. Play a favorite song with only index and middle fingers."
                }
            },
            {
                id: "bbking",
                name: "B.B. King",
                archetype: "The Minimalist",
                theme: "blue",
                quote: "Notes are expensive. Spend them wisely.",
                origin: "B.B. King proved that one note with intention is worth more than a thousand notes without it.",
                lessons: [
                    {
                        id: "bbking-1",
                        title: 'The "B.B. Box"',
                        theoryLabel: "The Sweet Spot",
                        theory: "Major pentatonic notes around the B string root create a singing blues quality.",
                        drillLabel: "The Shape",
                        drill: "In A, use high E 10th fret, B 12th fret, and B 10th fret. Make each note vocal before adding more.",
                        duration: 10
                    },
                    {
                        id: "bbking-2",
                        title: "The Butterfly Vibrato",
                        theoryLabel: "The Touch",
                        theory: "The hand must be free. A fast, narrow shake can speak more than a wide rock bend.",
                        drillLabel: "The Motion",
                        drill: "Fret a note with your index finger, remove your thumb from the back of the neck, and shake the whole hand rapidly.",
                        duration: 5
                    }
                ],
                artistChallenge: {
                    title: 'The "One Note" Solo',
                    description: "Put on a 12-bar blues backing track. Solo using only the root note, creating interest through rhythm, dynamics, and articulation."
                }
            }
        ]
    },
    {
        id: "legacy_musician_vol2",
        title: "Volume II: The Architects of Sound",
        subtitle: "Groove, Texture, and Physics",
        musicians: [
            {
                id: "nile",
                name: "Nile Rodgers",
                archetype: "The Hitmaker",
                theme: "pink",
                quote: "It's not about how many notes you play, it's about how many notes you don't let ring.",
                origin: "The master of funk guitar who turned rhythm playing into a lead instrument.",
                lessons: [
                    {
                        id: "nile-1",
                        title: 'The "Chuck" (Percussive Muting)',
                        theoryLabel: "The Groove",
                        theory: "Constant right-hand motion creates the groove before any chord is fretted.",
                        drillLabel: "The Motion",
                        drill: "Keep your right hand moving in 16th notes. Squeeze the chord only for accents, then relax instantly.",
                        duration: 10
                    },
                    {
                        id: "nile-2",
                        title: "Triad Inversions",
                        theoryLabel: "The Mix",
                        theory: "Big chords clutter the band. Small triads leave room for bass, keys, and vocal.",
                        drillLabel: "The Shape",
                        drill: "Use only the G, B, and E strings. Take a D major chord and find three different places to play it up the neck.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "16th Note" Marathon',
                    description: "Set a metronome to 100 BPM. Mute the strings with your fretting hand. Strum continuous 16th notes for 3 minutes without stopping or losing the groove."
                }
            },
            {
                id: "keith",
                name: "Keith Richards",
                archetype: "The Human Riff",
                theme: "stone",
                quote: "Five strings, three fingers, two notes, one attitude.",
                origin: "Keith Richards redefined rock rhythm by removing clutter until the riff had only the parts that mattered.",
                lessons: [
                    {
                        id: "keith-1",
                        title: "Open G Tuning",
                        theoryLabel: "The Tuning",
                        theory: "Removing the low E string creates a cleaner drone and lets one finger command major chords.",
                        drillLabel: "The Setup",
                        drill: "Tune to D-G-D-G-B-D. Avoid the low E string. Play major chords with one-finger barres and add small suspended-note moves.",
                        duration: 10
                    },
                    {
                        id: "keith-2",
                        title: 'The "Weaving" Art',
                        theoryLabel: "The Band",
                        theory: "Two guitars should interlock rather than duplicate the same part.",
                        drillLabel: "The Role",
                        drill: "If a partner plays low open-position chords, you must play high inversions or capo shapes.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "Drone" Riff',
                    description: "Create a riff where one string rings open continuously while you change notes on the other strings."
                }
            },
            {
                id: "vanhalen",
                name: "Eddie Van Halen",
                archetype: "The Innovator",
                theme: "rose",
                quote: "I just heard it in my head and ran my fingers until it matched.",
                origin: "A tinkerer who built his own guitars and reinvented the vocabulary of the instrument.",
                lessons: [
                    {
                        id: "vh-1",
                        title: "Two-Handed Tapping",
                        theoryLabel: "The Transfer",
                        theory: "Tapping moves a piano-like hammer motion onto the guitar neck.",
                        drillLabel: "The Pattern",
                        drill: "Tap a note with the right hand, pull off to the left index finger, then hammer a second left-hand note.",
                        duration: 15
                    },
                    {
                        id: "vh-2",
                        title: 'The "Brown Sound"',
                        theoryLabel: "The Feel",
                        theory: "A warmer, spongier distortion responds like a living amp instead of a rigid machine.",
                        drillLabel: "The Sim",
                        drill: "On a modeler, reduce sag or voltage-style stiffness until the attack feels softer under your fingers.",
                        duration: 5
                    }
                ],
                artistChallenge: {
                    title: 'The "Hummingbird" Pick',
                    description: "Tremolo pick a single note as fast as physically possible for 60 seconds. Focus on relaxing the wrist, not tensing it."
                }
            },
            {
                id: "holdsworth",
                name: "Allan Holdsworth",
                archetype: "The Alien",
                theme: "sky",
                quote: "I wanted to play the saxophone, but I couldn't afford one.",
                origin: "Holdsworth approached guitar as a breath instrument, chasing a fluid line that ignored typical fretboard limits.",
                lessons: [
                    {
                        id: "holdsworth-1",
                        title: "Four-Note-Per-String Scales",
                        theoryLabel: "The Reach",
                        theory: "Wider fingerings create fluid lines and unexpected interval movement.",
                        drillLabel: "The Stretch",
                        drill: "Play four notes per string instead of three. Move slowly and keep the thumb relaxed behind the neck.",
                        duration: 20
                    },
                    {
                        id: "holdsworth-2",
                        title: "Volume Swells",
                        theoryLabel: "The Attack",
                        theory: "Removing the pick attack makes the guitar behave more like a violin, synth, or horn.",
                        drillLabel: "The Motion",
                        drill: "Pick a note with volume at zero, then swell up after the attack so the note blooms in.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "No Pick" Solo',
                    description: "Play a solo using only hammer-ons and pull-offs. If you must pick, pick softly. Aim for a saxophone-like fluid sound."
                }
            }
        ]
    },
    {
        id: "legacy_musician_vol3",
        title: "Volume III: The Wind & The Keys",
        subtitle: "Learning from Non-Guitarists",
        musicians: [
            {
                id: "coltrane",
                name: "John Coltrane",
                archetype: "The High Priest",
                theme: "emerald",
                quote: "I start in the middle of a sentence and move both directions at once.",
                origin: "The saxophonist who pushed harmony to its absolute limit and turned arpeggios into a moving wall of sound.",
                lessons: [
                    {
                        id: "coltrane-1",
                        title: '"Sheets of Sound"',
                        theoryLabel: "The Harmony",
                        theory: "Superimposing upper arpeggios lets one chord imply richer extensions.",
                        drillLabel: "The Layer",
                        drill: "Over C major, play an E minor 7 arpeggio: E, G, B, D. Hear it as 3, 5, 7, and 9 against C.",
                        duration: 20
                    },
                    {
                        id: "coltrane-2",
                        title: 'The "3-on-1" Practice',
                        theoryLabel: "The Search",
                        theory: "One target note can be approached from many angles before it resolves.",
                        drillLabel: "The Landing",
                        drill: "Choose one target note. Find every way to approach it from above and below before landing cleanly.",
                        duration: 15
                    }
                ],
                artistChallenge: {
                    title: 'The "Giant Steps" Cycle',
                    description: "Play a major 7 arpeggio. Move it up by a major 3rd. Play it again. Move up another major 3rd. Repeat until you return to the start."
                }
            },
            {
                id: "davis",
                name: "Miles Davis",
                archetype: "The Prince of Darkness",
                theme: "cyan",
                quote: "Don't play what's there; play what's not there.",
                origin: "The master of space and cool, making restraint feel more magnetic than density.",
                lessons: [
                    {
                        id: "davis-1",
                        title: 'The "Eggshells" Dynamics',
                        theoryLabel: "The Touch",
                        theory: "Restrained power draws listeners in. Quiet playing can feel more intense than force.",
                        drillLabel: "The Volume",
                        drill: "Set your amp or instrument loud, then play incredibly lightly. Keep the tone full while the attack stays soft.",
                        duration: 10
                    },
                    {
                        id: "davis-2",
                        title: 'The "Cool" Note Choice',
                        theoryLabel: "The Mood",
                        theory: "Ambiguous endings keep a phrase floating instead of fully resolving.",
                        drillLabel: "The Landing",
                        drill: "End phrases on the 9th. In C major, land on D and let the unresolved color hang.",
                        duration: 5
                    }
                ],
                artistChallenge: {
                    title: 'The "Negative Space" Solo',
                    description: "Play a phrase. Then rest for a duration equal to the length of that phrase before playing again."
                }
            },
            {
                id: "monk",
                name: "Thelonious Monk",
                archetype: "The Maverick",
                theme: "slate",
                quote: "The piano ain't got no wrong notes.",
                origin: "Monk used dissonance, silence, and percussive attacks to make the piano speak with a crooked grin.",
                lessons: [
                    {
                        id: "monk-1",
                        title: "The Whole Tone Scale",
                        theoryLabel: "The Gravity",
                        theory: "A scale made only of whole steps has no normal tonal gravity.",
                        drillLabel: "The Color",
                        drill: "Use the whole tone scale over a dominant chord such as G7. Listen for the floating, unanchored effect.",
                        duration: 10
                    },
                    {
                        id: "monk-2",
                        title: "Rhythmic Displacement",
                        theoryLabel: "The Stumble",
                        theory: "A melody can become fresh when familiar notes land in unexpected rhythmic places.",
                        drillLabel: "The Delay",
                        drill: "Take a simple melody and delay every other note by an eighth note without losing the pulse.",
                        duration: 15
                    }
                ],
                artistChallenge: {
                    title: 'The "Elbow" Cluster',
                    description: "Play a chord, but intentionally mash two adjacent keys or frets for one of the notes to create percussive tension."
                }
            }
        ]
    },
    {
        id: "legacy_musician_vol4",
        title: "Volume IV: The Voice & The Engine",
        subtitle: "Vocals, Bass, and Drums",
        musicians: [
            {
                id: "sinatra",
                name: "Frank Sinatra",
                archetype: "The Chairman",
                theme: "orange",
                quote: "I was fascinated by how Tommy Dorsey could sneak a breath without breaking the note.",
                origin: "The master of phrasing and breath control, turning vocal lines into long arcs instead of disconnected notes.",
                lessons: [
                    {
                        id: "sinatra-1",
                        title: 'The "Underwater" Lung',
                        theoryLabel: "The Breath",
                        theory: "Breath control equals tone control. A phrase collapses when the air supply wavers.",
                        drillLabel: "The Hold",
                        drill: "Take one deep breath, play or sing a single note, and hold it for 30 seconds without wavering.",
                        duration: 5
                    },
                    {
                        id: "sinatra-2",
                        title: "The Dorsey Slur",
                        theoryLabel: "The Line",
                        theory: "Connected phrasing makes notes feel like one continuous sentence.",
                        drillLabel: "The Connection",
                        drill: "Do not play note-note. Connect the end of one note exactly into the start of the next note.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "One Breath" Verse',
                    description: "Try to play or sing an entire verse of a song in a single breath. If you run out of air, stop and work on extending your capacity."
                }
            },
            {
                id: "jamerson",
                name: "James Jamerson",
                archetype: "The Shadow",
                theme: "yellow",
                quote: "If you don't feel it, don't play it.",
                origin: "The Motown genius who played complex basslines with one finger and made movement feel inevitable.",
                lessons: [
                    {
                        id: "jamerson-1",
                        title: '"The Hook" One-Finger Technique',
                        theoryLabel: "The Limitation",
                        theory: "A physical limitation can deepen groove by forcing ghost notes, syncopation, and economy.",
                        drillLabel: "The Finger",
                        drill: "Play basslines using only your right index finger. Let ghost notes and syncopation carry the motion.",
                        duration: 15
                    },
                    {
                        id: "jamerson-2",
                        title: "Chromatic Walk-Ups",
                        theoryLabel: "The Lead-In",
                        theory: "A bassline can lead the listener into the next chord instead of jumping there.",
                        drillLabel: "The Walk",
                        drill: "Moving from G to C, play G, A, Bb, B, then C. Keep every passing note intentional.",
                        duration: 5
                    }
                ],
                artistChallenge: {
                    title: 'The "Ghost Note" Groove',
                    description: "Play a groove where you play twice as many ghost notes, or muted percussive hits, as actual pitched notes."
                }
            },
            {
                id: "bonham",
                name: "John Bonham",
                archetype: "The Heavyweight",
                theme: "zinc",
                quote: "I've always liked the drums to be the lead instrument.",
                origin: "The thunder of Led Zeppelin, built from feel, tuning, room sound, and a huge sense of time.",
                lessons: [
                    {
                        id: "bonham-1",
                        title: 'The "Lazy" Snare',
                        theoryLabel: "The Pocket",
                        theory: "A massive groove can come from tension between an eager kick and a relaxed snare.",
                        drillLabel: "The Push-Pull",
                        drill: "Place the kick slightly ahead of the beat and the snare slightly behind it. Keep the pulse steady while the feel stretches.",
                        duration: 10
                    },
                    {
                        id: "bonham-2",
                        title: "Tuning High",
                        theoryLabel: "The Cut",
                        theory: "High-tuned drums can project through dense guitars better than low, muddy drums.",
                        drillLabel: "The Check",
                        drill: "Tune a drum or drum sample higher than expected and test whether it cuts through a heavy loop.",
                        duration: 5
                    }
                ],
                artistChallenge: {
                    title: 'The "Triplet" Hand-Foot',
                    description: "Play triplets between your hands and right foot: right, left, foot. Speed it up until it sounds like rolling thunder."
                }
            },
            {
                id: "carol-kaye",
                name: "Carol Kaye",
                archetype: "The Metronome",
                theme: "amber",
                quote: "The beat is a grid. You just hang your notes on it.",
                origin: "One of the most recorded bassists in history, Carol Kaye treated every session with precision, speed, and deep time.",
                lessons: [
                    {
                        id: "kaye-1",
                        title: 'The "2 and 4" Click',
                        theoryLabel: "The Method",
                        theory: "Most players practice with clicks on every beat, but the pocket lives on the backbeat.",
                        drillLabel: "The Drill",
                        drill: "Set the metronome to half speed so the click is beats 2 and 4. Feel beat 1 yourself while playing a simple bassline or chord progression.",
                        duration: 15
                    },
                    {
                        id: "kaye-2",
                        title: "Mental Charting",
                        theoryLabel: "The Technique",
                        theory: "Scanning the form before playing frees your hands to feel the music instead of chasing the chart.",
                        drillLabel: "The Drill",
                        drill: "Read a chord chart silently before playing. Identify sections, key changes, and climax, then play it through mostly from memory.",
                        duration: 10
                    }
                ],
                artistChallenge: {
                    title: 'The "Session Player" Gauntlet',
                    description: "Play a 12-bar blues in F. You get one take. No stopping and no chart. Record it, then grade yourself on time, tone, and taste."
                }
            }
        ]
    },
    {
        id: "legacy_musician_vol5",
        title: "Volume V: The Soul of the Note",
        subtitle: "Blues, Hard Bop, and Classic Rock",
        musicians: [
            {
                id: "king-albert",
                name: "Albert King",
                archetype: "The Velvet Bulldozer",
                theme: "purple",
                quote: "I play the blues because I'm happy, I play the blues because I'm sad, but mostly I play to make you feel it.",
                origin: "A left-handed player who played a right-handed guitar upside down, creating a unique bending vocabulary.",
                lessons: [
                    {
                        id: "king-albert-1",
                        title: 'The "Over-Bend"',
                        theoryLabel: "The Signature",
                        theory: "Albert did not only bend one step. He bent one and a half or even two full steps for vocal intensity.",
                        drillLabel: "The Drill",
                        drill: "Play a note on the B string. Bend until it matches the pitch of the note three frets higher.",
                        duration: 15
                    },
                    {
                        id: "king-albert-2",
                        title: 'The "Squeeze"',
                        theoryLabel: "The Technique",
                        theory: "Do not hit the note dead-on. Squeeze it slightly sharp right after the attack.",
                        drillLabel: "The Drill",
                        drill: "Practice hitting a note and immediately squeezing it a quarter tone sharp for expression.",
                        duration: 5
                    }
                ]
            },
            {
                id: "king-freddie",
                name: "Freddie King",
                archetype: "The Texas Cannonball",
                theme: "red",
                quote: "You gotta have that drive. If you lose the drive, you lose the people.",
                origin: "A powerhouse bluesman known for energetic stage presence and hybrid picking bite.",
                lessons: [
                    {
                        id: "king-freddie-1",
                        title: 'The "Hybrid" Attack',
                        theoryLabel: "The Technique",
                        theory: "Use a pick for bass-string downstrokes and fingers to snap treble strings upward.",
                        drillLabel: "The Drill",
                        drill: "Alternate bass downstrokes with middle or ring finger snaps on the high strings.",
                        duration: 15
                    },
                    {
                        id: "king-freddie-2",
                        title: "The Turnaround",
                        theoryLabel: "The Concept",
                        theory: "If you play the same turnaround every time, the audience stops listening.",
                        drillLabel: "The Drill",
                        drill: "Learn five distinct ways to move from the I chord back to the V chord.",
                        duration: 10
                    }
                ]
            },
            {
                id: "rollins",
                name: "Sonny Rollins",
                archetype: "The Colossus",
                theme: "yellow",
                quote: "I don't just play the chords. I play the song.",
                origin: "A saxophone colossus who treats improvisation as thematic development rather than scale running.",
                lessons: [
                    {
                        id: "rollins-1",
                        title: 'The "Motif" Challenge',
                        theoryLabel: "The Assignment",
                        theory: "Improvising on a theme gives a solo memory, structure, and identity.",
                        drillLabel: "The Drill",
                        drill: 'Play a simple three-note rhythm such as "Da-Da-DA." Improvise a 12-bar solo using only that rhythm while changing pitches.',
                        duration: 20
                    },
                    {
                        id: "rollins-2",
                        title: "Staccato Articulation",
                        theoryLabel: "The Sound",
                        theory: "Short, punchy notes create a strutting feel and make the line speak.",
                        drillLabel: "The Drill",
                        drill: "Play a major scale with extreme staccato articulation, keeping every note short and intentional.",
                        duration: 10
                    }
                ]
            },
            {
                id: "gilmour",
                name: "David Gilmour",
                archetype: "The Tone Bender",
                theme: "cyan",
                quote: "I can't play fast, so I have to make every note count.",
                origin: "The voice of Pink Floyd, built on touch, bend accuracy, space, and tone.",
                lessons: [
                    {
                        id: "gilmour-1",
                        title: 'The "Step-and-a-Half" Bend',
                        theoryLabel: "The Challenge",
                        theory: "A wide bend only works when the target pitch is exact.",
                        drillLabel: "The Drill",
                        drill: "Play a note, then target the pitch three semitones higher. Bend up to it and check against a tuner.",
                        duration: 15
                    },
                    {
                        id: "gilmour-2",
                        title: 'The "Violin" Sustain',
                        theoryLabel: "The Technique",
                        theory: "Let the note bloom before adding vibrato. Vibrato at the end feels more vocal.",
                        drillLabel: "The Drill",
                        drill: "Hold a bent note perfectly still for two seconds, then add controlled vibrato.",
                        duration: 10
                    }
                ]
            },
            {
                id: "may",
                name: "Brian May",
                archetype: "The Orchestrator",
                theme: "orange",
                quote: "I wanted the guitar to be a voice, but not just one voice, a choir.",
                origin: "The Queen guitarist who built his own guitar and used layered parts to create orchestral rock textures.",
                lessons: [
                    {
                        id: "may-1",
                        title: 'The "Sixpence" Tone',
                        theoryLabel: "The Hack",
                        theory: "A rigid metal pick or coin creates a rasping attack that cuts through a dense mix.",
                        drillLabel: "The Drill",
                        drill: "Try picking with a coin or very hard pick and listen to how the attack changes.",
                        duration: 5
                    },
                    {
                        id: "may-2",
                        title: "Harmonized Thirds",
                        theoryLabel: "The Concept",
                        theory: "A second guitar part moving in thirds can turn a melody into a choir.",
                        drillLabel: "The Drill",
                        drill: "Record a melody. Record a second track playing the same rhythm a third higher.",
                        duration: 15
                    }
                ]
            }
        ]
    },
    {
        id: "legacy_musician_vol6",
        title: "Volume VI: The Acoustic Alchemists",
        subtitle: 'Tunings, Percussion, and The "Piano" Approach',
        musicians: [
            {
                id: "mitchell",
                name: "Joni Mitchell",
                archetype: "The Painter",
                theme: "amber",
                quote: "I didn't like standard tuning. It sounded like a starved horse. I tuned the guitar to chords I heard in my head.",
                origin: "Joni Mitchell used open tunings as composition engines, turning the fretboard into a canvas of unresolved chords and questions.",
                lessons: [
                    {
                        id: "legacy-mitchell-1",
                        title: "Tuning as Composition",
                        theoryLabel: "The Concept",
                        theory: "Writing outside standard tuning breaks muscle memory and forces new harmonic discoveries.",
                        drillLabel: "The Assignment",
                        drill: "Tune to an open chord such as Open D: D-A-D-F#-A-D. Find shapes with only one or two fingers and write from what the tuning suggests.",
                        duration: 15
                    },
                    {
                        id: "legacy-mitchell-2",
                        title: 'The "Suspended" Strum',
                        theoryLabel: "The Sound",
                        theory: "Suspended chords feel like they are asking a question instead of resolving.",
                        drillLabel: "The Drill",
                        drill: "Lift the third out of your chords. If you are playing A major, lift C# to create Asus2 and keep the harmony ambiguous.",
                        duration: 10
                    }
                ]
            },
            {
                id: "hedges",
                name: "Michael Hedges",
                archetype: "The Percussionist",
                theme: "stone",
                quote: "The guitar is a drum with strings.",
                origin: "Hedges revolutionized acoustic guitar by using the body as percussion and both hands as independent sound sources.",
                lessons: [
                    {
                        id: "hedges-1",
                        title: "The Left-Hand Hammer",
                        theoryLabel: "The Technique",
                        theory: "Hammer-ons can generate basslines by themselves while the picking hand handles percussion.",
                        drillLabel: "The Drill",
                        drill: "Tap a rhythm on the guitar body with your right hand. Without strumming, hammer a low-string bass note hard enough to ring.",
                        duration: 15
                    },
                    {
                        id: "hedges-2",
                        title: "Slap Harmonics",
                        theoryLabel: "The Move",
                        theory: "A fast slap directly over the 12th fret can create a huge ringing harmonic chord.",
                        drillLabel: "The Drill",
                        drill: "Slap the strings exactly over the 12th fret wire with your middle finger and pull away instantly until the harmonic rings clear.",
                        duration: 10
                    }
                ]
            },
            {
                id: "drake",
                name: "Nick Drake",
                archetype: "The Ghost",
                theme: "slate",
                quote: "I don't think I have a style. I just play the notes I hear.",
                origin: "Nick Drake created hypnotic folk textures from complex fingerpicking, strange tunings, and a quiet, trance-like pulse.",
                lessons: [
                    {
                        id: "drake-1",
                        title: 'The "Dead" Thumb',
                        theoryLabel: "The Feel",
                        theory: "A steady thumb can create a trance when it avoids bounce and syncopation.",
                        drillLabel: "The Drill",
                        drill: "Play a fingerstyle pattern where the thumb plays strict quarter notes with no accent. It should feel like a metronome, not a melody.",
                        duration: 15
                    },
                    {
                        id: "drake-2",
                        title: "The Cluster Chord",
                        theoryLabel: "The Voicing",
                        theory: "Open tunings can let adjacent notes ring together as tense, beautiful clusters.",
                        drillLabel: "The Drill",
                        drill: "Tune to an open tuning and find adjacent notes on adjacent strings that create dissonant clusters.",
                        duration: 10
                    }
                ]
            }
        ]
    },
    {
        id: "guitar_vol1",
        title: "Volume I: Tone Architects",
        subtitle: "Sound, Touch, and Expression",
        musicians: [
            {
                id: "johnmayer",
                name: "John Mayer",
                archetype: "The Touch Master",
                quote: "The biggest influence on the sound that comes out of my guitar has everything to do with me — much more than any of my gear.",
                lessons: [
                    {
                        id: "mayer-1",
                        title: "Finger-Style Blues",
                        theory: "Use your fingers to pluck strings rather than a pick. This gives you dynamic control and a warmer, more soulful tone.",
                        drill: "Play a 12-bar blues progression using only your fingers. Focus on varying your attack strength to control the amount of drive.",
                        duration: 30
                    },
                    {
                        id: "mayer-2", 
                        title: "Thumb Fretting",
                        theory: "Use your left thumb to fret bass notes while playing chords or melody with your fingers. This opens up voicings impossible with traditional technique.",
                        drill: "Fret the 6th string with your thumb while playing a D chord. Move the bass note to create D/F# and D/A.",
                        duration: 25
                    }
                ]
            },
            {
                id: "ericjohnson",
                name: "Eric Johnson",
                archetype: "The Perfectionist", 
                quote: "Your sound comes from how you pick and dampen the strings, and from your attack, as much as anything.",
                lessons: [
                    {
                        id: "johnson-1",
                        title: "The Koto Technique",
                        theory: "Fret the note with your index finger of your right hand, then pick directly behind it. Creates a bell-like, Asian-inspired tone.",
                        drill: "Play single notes on the high E string. Fret with right index finger at 12th fret, pick behind your finger with your picking hand.",
                        duration: 20
                    },
                    {
                        id: "johnson-2",
                        title: "Stratocaster Purity", 
                        theory: "Remove string trees and use staggered tuners. String trees mess with sustain and tone. Every vibration matters.",
                        drill: "Play sustained bends on each string. Listen for any interference or dead spots. Adjust your setup to maximize natural resonance.",
                        duration: 35
                    }
                ]
            }
        ]
    },
    {
        id: "guitar_vol2",
        title: "Volume II: The Innovators",
        subtitle: "Tunings, Techniques, and Breaking Rules",
        musicians: [
            {
                id: "jonimitchell",
                name: "Joni Mitchell",
                archetype: "The Tuning Explorer",
                quote: "For me, songwriting is as much about crafting a tuning as it is about choosing the chords, melodies, or lyrics.",
                lessons: [
                    {
                        id: "mitchell-1",
                        title: "Open Tuning Freedom",
                        theory: "Use over 50 different tunings. Each tuning is a different instrument that suggests different chord voicings and melodies.",
                        drill: "Tune to DADGAD. Explore what chords are possible with just open strings and simple barre shapes.",
                        duration: 45
                    },
                    {
                        id: "mitchell-2",
                        title: "Dulcimer Percussion",
                        theory: "Combine strumming with percussive slaps. Strum across strings then dampen with the fleshy side of your hand.",
                        drill: "Play a simple chord progression while adding rhythmic slaps on beats 2 and 4. Focus on the percussive texture.",
                        duration: 25
                    }
                ]
            }
        ]
    }
];

export const VOLUMES = [
    {
        id: "vol1",
        title: "Volume I: Masters",
        musicians: [
            {
                id: "clapton",
                name: "Eric Clapton",
                lessons: [{ id: "c1", title: "Woman Tone", theory: "Roll off tone.", drill: "Tone to 0.", duration: 10 }]
            }
        ]
    }
];

export const SPARK_DATA = {
    progressions: [
        { name: "The Standard", pattern: "I - IV - V - I", chords: ["C Major", "F Major", "G Major", "C Major"] },
        { name: "Emotional Pop", pattern: "vi - IV - I - V", chords: ["A Minor", "F Major", "C Major", "G Major"] },
        { name: "Jazz Turnaround", pattern: "ii - V - I - vi", chords: ["D Minor", "G Dominant", "C Major", "A Minor"] },
        { name: "Epic Journey", pattern: "i - VII - VI - V", chords: ["A Minor", "G Major", "F Major", "E Major"] },
        { name: "Neo-Soul", pattern: "i9 - iv9 - v9", chords: ["C Minor 9", "F Minor 9", "G Minor 9"] }
    ],
    moods: [
        "Cinematic Noir", "Cyberpunk Industrial", "Lo-Fi Sunday", "Arena Rock Anthem", 
        "Underground Techno", "80s Synthwave", "Acoustic Campfire", "Video Game Boss Fight"
    ],
    constraints: [
        "No using the Root note", "16th notes only", "Use only 3 tracks", "Make the bass the lead",
        "No hi-hats allowed", "Sample your own voice", "Use only one synthesizer patch", "Tempo must be under 80 BPM"
    ]
};
