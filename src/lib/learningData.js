import {
  GUITAR_VOLUMES,
  MUSICIAN_CHALLENGES,
  PRODUCER_CHALLENGES,
  PRODUCER_VOLUMES as LEGACY_PRODUCER_VOLUMES
} from "./legacyLearningData.js";

export const PRODUCER_WORKBOOK_VOLUME = {
  id: "prod_vol5",
  title: "Volume V: The Producer's Workbook",
  subtitle: "The 7-Day Studio Challenge",
  musicians: [
    {
      id: "producer_workbook",
      name: "The Producer's Workbook",
      archetype: "The Assignment Deck",
      theme: "orange",
      quote: "Finish one constrained studio assignment before changing the brief.",
      origin: "A seven-part studio workbook pulled from The Producer's Shed Volume V.",
      artistChallenge: {
        title: "Complete One Assignment",
        description: "Pick any workbook lesson, set the timer exactly as written, and render or save the result before judging it."
      },
      lessons: [
        {
          id: "producer-workbook-1",
          title: 'The "Dr. Dre" Drum Day',
          theoryLabel: "The Constraint",
          theory: "A producer can spend an entire session making the drums hit before adding melody.",
          drillLabel: "The Assignment",
          drill: "Spend one full session creating one drum loop. Layer 3 snares, EQ the kick, get it perfect, and save it as a template before adding melodies.",
          duration: 120
        },
        {
          id: "producer-workbook-2",
          title: 'The "Rick Rubin" Mute',
          theoryLabel: "The Constraint",
          theory: "A cluttered project often improves when the arrangement is reduced before it is mixed.",
          drillLabel: "The Assignment",
          drill: 'Open an old project that feels cluttered. Mute 50% of the tracks, press play, and delete the muted tracks if the song still works.',
          duration: 30
        },
        {
          id: "producer-workbook-3",
          title: 'The "Max Martin" Melody',
          theoryLabel: "The Constraint",
          theory: "The melody dictates the lyric rhythm, so syllables must lock to the line.",
          drillLabel: "The Assignment",
          drill: "Write a melody with a synth lead, record it, then write lyrics that match the rhythm exactly. Change any lyric with the wrong syllable count.",
          duration: 30
        },
        {
          id: "producer-workbook-4",
          title: 'The "Foley" Hunt',
          theoryLabel: "The Constraint",
          theory: "Everyday sounds can become a kit when the source has texture and intent.",
          drillLabel: "The Assignment",
          drill: "Walk around with a phone or recorder and capture 10 sounds. Put them in your DAW and create a beat using only those recordings.",
          duration: 45
        },
        {
          id: "producer-workbook-5",
          title: 'The "Mono" Mix',
          theoryLabel: "The Constraint",
          theory: "If every important element can be heard without panning, the mix balance is doing real work.",
          drillLabel: "The Assignment",
          drill: 'Force the master output to mono and mix the song until every part is clear. Turn stereo back on only after the mono balance works.',
          duration: 45
        },
        {
          id: "producer-workbook-6",
          title: 'The "Timer" Limit',
          theoryLabel: "The Constraint",
          theory: "Arrangement decisions should happen before sound tweaking steals the session.",
          drillLabel: "The Assignment",
          drill: "Set a 30-minute timer and arrange the full song structure with placeholder clips. Do not tweak a single sound until the structure is complete.",
          duration: 30
        },
        {
          id: "producer-workbook-7",
          title: 'The "Reference" Match',
          theoryLabel: "The Constraint",
          theory: "A level-matched reference turns vague taste into concrete mix decisions.",
          drillLabel: "The Assignment",
          drill: "Import a professional song you love, level-match it against your track, and A/B the snare brightness, bass level, and overall EQ balance.",
          duration: 30
        }
      ]
    }
  ]
};

