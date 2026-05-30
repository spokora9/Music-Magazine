// SHED POWER: Intelligent Bassline and Harmony Engine
use crate::audio::Voice;

#[derive(Debug, Clone)]
pub struct ChordInfo {
    pub root: u8,
    pub chord_type: ChordType,
    pub start_time: u64, // Sample position when chord started
}

impl ChordInfo {
    pub fn analyze_chord(notes: &[u8]) -> ChordType {
        if notes.len() < 2 {
            return ChordType::Major;
        }

        let normalized: Vec<u8> = notes.iter().map(|&n| n % 12).collect();
        let root = normalized[0];

        let intervals: Vec<u8> = normalized.iter().map(|&n| (n + 12 - root) % 12).collect();

        // Analyze intervals to determine chord type
        if intervals.contains(&3) && intervals.contains(&7) {
            if intervals.contains(&10) {
                ChordType::Minor7
            } else {
                ChordType::Minor
            }
        } else if intervals.contains(&4) && intervals.contains(&7) {
            if intervals.contains(&10) {
                ChordType::Dominant7
            } else if intervals.contains(&11) {
                ChordType::Major7
            } else {
                ChordType::Major
            }
        } else if intervals.contains(&2) && intervals.contains(&7) {
            ChordType::Sus2
        } else if intervals.contains(&5) && intervals.contains(&7) {
            ChordType::Sus4
        } else if intervals.contains(&3) && intervals.contains(&6) {
            ChordType::Diminished
        } else {
            ChordType::Major // Default fallback
        }
    }

    pub fn get_chord_tones(chord: &ChordInfo) -> Vec<u8> {
        let root = chord.root % 12; // Normalize to octave

        match chord.chord_type {
            ChordType::Major => vec![root, root + 4, root + 7], // 1, 3, 5
            ChordType::Minor => vec![root, root + 3, root + 7], // 1, b3, 5
            ChordType::Dominant7 => vec![root, root + 4, root + 7, root + 10], // 1, 3, 5, b7
            ChordType::Minor7 => vec![root, root + 3, root + 7, root + 10], // 1, b3, 5, b7
            ChordType::Major7 => vec![root, root + 4, root + 7, root + 11], // 1, 3, 5, 7
            ChordType::Diminished => vec![root, root + 3, root + 6], // 1, b3, b5
            ChordType::Sus2 => vec![root, root + 2, root + 7],  // 1, 2, 5
            ChordType::Sus4 => vec![root, root + 5, root + 7],  // 1, 4, 5
        }
        .into_iter()
        .map(|n| (n % 12) + (chord.root / 12) * 12)
        .collect()
    }
}

#[derive(Debug, Clone)]
pub enum ChordType {
    Major,
    Minor,
    Dominant7,
    Minor7,
    Major7,
    Diminished,
    Sus2,
    Sus4,
}

#[derive(Debug, Clone)]
pub enum BasslinePattern {
    Root,     // Just the root note
    Octave,   // Root and octave alternating
    Walking,  // Walking bassline connecting chords
    Rhythmic, // Rhythmic pattern with chord tones
}

pub struct BasslineEngine {
    pub current_chord: Option<ChordInfo>,
    pub pattern: BasslinePattern,
    pub sample_rate: f32,
    pub tempo_samples_per_beat: u64, // How many samples per beat
    pub current_sample: u64,
    pub pattern_position: u32, // Position within current pattern
    pub last_bass_note: Option<u8>,
    pub next_chord_queue: Option<ChordInfo>, // For walking bass preparation
}

impl BasslineEngine {
    pub fn new(sample_rate: f32, tempo_bpm: f32) -> Self {
        let samples_per_beat = (sample_rate * 60.0 / tempo_bpm) as u64;

        Self {
            current_chord: None,
            pattern: BasslinePattern::Root,
            sample_rate,
            tempo_samples_per_beat: samples_per_beat,
            current_sample: 0,
            pattern_position: 0,
            last_bass_note: None,
            next_chord_queue: None,
        }
    }

    pub fn set_tempo(&mut self, tempo_bpm: f32) {
        self.tempo_samples_per_beat = (self.sample_rate * 60.0 / tempo_bpm) as u64;
    }

    pub fn set_pattern(&mut self, pattern: BasslinePattern) {
        self.pattern = pattern;
        self.pattern_position = 0; // Reset pattern position
    }

