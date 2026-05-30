import {
  chordScaleOptions,
  progressionScaleLinks
} from "../src/lib/modalMixture.js";

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function findScale(options, root, type) {
  return options.find(option => option.root === root && option.type === type);
}

function assertMetadata(option, context) {
  assert(option, `${context}: missing option`);
  assert(typeof option.id === "string" && option.id.length > 0, `${context}: missing stable id`);
  assert(typeof option.label === "string" && option.label.length > 0, `${context}: missing label`);
  assert(typeof option.root === "string" && option.root.length > 0, `${context}: missing root`);
  assert(typeof option.type === "string" && option.type.length > 0, `${context}: missing type`);
  assert(Array.isArray(option.notes) && option.notes.length > 0, `${context}: missing notes`);
  assert(typeof option.exact === "boolean", `${context}: missing exact flag`);
  assert(Number.isFinite(option.coverage), `${context}: missing coverage`);
  assert(Array.isArray(option.missing), `${context}: missing missing-notes list`);
  assert(Array.isArray(option.clashes), `${context}: missing clashes list`);
  assert(Number.isInteger(option.suggested), `${context}: missing suggested order`);
}

function check(name, fn) {
  fn();
  console.log(`[PASS] ${name}`);
}

check("current chord candidates use actual chord tones", () => {
  const options = chordScaleOptions({ name: "C", notes: ["C3", "E3", "G3", "Bb3"] }, "C", 24);
  assert(findScale(options, "C", "mixolydian"), "C7 tones should allow C Mixolydian");
  assert(!findScale(options, "C", "major"), "C7 tones should not allow C Ionian");
  assertMetadata(options[0], "C7 actual-tones option");
});

check("C plus Bb7 exposes C Mixolydian b6 as an exact link", () => {
  const options = progressionScaleLinks({
    progression: [
      { name: "C", notes: ["C3", "E3", "G3"] },
      { name: "Bb7", notes: ["Bb2", "D3", "F3", "Ab3"] }
    ]
  }, "C", 24);
  const link = findScale(options, "C", "mixolydian_b6");
  assertMetadata(link, "C Mixolydian b6 link");
  assert(link.exact, "C Mixolydian b6 should fully cover C plus Bb7");
  assert(link.coverage === 1, `expected full coverage, got ${link.coverage}`);
  assert(link.missing.length === 0, `expected no missing notes, got ${link.missing.join(" ")}`);
});

check("Dm in A suggests A Harmonic Major locally", () => {
  const options = chordScaleOptions("Dm", "A", 24);
  const harmonicMajor = findScale(options, "A", "harmonic_major");
  assertMetadata(harmonicMajor, "A Harmonic Major over Dm");
  assert(harmonicMajor.exact, "A Harmonic Major should cover Dm");
  assert(options[0].id === harmonicMajor.id, `expected A Harmonic Major first, got ${options[0].label}`);
});

check("G in A keeps multiple home-root origins visible", () => {
  const options = chordScaleOptions("G", "A", 24);
  for (const type of ["mixolydian", "dorian", "minor"]) {
    const option = findScale(options, "A", type);
    assertMetadata(option, `A ${type} over G`);
  }
});

check("bridge links fall back to best coverage with missing notes", () => {
  const options = progressionScaleLinks({
    progression: [
      { name: "cluster-a", notes: ["C3", "C#3", "D3", "D#3"] },
      { name: "cluster-b", notes: ["E3", "F3", "F#3", "G3"] }
    ]
  }, "C", 24);
  assert(options.length > 0, "expected bridge candidates");
  assert(options.every(option => !option.exact), "expected bridge candidates, not exact links");
  assert(options.every(option => option.missing.length > 0), "expected missing notes on bridge candidates");
  assert(new Set(options.map(option => option.coveredCount)).size === 1, "expected only max-coverage candidates");
  assertMetadata(options[0], "best bridge option");
});

console.log("[PASS] modal mixture verification complete");