export const PRODUCER_UTILITY_BELT_VOLUME = {
  id: "prod_utility_belt",
  title: "The Utility Belt: Cheat Sheets & Reference Charts",
  subtitle: "EQ, Compression, and Template Architecture",
  musicians: [
    {
      id: "utility_eq",
      name: "EQ Cheat Sheet",
      archetype: "The Frequency Map",
      theme: "cyan",
      quote: "Cut the mud before you chase sparkle.",
      origin: "A reference map for the frequency zones that shape a production.",
      artistChallenge: {
        title: "One EQ Move Only",
        description: "Choose one problem band, make a single deliberate EQ move, and leave the rest of the channel untouched for one pass."
      },
      lessons: [
        {
          id: "utility-eq-1",
          title: "Sub and Mud Map",
          theoryLabel: "The Reference",
          theory: "Sub lives at 20-60Hz and should be reserved for kick and bass. Low mids at 200-500Hz are where mud builds up.",
          drillLabel: "The Check",
          drill: "High-pass every non-bass element, then sweep 200-500Hz on guitars, snares, and vocals to find one muddy area to cut.",
          duration: 10
        },
        {
          id: "utility-eq-2",
          title: "Presence and Air Map",
          theoryLabel: "The Reference",
          theory: "The body of guitars and voices lives around 500Hz-2kHz, presence sits at 2kHz-4kHz, and air begins around 10kHz.",
          drillLabel: "The Check",
          drill: "A/B a vocal with a small presence move and a small air shelf. Keep the one that improves intelligibility without harshness.",
          duration: 10
        }
      ]
    },
    {
      id: "utility_compression",
      name: "Compression Ratio Guide",
      archetype: "The Dynamics Map",
      theme: "amber",
      quote: "Choose the ratio for the job before touching the threshold.",
      origin: "A quick reference for matching compression ratios to musical intent.",
      artistChallenge: {
        title: "Ratio Audit",
        description: "Open one session and name the job of every compressor. Change any ratio that does not match its job."
      },
      lessons: [
        {
          id: "utility-compression-1",
          title: "Glue and Control",
          theoryLabel: "The Reference",
          theory: "A 2:1 ratio gently glues a bus, while 4:1 gives steadier control for bass and vocals.",
          drillLabel: "The Check",
          drill: "Put 2:1 on an instrument bus for subtle movement, then compare a vocal or bass at 4:1 with the same gain reduction.",
          duration: 10
        },
        {
          id: "utility-compression-2",
          title: "Lockdown and Limiting",
          theoryLabel: "The Reference",
          theory: "Ratios from 8:1 to 10:1 lock down rap vocals or parallel drums. 20:1 limiting is a brickwall for spikes.",
          drillLabel: "The Check",
          drill: "Use heavy compression on a parallel drum bus, blend it under the dry drums, then use limiting only to catch the largest peaks.",
          duration: 10
        }
      ]
    },
    {
      id: "utility_template",
      name: "Template Architect",
      archetype: "The Session Builder",
      theme: "slate",
      quote: "Stop routing, start creating.",
      origin: "A reference setup for loading a project with routing, chains, sends, and sidechain paths ready.",
      artistChallenge: {
        title: "Template Reset",
        description: "Spend one focused pass improving the default session template instead of improving a song."
      },
      lessons: [
        {
          id: "utility-template-1",
          title: "Default Drum and Vocal Chains",
          theoryLabel: "The Reference",
          theory: "A useful template opens with a drum bus that already has clipping and compression, plus a vocal chain with tuning, EQ, compression, and de-essing.",
          drillLabel: "The Setup",
          drill: "Build a default project with a drum bus and vocal chain already loaded. Save it before writing anything new.",
          duration: 15
        },
        {
          id: "utility-template-2",
          title: "Ready FX and Ghost Kick",
          theoryLabel: "The Reference",
          theory: "Short reverb, long reverb, quarter-note delay, and a ghost kick sidechain should be ready before the session starts.",
          drillLabel: "The Setup",
          drill: "Create three FX sends and a muted ghost kick channel routed to duck the bass. Save this routing into the template.",
          duration: 15
        }
      ]
    }
  ]
};

function insertAfterVolume(volumes, afterId, insertedVolume) {
  const index = volumes.findIndex((volume) => volume.id === afterId);
  if (index === -1) {
    return [...volumes, insertedVolume];
  }

  return [
    ...volumes.slice(0, index + 1),
    insertedVolume,
    ...volumes.slice(index + 1)
  ];
}

export const PRODUCER_VOLUMES = [
  ...insertAfterVolume(LEGACY_PRODUCER_VOLUMES, "prod_vol4", PRODUCER_WORKBOOK_VOLUME),
  PRODUCER_UTILITY_BELT_VOLUME
];

export { GUITAR_VOLUMES, MUSICIAN_CHALLENGES, PRODUCER_CHALLENGES };

export function getArtistChallengeCards(volumes) {
  return volumes.flatMap((volume) =>
    (volume.musicians || [])
      .filter((artist) => artist.artistChallenge)
      .map((artist) => ({
        id: `ART-${artist.id}`,
        title: artist.artistChallenge.title,
        artist: artist.name,
        text: artist.artistChallenge.description,
        volumeId: volume.id
      }))
  );
}

export const MUSICIAN_ARTIST_CHALLENGES = getArtistChallengeCards(GUITAR_VOLUMES);
export const PRODUCER_ARTIST_CHALLENGES = getArtistChallengeCards(PRODUCER_VOLUMES);

export const MUSICIAN_CHALLENGE_CARDS = [
  ...MUSICIAN_CHALLENGES,
  ...MUSICIAN_ARTIST_CHALLENGES
];

export const PRODUCER_CHALLENGE_CARDS = [
  ...PRODUCER_CHALLENGES,
  ...PRODUCER_ARTIST_CHALLENGES
];