    pub fn new_chord(&mut self, notes: &[u8]) {
        if notes.is_empty() {
            return;
        }

        let root = notes[0];
        let chord_type = ChordInfo::analyze_chord(notes);

        // Queue the next chord for walking bass preparation
        if self.current_chord.is_some() {
            self.next_chord_queue = Some(ChordInfo {
                root,
                chord_type,
                start_time: self.current_sample,
            });
        } else {
            self.current_chord = Some(ChordInfo {
                root,
                chord_type,
                start_time: self.current_sample,
            });
        }

        self.pattern_position = 0;
    }

    pub fn stop_chord(&mut self) {
        self.current_chord = None;
        self.next_chord_queue = None;
        self.pattern_position = 0;
    }

    // Process one sample and return bass note if should trigger
    pub fn process_sample(&mut self, bass_voice: &mut Voice) -> Option<u8> {
        self.current_sample += 1;

        // Check if we should advance the chord
        if let Some(next_chord) = self.next_chord_queue.take() {
            self.current_chord = Some(next_chord);
            self.pattern_position = 0;
        }

        let chord_info = self.current_chord.clone();
        let Some(chord) = chord_info else {
            return None;
        };

        // Calculate position within the beat pattern
        let samples_since_chord = self.current_sample - chord.start_time;
        let beat_position = samples_since_chord / self.tempo_samples_per_beat;
        let sample_within_beat = samples_since_chord % self.tempo_samples_per_beat;

        // Check if we should trigger a new bass note based on pattern
        let should_trigger = match self.pattern {
            BasslinePattern::Root => {
                // Trigger on beat 1 only
                beat_position == 0 && sample_within_beat == 0
            }
            BasslinePattern::Octave => {
                // Trigger on beats 1 and 3 (alternating root and octave)
                (beat_position % 4 == 0 || beat_position % 4 == 2) && sample_within_beat == 0
            }
            BasslinePattern::Walking => {
                // Trigger on every beat (quarter notes)
                sample_within_beat == 0
            }
            BasslinePattern::Rhythmic => {
                // Trigger on 1, 2+ (syncopated)
                (sample_within_beat == 0 || sample_within_beat == self.tempo_samples_per_beat / 2)
                    && beat_position % 2 == 0
            }
        };

        if should_trigger {
            let bass_note = self.get_bass_note_for_pattern(&chord, beat_position);

            // Stop previous note and play new one
            if bass_voice.active {
                bass_voice.note_off();
            }

            if let Some(note) = bass_note {
                bass_voice.note_on(note, 90);
                self.last_bass_note = Some(note);
                return Some(note);
            }
        }

        None
    }

    fn get_bass_note_for_pattern(&mut self, chord: &ChordInfo, beat_position: u64) -> Option<u8> {
        match self.pattern {
            BasslinePattern::Root => Some(BasslineEngine::get_bass_note(chord.root)),
            BasslinePattern::Octave => {
                let is_octave_beat = (beat_position % 4) == 2; // beats 3, 7, 11, etc.
                if is_octave_beat {
                    Some(BasslineEngine::get_bass_note(chord.root) + 12) // Octave up
                } else {
                    Some(BasslineEngine::get_bass_note(chord.root))
                }
            }
            BasslinePattern::Walking => self.get_walking_bass_note(chord, beat_position),
            BasslinePattern::Rhythmic => self.get_rhythmic_bass_note(chord, beat_position),
        }
    }

    fn get_walking_bass_note(&mut self, chord: &ChordInfo, beat_position: u64) -> Option<u8> {
        let beat_in_measure = beat_position % 4;

        match beat_in_measure {
            0 => Some(BasslineEngine::get_bass_note(chord.root)), // Root on beat 1
            1 => {
                // Third or fifth
                let chord_tones = ChordInfo::get_chord_tones(chord);
                chord_tones
                    .get(1)
                    .copied()
                    .map(|n| BasslineEngine::get_bass_note(n))
            }
            2 => {
                // Fifth
                let chord_tones = ChordInfo::get_chord_tones(chord);
                chord_tones
                    .get(2)
                    .copied()
                    .map(|n| BasslineEngine::get_bass_note(n))
            }
            3 => {
                // Approach note to next chord's root (if we have next chord info)
                if let Some(ref next_chord) = self.next_chord_queue {
                    // Play note that walks to next root
                    let current_root = BasslineEngine::get_bass_note(chord.root);
                    let next_root = BasslineEngine::get_bass_note(next_chord.root);

                    if next_root > current_root {
                        Some(current_root + 2) // Whole step up approach
                    } else {
                        Some(current_root - 2) // Whole step down approach
                    }
                } else {
                    // Just play seventh or dominant approach
                    Some(BasslineEngine::get_bass_note(chord.root) - 2) // Seventh approach
                }
            }
            _ => Some(BasslineEngine::get_bass_note(chord.root)),
        }
    }

