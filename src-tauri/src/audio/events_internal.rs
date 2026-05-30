// SHED POWER: Internal Audio Thread -> Main Thread Events

#[derive(Debug)]
pub enum AudioThreadEvent {
    WaveformReady {
        part_id: usize,
        data: Vec<f32>,
    },
    ParamChange {
        id: u8,
        value: f32,
    },
    MpcParamChange {
        id: u8,
        value: f32,
    },
    JamParamChange {
        id: u8,
        value: f32,
    },
    PartActive {
        part_id: usize,
    },

    // Snapshot for Saving
    ProjectSnapshot {
        buffers: Vec<Vec<f32>>,
        layers: Vec<Vec<Vec<f32>>>,
        sources: Vec<crate::audio::LooperSource>,
        sample_rate: u32,
    },

    JamControl {
        action: u8,
    },
    JamChordStep {
        index: usize,
        notes: Vec<u8>,
        label: String,
    },

    // Diagnostics
    #[cfg_attr(target_os = "android", allow(dead_code))]
    MidiDebug {
        status: u8,
        data1: u8,
        data2: u8,
    },
    EngineInfo {
        sample_rate: f32,
        input_device: String,
    },
    TunerReading {
        frequency: f32,
        rms: f32,
    },
    DrumTrigger {
        note: u8,
    },
    MpcStep {
        step: usize,
    },
    MpcTransport {
        playing: bool,
    },
    AllSoundsStopped,
    MpcSampleLoaded {
        pad_id: usize,
        sample_rate: u32,
        samples: usize,
        waveform: Vec<f32>,
    },
    LooperStateChange {
        part_id: usize,
        state: String,
    }, // "recording", "playing", "stopped", "overdubbing"
    LooperLayerCount {
        part_id: usize,
        layers: usize,
    },
    MidiActive {
        active: bool,
    },
    LoopDuration {
        samples: usize,
    },

    // Song Sequence Events
    SequenceStep {
        step: usize,
        part_id: usize,
    },
    SequenceFinished,
    ProjectLoaded {
        all_empty: bool,
        samples: usize,
    },

    // Device Management Events
    DeviceListUpdate,
}
