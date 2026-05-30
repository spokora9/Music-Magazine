// SHED POWER: MIDI Mapping Store
use crate::audio::LooperSource;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MidiTarget {
    Param(u8),     // Synth Params 0-7
    Transport(u8), // 0=Play, 1=Stop, 2=Rec
    // New Targets for Looper
    LooperRecord(usize),  // Force Rec
    LooperOverdub(usize), // Force Dub
    LooperToggle(usize),  // RC-505 Style (Rec/Play/Dub)
    LooperSelect(usize),  // Focus Part
    LooperClear(usize),
    LooperUndo(usize),
    LooperActiveToggle(usize), // Pause/resume an existing loop part
    LooperSourceInput {
        part_id: usize,
        channel: u8,
    }, // 0=mix, 1..8=interface inputs
    LooperSource {
        part_id: usize,
        source: LooperSource,
    },
    MicGain,
    Note(u8),     // Trigger Note
    MpcParam(u8), // 0=Swing, 1=Kit

    // Jam Station
    JamPlay,
    JamStop,
    JamNext,
    JamPrev,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MidiMap {
    // Context -> (CC Number -> Target)
    pub cc_maps: HashMap<String, HashMap<u8, MidiTarget>>,
    pub note_maps: HashMap<String, HashMap<u8, MidiTarget>>,
    pub active_context: String,
}

impl MidiMap {
    pub fn new() -> Self {
        let mut map = Self {
            cc_maps: HashMap::new(),
            note_maps: HashMap::new(),
            active_context: "synth".to_string(), // Default
        };

        // --- SYNTH DEFAULTS ---
        let synth_cc = HashMap::new();
        /*
        for i in 0..8 {
            synth_cc.insert(i + 1, MidiTarget::Param(i)); // CC 1-8 -> Params 0-7
        }
        */
        map.cc_maps.insert("synth".to_string(), synth_cc);

        // --- LOOPER DEFAULTS ---
        let mut looper_cc = HashMap::new();
        // CC 1-5: record A-E, 6-10: RC-style toggle A-E, 11-15: select A-E.
        for part_id in 0..5 {
            looper_cc.insert(1 + part_id as u8, MidiTarget::LooperRecord(part_id));
            looper_cc.insert(6 + part_id as u8, MidiTarget::LooperToggle(part_id));
            looper_cc.insert(11 + part_id as u8, MidiTarget::LooperSelect(part_id));
        }
        map.cc_maps.insert("looper".to_string(), looper_cc);

        map
    }

    #[cfg_attr(target_os = "android", allow(dead_code))]
    pub fn get_target(&self, status: u8, data1: u8) -> Option<MidiTarget> {
        let context = &self.active_context;

        // 1. Try Context-Specific Map
        if status == 0xB0 {
            if let Some(map) = self.cc_maps.get(context) {
                if let Some(target) = map.get(&data1) {
                    return Some(*target);
                }
            }
        }

        if status == 0x90 || status == 0x80 {
            if let Some(map) = self.note_maps.get(context) {
                if let Some(target) = map.get(&data1) {
                    return Some(*target);
                }
            }
        }

        // 2. Try "Global" Map (Future feature)

        None
    }
}