    fn get_rhythmic_bass_note(&mut self, chord: &ChordInfo, beat_position: u64) -> Option<u8> {
        let beat_in_measure = beat_position % 4;
        let chord_tones = ChordInfo::get_chord_tones(chord);

        match beat_in_measure {
            0 => Some(BasslineEngine::get_bass_note(chord.root)), // Strong beat - root
            1 => None,                                            // Rest
            2 => chord_tones
                .get(2)
                .copied()
                .map(|n| BasslineEngine::get_bass_note(n)), // Fifth
            3 => None,                                            // Rest
            _ => Some(BasslineEngine::get_bass_note(chord.root)),
        }
    }

    fn get_bass_note(note: u8) -> u8 {
        // Put bass notes in proper bass range (typically 2 octaves below)
        if note > 48 {
            note - 24
        } else if note > 36 {
            note - 12
        } else {
            note
        }
    }
}

pub struct HarmonyEngine {
    pub enabled: bool,
    pub voicing_type: u8, // 0=Close, 1=Open, 2=Drop2, 3=Quartal, 4=Extensions
    pub current_chord: Option<ChordInfo>,
    pub previous_voicing: Vec<u8>, // For voice leading
    pub humanize_amount: f32,      // Subtle timing variations
}

#[derive(Debug, Clone)]
pub struct VoicingNote {
    pub note: u8,
}

impl HarmonyEngine {
    pub fn new() -> Self {
        Self {
            enabled: false,
            voicing_type: 0,
            current_chord: None,
            previous_voicing: Vec::new(),
            humanize_amount: 0.15, // 15% humanization
        }
    }

    pub fn set_chord(&mut self, notes: &[u8]) {
        if !notes.is_empty() {
            let root = notes[0];
            let chord_type = ChordInfo::analyze_chord(notes);
            self.current_chord = Some(ChordInfo {
                root,
                chord_type,
                start_time: 0,
            });
        }
    }

    pub fn get_professional_harmony(&mut self, primary_notes: &[u8]) -> Vec<VoicingNote> {
        if !self.enabled || primary_notes.is_empty() {
            return Vec::new();
        }

        let Some(ref chord) = self.current_chord else {
            return Vec::new();
        };

        let harmony_notes = match self.voicing_type {
            0 => self.close_voicing(chord),      // Traditional close harmony
            1 => self.open_voicing(chord),       // Open position voicing
            2 => self.drop2_voicing(chord),      // Jazz drop-2 voicing
            3 => self.quartal_voicing(chord),    // Modern quartal harmony
            4 => self.extensions_voicing(chord), // Extended chords (9ths, 11ths)
            _ => self.close_voicing(chord),
        };

        // Apply voice leading to smooth transitions
        let voiced_harmony = self.apply_voice_leading(harmony_notes);

        // Add humanization (subtle timing)
        self.add_humanization(voiced_harmony)
    }

    fn close_voicing(&self, chord: &ChordInfo) -> Vec<u8> {
        let chord_tones = ChordInfo::get_chord_tones(chord);
        let mut voicing = Vec::new();

        // Build close voicing: Root, Third, Fifth, (Seventh if available)
        if let Some(&root) = chord_tones.get(0) {
            voicing.push(root + 12); // Root an octave up for spacing
        }
        if let Some(&third) = chord_tones.get(1) {
            voicing.push(third + 12);
        }
        if let Some(&fifth) = chord_tones.get(2) {
            voicing.push(fifth + 12);
        }
        if let Some(&seventh) = chord_tones.get(3) {
            voicing.push(seventh + 12);
        }

        voicing
    }

    fn open_voicing(&self, chord: &ChordInfo) -> Vec<u8> {
        let chord_tones = ChordInfo::get_chord_tones(chord);
        let mut voicing = Vec::new();

        // Open voicing: wider intervals, more spread out
        if let Some(&root) = chord_tones.get(0) {
            voicing.push(root + 12); // Root octave up
        }
        if let Some(&fifth) = chord_tones.get(2) {
            voicing.push(fifth + 12); // Fifth in middle
        }
        if let Some(&third) = chord_tones.get(1) {
            voicing.push(third + 24); // Third high
        }
        if let Some(&seventh) = chord_tones.get(3) {
            voicing.push(seventh + 24); // Seventh on top
        }

        voicing
    }

