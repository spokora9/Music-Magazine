import { getChordNotes } from "./data.js";

export const LICK_TUNING = [40, 45, 50, 55, 59, 64];

export const LICK_LIBRARY = [
    {
        id: "bb-box-vibrato",
        name: "The BB Shake",
        artist: "B.B. King",
        style: "blues",
        key: "A",
        bpm: 80,
        description: "The quintessential BB King vibrato on the root note. It is all about the wrist.",
        theory: "This lick targets the root note (A) on the B string (10th fret). The magic is in the \"butterfly\" vibrato - rapid, wide, and expressive.",
        progression: [{ chord: "A7", beats: 4 }],
        sequence: [{ t: 0.0, s: 4, f: 10, d: 3.5, type: "vibrato" }],
        tab: `e|------------------|\nB|--10~~~~~~~~~~~~--|\nG|------------------|\nD|------------------|\nA|------------------|\nE|------------------|`
    },
    {
        id: "bb-sweet-bend",
        name: "The Sweet Cry",
        artist: "B.B. King",
        style: "blues",
        key: "A",
        bpm: 70,
        description: "A full step bend from the 4th to the 5th, resolving back to the root.",
        theory: "Bending the D (4th) up to E (5th) creates tension that resolves beautifully back to the root or the flat 7th.",
        progression: [{ chord: "A7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 5, f: 10, d: 0.8, type: "bend", bend: 2 },
            { t: 1.0, s: 5, f: 10, d: 0.5, type: "normal" },
            { t: 1.5, s: 5, f: 8, d: 0.5, type: "normal" },
            { t: 2.0, s: 4, f: 10, d: 2.0, type: "vibrato" }
        ],
        tab: `e|--10b12r10--8-------|\nB|----------------10~~|\nG|--------------------|\nD|--------------------|\nA|--------------------|\nE|--------------------|`
    },
    {
        id: "albert-bend",
        name: "Velvet Bulldozer",
        artist: "Albert King",
        style: "blues",
        key: "E",
        bpm: 90,
        description: "A massive 1.5 step bend. You need strong fingers for this one.",
        theory: "Albert often bent notes up a minor third (3 semitones). Here we bend the C# (6th) up to E (Root).",
        progression: [{ chord: "E7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 4, f: 14, d: 2.0, type: "bend", bend: 3 },
            { t: 2.0, s: 4, f: 12, d: 1.0, type: "normal" },
            { t: 3.0, s: 5, f: 12, d: 1.0, type: "vibrato" }
        ],
        tab: `e|----------------12~~|\nB|--14b17---12--------|\nG|--------------------|\nD|--------------------|\nA|--------------------|\nE|--------------------|`
    },
    {
        id: "srv-shuffle",
        name: "Texas Double Stops",
        artist: "SRV",
        style: "blues",
        key: "E",
        bpm: 110,
        description: "Driving double-stops with a triplet shuffle feel.",
        theory: "Playing the root (E) and 5th (B) together on the top strings creates a thick, organ-like texture.",
        progression: [{ chord: "E7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 5, f: 12, d: 0.3, type: "normal" }, { t: 0.0, s: 4, f: 12, d: 0.3, type: "normal" },
            { t: 0.33, s: 5, f: 12, d: 0.3, type: "normal" }, { t: 0.33, s: 4, f: 12, d: 0.3, type: "normal" },
            { t: 0.66, s: 5, f: 14, d: 0.3, type: "normal" }, { t: 0.66, s: 4, f: 14, d: 0.3, type: "normal" },
            { t: 1.0, s: 5, f: 12, d: 0.3, type: "normal" }, { t: 1.0, s: 4, f: 12, d: 0.3, type: "normal" },
            { t: 2.0, s: 5, f: 12, d: 2.0, type: "slide-down" }
        ],
        tab: `e|--12-12-14-12--12\\--|\nB|--12-12-14-12--12\\--|\nG|--------------------|\nD|--------------------|\nA|--------------------|\nE|--------------------|`
    },
    {
        id: "jazz-ii-V-I",
        name: "Bebop ii-V-I",
        artist: "Jazz",
        style: "jazz",
        key: "C",
        bpm: 120,
        description: "Classic bebop line utilizing enclosures and guide tones.",
        theory: "Targets the 3rd of Dm7 (F), then the 3rd of G7 (B), resolving to the 9th of Cmaj7 (D).",
        progression: [{ chord: "Dm7", beats: 2 }, { chord: "G7", beats: 2 }, { chord: "Cmaj7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 3, f: 10, d: 0.5, type: "normal" }, { t: 0.5, s: 3, f: 12, d: 0.5, type: "normal" },
            { t: 1.0, s: 3, f: 14, d: 0.5, type: "normal" }, { t: 1.5, s: 2, f: 13, d: 0.5, type: "normal" },
            { t: 2.0, s: 3, f: 12, d: 0.5, type: "normal" }, { t: 2.5, s: 3, f: 10, d: 0.5, type: "normal" },
            { t: 3.0, s: 3, f: 9, d: 0.5, type: "normal" }, { t: 3.5, s: 4, f: 12, d: 0.5, type: "normal" },
            { t: 4.0, s: 4, f: 10, d: 4.0, type: "normal" }
        ],
        tab: `e|------------------------|\nB|-----------13-12--10----|\nG|--10-12-14-----------9--|\nD|------------------------|\nA|------------------------|\nE|------------------------|`
    },
    {
        id: "clapton-crossroads",
        name: "Crossroads Turnaround",
        artist: "Clapton",
        style: "blues",
        key: "A",
        bpm: 130,
        description: "The legendary descending run from the Cream era live version of Crossroads.",
        theory: "A classic major blues turnaround. Uses the major pentatonic mixed with the minor 3rd (C) for that bluesy clash.",
        progression: [{ chord: "A7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 5, f: 8, d: 0.3, type: "bend", bend: 1 }, { t: 0.5, s: 5, f: 5, d: 0.3, type: "normal" },
            { t: 1.0, s: 4, f: 8, d: 0.3, type: "normal" }, { t: 1.5, s: 4, f: 5, d: 0.3, type: "normal" },
            { t: 2.0, s: 3, f: 7, d: 0.3, type: "normal" }, { t: 2.5, s: 3, f: 5, d: 0.3, type: "normal" },
            { t: 3.0, s: 2, f: 7, d: 1.0, type: "vibrato" }
        ],
        tab: `e|--8b9-5-----------------|\nB|--------8-5-------------|\nG|------------7-5---------|\nD|----------------7~~~~---|\nA|------------------------|\nE|------------------------|`
    },
    {
        id: "srv-mary-lamb",
        name: "Texas Nursery Rhyme",
        artist: "SRV",
        style: "rock",
        key: "E",
        bpm: 115,
        description: "The bouncy, rhythmic lead line based on \"Mary Had a Little Lamb\".",
        theory: "Uses E Minor Pentatonic with the added Major 3rd (G#). The key is the staccato, rhythmic \"bounce\" on the open low E string.",
        progression: [{ chord: "E7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 5, f: 12, d: 0.5, type: "normal" }, { t: 0.5, s: 4, f: 12, d: 0.5, type: "normal" },
            { t: 1.0, s: 3, f: 12, d: 0.25, type: "normal" }, { t: 1.25, s: 3, f: 13, d: 0.25, type: "normal" },
            { t: 1.5, s: 4, f: 12, d: 0.5, type: "normal" }, { t: 2.0, s: 5, f: 0, d: 0.5, type: "normal" },
            { t: 2.5, s: 3, f: 12, d: 0.25, type: "normal" }, { t: 2.75, s: 3, f: 14, d: 0.25, type: "normal" },
            { t: 3.0, s: 3, f: 12, d: 0.5, type: "normal" }, { t: 3.5, s: 5, f: 0, d: 0.5, type: "normal" }
        ],
        tab: `e|--12----------------|\nB|-----12-------12----|\nG|--------12h13----12-|\nD|--------------------|\nA|--------------------|\nE|----------0-------0-|`
    },
    {
        id: "peter-green-sustain",
        name: "The Green Sustain",
        artist: "Peter Green",
        style: "blues",
        key: "Dm",
        bpm: 60,
        description: "Less is more. A masterclass in touch, tone, and holding a note until it cries.",
        theory: "Peter Green often used the natural minor scale (Aeolian) over minor blues.",
        progression: [{ chord: "Dm7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 4, f: 10, d: 0.5, type: "normal" }, { t: 0.5, s: 5, f: 10, d: 1.5, type: "vibrato" },
            { t: 2.0, s: 5, f: 12, d: 0.5, type: "bend", bend: 1 }, { t: 2.5, s: 5, f: 12, d: 0.5, type: "normal" },
            { t: 3.0, s: 5, f: 10, d: 1.5, type: "normal" }, { t: 4.5, s: 4, f: 13, d: 0.0, type: "normal" }
        ],
        tab: `e|-----10~~--12b13r12--10-----|\nB|--10--------------------13--|\nG|----------------------------|\nD|----------------------------|\nA|----------------------------|\nE|----------------------------|`
    },
    {
        id: "jlh-boom-boom",
        name: "The Boom Boom Riff",
        artist: "John Lee Hooker",
        style: "blues",
        key: "E",
        bpm: 140,
        description: "The call-and-response riff that defines the Detroit Blues sound.",
        theory: "Strictly E Minor Pentatonic. The power comes from the staccato attack.",
        progression: [{ chord: "E7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 0, f: 0, d: 0.25, type: "normal" }, { t: 0.5, s: 0, f: 3, d: 0.25, type: "normal" },
            { t: 1.0, s: 1, f: 0, d: 0.25, type: "normal" }, { t: 1.5, s: 0, f: 3, d: 0.25, type: "normal" },
            { t: 2.0, s: 0, f: 0, d: 0.25, type: "normal" }, { t: 2.5, s: 0, f: 0, d: 0.25, type: "normal" },
            { t: 3.0, s: 0, f: 3, d: 0.25, type: "normal" }, { t: 3.5, s: 1, f: 0, d: 0.25, type: "normal" }
        ],
        tab: `e|--------------------|\nB|--------------------|\nG|--------------------|\nD|--------------------|\nA|-----0--------0-----|\nE|--0-3--3-0-0-3------|`
    },
    {
        id: "freddie-king-stinger",
        name: "The Freddie Stinger",
        artist: "Freddie King",
        style: "blues",
        key: "D",
        bpm: 100,
        description: "A sharp, aggressive high-register lick that cuts through any mix.",
        theory: "Freddie often played with metal fingerpicks, giving his notes a sharp attack. This lick is in D Major Pentatonic.",
        progression: [{ chord: "D7", beats: 4 }],
        sequence: [
            { t: 0.0, s: 5, f: 10, d: 0.5, type: "bend", bend: 2 },
            { t: 0.5, s: 5, f: 10, d: 0.5, type: "normal" },
            { t: 1.0, s: 4, f: 10, d: 1.0, type: "vibrato" }
        ],
        tab: `e|--10b12r10--------|\nB|------------10~~--|\nG|------------------|\nD|------------------|\nA|------------------|\nE|------------------|`
    },
    {
        id: "funk-strat-scratch",
        name: "Ghost Note Groove",
        artist: "Funk",
        style: "funk",
        key: "E",
        bpm: 105,
        description: "Percussive 16th note scratching with a tight 9th chord.",
        theory: "The essence of funk is the \"scratch\" - muted strings hit rhythmically between fretted notes.",
        progression: [{ chord: "E9", beats: 4 }],
        sequence: [
            { t: 0.0, s: 3, f: 6, d: 0.25, type: "normal" }, { t: 0.0, s: 2, f: 7, d: 0.25, type: "normal" }, { t: 0.0, s: 1, f: 7, d: 0.25, type: "normal" },
            { t: 0.25, s: 3, f: 6, d: 0.1, type: "normal" },
            { t: 0.5, s: 3, f: 6, d: 0.25, type: "normal" }, { t: 0.5, s: 2, f: 7, d: 0.25, type: "normal" }, { t: 0.5, s: 1, f: 7, d: 0.25, type: "normal" }
        ],
        tab: `e|---7---7---|\nB|---7-x-7---|\nG|---6-x-6---|\nD|-----------|\nA|-----------|\nE|-----------|`
    }
];

export function stringFretToMidi(event) {
    if (!event) return null;

    const stringIndex = Number(event.s);
    const fret = Number(event.f);

    if (!Number.isInteger(stringIndex) || stringIndex < 0 || stringIndex >= LICK_TUNING.length) {
        return null;
    }

    if (!Number.isFinite(fret)) {
        return null;
    }

    return LICK_TUNING[stringIndex] + fret;
}

export function lickToJamTrack(lick) {
    if (!lick) return null;

    return {
        id: `lick-${lick.id}`,
        title: lick.name,
        genre: `Lick Library / ${lick.style}`,
        bpm: lick.bpm,
        key: lick.key,
        source: "lick-library",
        sourceLickId: lick.id,
        progression: (lick.progression || []).map(chord => ({
            name: chord.chord,
            beats: Number(chord.beats) || 4,
            notes: getChordNotes(chord.chord),
            theory: `${lick.artist} progression`
        }))
    };
}
