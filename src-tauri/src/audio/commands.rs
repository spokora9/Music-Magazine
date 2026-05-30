// SHED POWER: Cross-Thread Command Registry

#[derive(Debug, Clone, serde::Deserialize)]
pub struct JamChordCommand {
    pub notes: Vec<u8>,
    pub beats: u64,
    pub name: Option<String>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, std::hash::Hash, serde::Deserialize, serde::Serialize,
)]
pub enum LooperSource {
    InputChannel(u8),
    InputMix,
    Synth,
    Mpc,
    Jam,
    InstrumentMix,
    Silent,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum AudioCommand {
    // Transport
    Play,
    Stop,
    StopAllSounds,
    TogglePlayback,
    Record {
        part_id: usize,
    }, // 0=A, 1=B, 2=C, 3=D, 4=E
    Overdub {
        part_id: usize,
    },
    ToggleLooper {
        part_id: usize,
    }, // RC-505 Style
    SelectPart {
        part_id: usize,
    },
    ClearPart {
        part_id: usize,
    },
    Undo {
        part_id: usize,
    },

    // Synth
    NoteOn {
        note: u8,
        velocity: u8,
    },
    NoteOff {
        note: u8,
    },
    SetParam {
        id: u8,
        value: f32,
    },
    SetMpcParam {
        id: u8,
        value: f32,
    },
    SetMpcKit {
        kit: u8,
    },
    SetMpcStep {
        pad_id: usize,
        step: usize,
        active: bool,
    },
    SetMpcSampleTrim {
        pad_id: usize,
        start: f32,
        end: f32,
        volume: f32,
        pitch: f32,
    },
    StartMpcSequencer {
        bpm: f32,
        swing: f32,
    },
    StopMpcSequencer,
    SetJamParam {
        id: u8,
        value: f32,
    }, // Separate params for jam station

    // Looper
    SetMicGain {
        gain: f32,
    },
    SetMicActive {
        active: bool,
    },
    SetNativeInputChannel {
        channel: u8,
    }, // 0=mix, 1..8=interface inputs for monitor/tuner
    SetLooperPartInput {
        part_id: usize,
        channel: u8,
    }, // Legacy alias: 0=mix, 1..8=interface inputs
    SetLooperPartSource {
        part_id: usize,
        source: LooperSource,
    },
    SetLooperFx {
        part_id: usize,
        effect_id: u8,
        value: f32,
    },
    ToggleLooperPartActive {
        part_id: usize,
    },

    // Jam Station
    PlayChord {
        notes: Vec<u8>,
        tempo: f32,
    },
    StopChord,
    JamControl {
        action: u8,
    }, // 0=Play, 1=Stop, 2=Next, 3=Prev
    SetJamSound {
        preset_id: u8,
    }, // 0=Piano, 1=E-Piano, 2=Organ
    PlayJamTrack {
        chords: Vec<JamChordCommand>,
        tempo: f32,
    },

    // Jam Station Enhancements
    SetBasslineEnabled {
        enabled: bool,
    },
    SetHarmonicsEnabled {
        enabled: bool,
    },
    SetBasslineStyle {
        style: u8,
    }, // 0=Root, 1=Octave, 2=Walking, 3=Rhythmic
    SetBasslinePreset {
        preset_id: u8,
    },
    SetHarmonicsPreset {
        preset_id: u8,
    },
    PlayCustomSong {
        parts: Vec<String>,
        tempo: f32,
    }, // Custom song structure

    // System / File IO
    SaveProject {
        path: String,
    },
    LoadPartBuffer {
        part_id: usize,
        data: Vec<f32>,
    },
    LoadPartLayers {
        part_id: usize,
        layers: Vec<Vec<f32>>,
    },
    LoadProjectDone {
        all_empty: bool,
    },
    UploadSample {
        pad_id: usize,
        data: Vec<f32>,
    },

    // Metronome
    SetMetronome {
        enabled: bool,
        bpm: f32,
    },

    // Song Sequence
    PlaySequence {
        parts: Vec<usize>,
    },
    StopSequence,

    // Input Monitoring
    SetInputMonitoring {
        active: bool,
    },

    // Device Management
    ScanDevices,
    RefreshMidi,
    RefreshAudio,
}