    fn drop2_voicing(&self, chord: &ChordInfo) -> Vec<u8> {
        let chord_tones = ChordInfo::get_chord_tones(chord);
        let mut voicing = Vec::new();

        // Jazz drop-2: Drop the second highest note an octave
        if chord_tones.len() >= 3 {
            if let Some(&root) = chord_tones.get(0) {
                voicing.push(root + 12);
            }
            if let Some(&fifth) = chord_tones.get(2) {
                voicing.push(fifth + 12); // Dropped note
            }
            if let Some(&seventh) = chord_tones.get(3) {
                voicing.push(seventh + 24);
            }
            if let Some(&third) = chord_tones.get(1) {
                voicing.push(third + 24);
            }
        }

        voicing
    }

    fn quartal_voicing(&self, chord: &ChordInfo) -> Vec<u8> {
        // Modern harmony: stacked fourths instead of thirds
        let root = chord.root;
        let mut voicing = Vec::new();

        voicing.push(root + 12); // Root
        voicing.push(root + 17); // Perfect 4th (5 semitones up)
        voicing.push(root + 22); // Another 4th
        voicing.push(root + 27); // Another 4th

        voicing
    }

    fn extensions_voicing(&self, chord: &ChordInfo) -> Vec<u8> {
        let chord_tones = ChordInfo::get_chord_tones(chord);
        let mut voicing = Vec::new();
        let root = chord.root;

        // Add basic triad
        if let Some(&chord_root) = chord_tones.get(0) {
            voicing.push(chord_root + 12);
        }
        if let Some(&third) = chord_tones.get(1) {
            voicing.push(third + 12);
        }
        if let Some(&fifth) = chord_tones.get(2) {
            voicing.push(fifth + 12);
        }

        // Add extensions based on chord type
        match chord.chord_type {
            ChordType::Major | ChordType::Major7 => {
                voicing.push(root + 14); // 9th (Major 2nd + octave)
                voicing.push(root + 29); // 13th (Major 6th + 2 octaves)
            }
            ChordType::Minor | ChordType::Minor7 => {
                voicing.push(root + 14); // 9th
                voicing.push(root + 17); // 11th (Perfect 4th + octave)
            }
            ChordType::Dominant7 => {
                voicing.push(root + 14); // 9th
                voicing.push(root + 22); // #11th (tritone + octave)
                voicing.push(root + 28); // 13th (minor 6th + 2 octaves)
            }
            _ => {
                voicing.push(root + 14); // Default: add 9th
            }
        }

        voicing
    }

    fn apply_voice_leading(&mut self, new_voicing: Vec<u8>) -> Vec<u8> {
        if self.previous_voicing.is_empty() {
            self.previous_voicing = new_voicing.clone();
            return new_voicing;
        }

        // Simple voice leading: move each voice to closest note in new chord
        let mut led_voicing = Vec::new();

        for (i, &new_note) in new_voicing.iter().enumerate() {
            if let Some(&prev_note) = self.previous_voicing.get(i) {
                // Find the closest voicing of this note to the previous note
                let mut closest_note = new_note;
                let mut closest_distance = ((new_note as i32) - (prev_note as i32)).abs();

                // Check octave above and below
                for octave_offset in [-12i8, 0, 12] {
                    let candidate = (new_note as i32 + octave_offset as i32) as u8;
                    if candidate > 24 && candidate < 96 {
                        // Stay in reasonable range
                        let distance = ((candidate as i32) - (prev_note as i32)).abs();
                        if distance < closest_distance {
                            closest_distance = distance;
                            closest_note = candidate;
                        }
                    }
                }

                led_voicing.push(closest_note);
            } else {
                led_voicing.push(new_note);
            }
        }

        self.previous_voicing = led_voicing.clone();
        led_voicing
    }

    fn add_humanization(&mut self, voicing: Vec<u8>) -> Vec<VoicingNote> {
        let mut humanized = Vec::new();

        for (i, &note) in voicing.iter().enumerate() {
            // Add subtle timing offset (max ±50 samples at 44.1kHz = ±1.13ms)
            let timing_offset = if self.humanize_amount > 0.0 {
                let max_offset = (50.0 * self.humanize_amount) as i32;
                // Simple pseudo-random based on note and voice
                let seed = (note as i32 * 7 + i as i32 * 13) % 101;
                (seed - 50) * max_offset / 50
            } else {
                0
            };

            let _ = timing_offset;
            humanized.push(VoicingNote { note });
        }

        humanized
    }
}
