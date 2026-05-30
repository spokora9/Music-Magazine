// SHED POWER: Main Audio Engine Controller
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::audio::oscillator::Waveform;
use crate::audio::sampler::Sampler;
use crate::audio::{
    AudioCommand, AudioThreadEvent, BasslineEngine, FxChain, HarmonyEngine, JamChordCommand,
    Looper, LooperSource, Voice, LOOPER_PART_COUNT,
};
use crate::audio::{HiHat, Kick, Snare};

fn normalized_mpc_kit_index(value: f32) -> u8 {
    ((value.clamp(0.0, 1.0) * 4.0).floor() as u8).min(3)
}

fn mpc_step_interval_samples(step: usize, bpm: f32, swing: f32, sample_rate: f32) -> u64 {
    let safe_bpm = bpm.clamp(40.0, 240.0);
    let straight = sample_rate * 60.0 / safe_bpm / 4.0;
    let swing_delay = straight * swing.clamp(0.0, 0.5) * 0.66;
    let interval = if step % 2 == 0 {
        straight + swing_delay
    } else {
        straight - swing_delay
    };

    interval.max(1.0) as u64
}

fn trigger_synth_voice(voices: &mut [Voice], note: u8, velocity: u8) {
    if let Some(voice) = voices.iter_mut().find(|v| !v.active) {
        voice.note_on(note, velocity);
    }
}

const MAX_NATIVE_INPUT_CHANNELS: usize = 8;

#[derive(Clone, Copy, Debug)]
struct NativeInputFrame {
    channels: [f32; MAX_NATIVE_INPUT_CHANNELS],
    channel_count: usize,
}

impl Default for NativeInputFrame {
    fn default() -> Self {
        Self {
            channels: [0.0; MAX_NATIVE_INPUT_CHANNELS],
            channel_count: 0,
        }
    }
}

impl NativeInputFrame {
    fn from_f32_frame(frame: &[f32]) -> Self {
        Self::from_mapped_frame(frame, |sample| *sample)
    }

    fn from_mapped_frame<T, F>(frame: &[T], convert: F) -> Self
    where
        F: Fn(&T) -> f32,
    {
        let mut input = Self::default();
        let count = frame.len().min(MAX_NATIVE_INPUT_CHANNELS);
        input.channel_count = count;

        for (idx, sample) in frame.iter().take(count).enumerate() {
            input.channels[idx] = convert(sample).clamp(-1.0, 1.0);
        }

        input
    }

    fn sample_for_mode(&self, mode: usize) -> f32 {
        if self.channel_count == 0 {
            return 0.0;
        }

        if mode == 0 {
            let sum: f32 = self.channels[..self.channel_count].iter().copied().sum();
            return (sum / self.channel_count as f32).clamp(-1.0, 1.0);
        }

        if mode <= self.channel_count && mode <= MAX_NATIVE_INPUT_CHANNELS {
            self.channels[mode - 1]
        } else {
            0.0
        }
    }
}

fn input_mode_label(mode: usize) -> String {
    if mode == 0 {
        "Input Mix".to_string()
    } else {
        format!("Input {}", mode)
    }
}

fn legacy_channel_to_looper_source(channel: u8) -> LooperSource {
    if channel == 0 {
        LooperSource::InputMix
    } else {
        LooperSource::InputChannel(channel.min(MAX_NATIVE_INPUT_CHANNELS as u8))
    }
}

fn normalize_looper_source(source: LooperSource) -> LooperSource {
    match source {
        LooperSource::InputChannel(0) => LooperSource::InputMix,
        LooperSource::InputChannel(channel) => {
            LooperSource::InputChannel(channel.min(MAX_NATIVE_INPUT_CHANNELS as u8))
        }
        other => other,
    }
}

fn default_looper_sources() -> [LooperSource; LOOPER_PART_COUNT] {
    [
        LooperSource::InputChannel(1),
        LooperSource::InputChannel(2),
        LooperSource::InputMix,
        LooperSource::Silent,
        LooperSource::Silent,
    ]
}

fn looper_source_label(source: LooperSource) -> String {
    match source {
        LooperSource::InputChannel(channel) => format!("Input {}", channel),
        LooperSource::InputMix => "Input Mix".to_string(),
        LooperSource::Synth => "Synth".to_string(),
        LooperSource::Mpc => "MPC".to_string(),
        LooperSource::Jam => "Jam".to_string(),
        LooperSource::InstrumentMix => "Instrument Mix".to_string(),
        LooperSource::Silent => "Silent".to_string(),
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct AudioBuses {
    native_input: NativeInputFrame,
    synth: f32,
    mpc: f32,
    jam: f32,
}

fn sample_for_looper_source(source: LooperSource, buses: AudioBuses) -> f32 {
    match normalize_looper_source(source) {
        LooperSource::InputChannel(channel) => buses.native_input.sample_for_mode(channel as usize),
        LooperSource::InputMix => buses.native_input.sample_for_mode(0),
        LooperSource::Synth => buses.synth,
        LooperSource::Mpc => buses.mpc,
        LooperSource::Jam => buses.jam,
        LooperSource::InstrumentMix => buses.synth + buses.mpc + buses.jam,
        LooperSource::Silent => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn default_looper_sources_match_v2_safe_defaults() {
        assert_eq!(
            default_looper_sources(),
            [
                LooperSource::InputChannel(1),
                LooperSource::InputChannel(2),
                LooperSource::InputMix,
                LooperSource::Silent,
                LooperSource::Silent,
            ]
        );
    }

    #[test]
    fn looper_sources_route_to_explicit_buses() {
        let buses = AudioBuses {
            native_input: NativeInputFrame::from_f32_frame(&[0.25, -0.75]),
            synth: 0.1,
            mpc: 0.2,
            jam: 0.3,
        };

        assert_close(
            sample_for_looper_source(LooperSource::InputChannel(1), buses),
            0.25,
        );
        assert_close(
            sample_for_looper_source(LooperSource::InputChannel(2), buses),
            -0.75,
        );
        assert_close(
            sample_for_looper_source(LooperSource::InputMix, buses),
            -0.25,
        );
        assert_close(sample_for_looper_source(LooperSource::Synth, buses), 0.1);
        assert_close(sample_for_looper_source(LooperSource::Mpc, buses), 0.2);
        assert_close(sample_for_looper_source(LooperSource::Jam, buses), 0.3);
        assert_close(
            sample_for_looper_source(LooperSource::InstrumentMix, buses),
            0.6,
        );
        assert_close(sample_for_looper_source(LooperSource::Silent, buses), 0.0);
    }

    #[test]
    fn selected_looper_sources_record_into_matching_parts() {
        let buses = AudioBuses {
            native_input: NativeInputFrame::from_f32_frame(&[0.4, 0.0]),
            synth: 0.1,
            mpc: 0.2,
            jam: 0.3,
        };
        let sources = [
            LooperSource::InputMix,
            LooperSource::Synth,
            LooperSource::Mpc,
            LooperSource::Jam,
            LooperSource::InstrumentMix,
        ];
        let mut looper = Looper::new();

        for part_id in 0..LOOPER_PART_COUNT {
            looper.start_recording(part_id);
        }

        let inputs =
            std::array::from_fn(|part_id| sample_for_looper_source(sources[part_id], buses));
        looper.process(inputs);

        for part_id in 0..LOOPER_PART_COUNT {
            assert!(looper.stop_recording(part_id));
        }

        let expected = [0.2, 0.1, 0.2, 0.3, 0.6];
        for (part_id, expected_sample) in expected.into_iter().enumerate() {
            assert_eq!(looper.parts[part_id].layer_count(), 1);
            assert_close(looper.parts[part_id].buffer[0], expected_sample);
        }
    }

    #[test]
    fn looper_source_normalization_keeps_legacy_channel_semantics() {
        assert_eq!(legacy_channel_to_looper_source(0), LooperSource::InputMix);
        assert_eq!(
            legacy_channel_to_looper_source(99),
            LooperSource::InputChannel(MAX_NATIVE_INPUT_CHANNELS as u8)
        );
        assert_eq!(
            normalize_looper_source(LooperSource::InputChannel(0)),
            LooperSource::InputMix
        );
    }
}

fn push_native_input_f32(
    data: &[f32],
    channels: usize,
    input_prod: &mut HeapProducer<NativeInputFrame>,
) {
    let channels = channels.max(1);
    for frame in data.chunks(channels) {
        let _ = input_prod.push(NativeInputFrame::from_f32_frame(frame));
    }
}

fn push_native_input_i8(
    data: &[i8],
    channels: usize,
    input_prod: &mut HeapProducer<NativeInputFrame>,
) {
    let channels = channels.max(1);
    for frame in data.chunks(channels) {
        let _ = input_prod.push(NativeInputFrame::from_mapped_frame(frame, |sample| {
            *sample as f32 / i8::MAX as f32
        }));
    }
}

fn push_native_input_i16(
    data: &[i16],
    channels: usize,
    input_prod: &mut HeapProducer<NativeInputFrame>,
) {
    let channels = channels.max(1);
    for frame in data.chunks(channels) {
        let _ = input_prod.push(NativeInputFrame::from_mapped_frame(frame, |sample| {
            *sample as f32 / i16::MAX as f32
        }));
    }
}

fn push_native_input_u8(
    data: &[u8],
    channels: usize,
    input_prod: &mut HeapProducer<NativeInputFrame>,
) {
    let channels = channels.max(1);
    for frame in data.chunks(channels) {
        let _ = input_prod.push(NativeInputFrame::from_mapped_frame(frame, |sample| {
            (*sample as f32 / u8::MAX as f32) * 2.0 - 1.0
        }));
    }
}

fn push_native_input_u16(
    data: &[u16],
    channels: usize,
    input_prod: &mut HeapProducer<NativeInputFrame>,
) {
    let channels = channels.max(1);
    for frame in data.chunks(channels) {
        let _ = input_prod.push(NativeInputFrame::from_mapped_frame(frame, |sample| {
            (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0
        }));
    }
}

fn input_format_rank(format: SampleFormat) -> u8 {
    match format {
        SampleFormat::F32 => 0,
        SampleFormat::I16 => 1,
        SampleFormat::U16 => 2,
        SampleFormat::I8 => 6,
        SampleFormat::U8 => 7,
        _ => 20,
    }
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|sample| sample * sample).sum();
    (sum / samples.len() as f32).sqrt()
}

fn detect_tuner_frequency(samples: &[f32], sample_rate: f32) -> Option<f32> {
    let level = rms(samples);
    if level < 0.003 || samples.len() < 256 {
        return None;
    }

    let min_lag = (sample_rate / 1000.0).max(1.0) as usize;
    let max_lag = (sample_rate / 40.0).min(samples.len() as f32 / 2.0) as usize;
    if max_lag <= min_lag {
        return None;
    }

    let mut best_lag = 0usize;
    let mut best_corr = 0.0f32;

    for lag in min_lag..=max_lag {
        let mut corr = 0.0f32;
        for i in 0..(samples.len() - lag) {
            corr += samples[i] * samples[i + lag];
        }

        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    if best_lag == 0 || best_corr < level * level * samples.len() as f32 * 0.15 {
        return None;
    }

    Some(sample_rate / best_lag as f32)
}

fn trigger_mpc_tone_voice(voices: &mut [Voice], note: u8, velocity: u8) {
    if let Some(voice) = voices.iter_mut().find(|v| !v.active) {
        voice.osc1.waveform = Waveform::Square;
        voice.osc2.waveform = Waveform::Sine;
        voice.filter.cutoff = 5000.0;
        voice.filter.resonance = 0.25;
        voice.env.attack = 0.001;
        voice.env.decay = 0.12;
        voice.env.sustain = 0.0;
        voice.env.release = 0.03;
        voice.note_on(note, velocity);
    }
}

fn stop_mpc_output(
    sampler: &mut Sampler,
    kick: &mut Kick,
    snare: &mut Snare,
    hihat: &mut HiHat,
    mpc_voices: &mut [Voice],
) {
    sampler.stop_all();
    kick.stop();
    snare.stop();
    hihat.stop();
    for voice in mpc_voices {
        voice.silence();
    }
}

fn silence_voices(voices: &mut [Voice]) {
    for voice in voices {
        voice.silence();
    }
}

fn trigger_mpc_kit_pad(
    pad_id: usize,
    kit: u8,
    velocity: u8,
    kick: &mut Kick,
    snare: &mut Snare,
    hihat: &mut HiHat,
    mpc_voices: &mut [Voice],
) {
    match kit {
        // Classic 808: low pads are kicks/sub/toms, middle pads snares/claps, top pads hats/fx.
        0 => match pad_id {
            0 | 1 | 2 | 3 => kick.trigger(),
            4 | 5 | 6 | 7 => snare.trigger(),
            8 | 9 | 10 | 11 | 13 => hihat.trigger(),
            12 | 14 | 15 => trigger_mpc_tone_voice(mpc_voices, 60 + pad_id as u8, velocity),
            _ => {}
        },
        // House 909: tighter kick/snare/hat groups, with higher pads as simple percussion tones.
        1 => match pad_id {
            0 | 1 | 2 => kick.trigger(),
            3 | 4 | 5 | 6 | 7 => snare.trigger(),
            8 | 9 | 10 | 11 | 12 => hihat.trigger(),
            13 | 14 | 15 => trigger_mpc_tone_voice(mpc_voices, 64 + pad_id as u8, velocity),
            _ => {}
        },
        // Acoustic: more snare/tom-style fallback in the middle, cymbal hats on the right.
        2 => match pad_id {
            0 | 1 => kick.trigger(),
            2 | 3 | 4 | 5 | 6 | 7 => snare.trigger(),
            8 | 9 | 10 | 11 | 12 | 13 => hihat.trigger(),
            14 | 15 => trigger_mpc_tone_voice(mpc_voices, 55 + pad_id as u8, velocity),
            _ => {}
        },
        // Lo-fi: conservative MPC-style fallback with most non-kick pads acting percussive.
        _ => match pad_id {
            0 | 1 | 3 => kick.trigger(),
            4 | 5 | 6 | 7 | 12 | 13 => snare.trigger(),
            8 | 9 | 10 | 11 => hihat.trigger(),
            2 | 14 | 15 => trigger_mpc_tone_voice(mpc_voices, 48 + pad_id as u8, velocity),
            _ => {}
        },
    }
}

#[derive(Debug, Clone)]
struct JamSongChord {
    notes: Vec<u8>,
    beats: u64,
    label: String,
}

struct JamSong {
    chords: Vec<JamSongChord>,
    tempo: f32,
    samples_per_beat: u64,
    chord_index: usize,
    frames_in_chord: u64,
}

impl JamSong {
    fn from_chords(chords: Vec<JamChordCommand>, tempo: f32, sample_rate: f32) -> Option<Self> {
        let chords: Vec<JamSongChord> = chords
            .into_iter()
            .filter(|chord| !chord.notes.is_empty())
            .map(|chord| JamSongChord {
                notes: chord.notes,
                beats: chord.beats.max(1),
                label: chord.name.unwrap_or_else(|| "Chord".to_string()),
            })
            .collect();

        Self::new(chords, tempo, sample_rate)
    }

    fn from_parts(parts: Vec<String>, tempo: f32, sample_rate: f32) -> Option<Self> {
        let chords: Vec<JamSongChord> = parts
            .iter()
            .flat_map(|part| part.split(|c: char| c.is_whitespace() || c == ',' || c == '|'))
            .filter_map(parse_custom_chord)
            .collect();

        Self::new(chords, tempo, sample_rate)
    }

    fn new(chords: Vec<JamSongChord>, tempo: f32, sample_rate: f32) -> Option<Self> {
        if chords.is_empty() {
            return None;
        }

        let safe_tempo = tempo.clamp(40.0, 240.0);
        Some(Self {
            chords,
            tempo: safe_tempo,
            samples_per_beat: (sample_rate * 60.0 / safe_tempo) as u64,
            chord_index: 0,
            frames_in_chord: 0,
        })
    }

    fn current_step(&self) -> (usize, Vec<u8>, String) {
        let chord = &self.chords[self.chord_index];
        (self.chord_index, chord.notes.clone(), chord.label.clone())
    }

    fn advance_frame(&mut self) -> Option<(usize, Vec<u8>, String)> {
        self.frames_in_chord += 1;
        let chord_frames = self.chords[self.chord_index].beats * self.samples_per_beat;

        if self.frames_in_chord >= chord_frames {
            self.frames_in_chord = 0;
            self.chord_index = (self.chord_index + 1) % self.chords.len();
            return Some(self.current_step());
        }

        None
    }
}

#[cfg(test)]
mod jam_song_tests {
    use super::{
        silence_jam_output, silence_voices, trigger_jam_chord, JamSong, JamSongChord,
        NativeInputFrame,
    };
    use crate::audio::{AdsrState, BasslineEngine, HarmonyEngine, JamChordCommand, Voice};

    fn chord(label: &str, beats: u64) -> JamSongChord {
        JamSongChord {
            notes: vec![60, 64, 67],
            beats,
            label: label.to_string(),
        }
    }

    #[test]
    fn jam_song_advances_and_wraps_by_beat_lengths() {
        let mut song = JamSong::new(vec![chord("C", 2), chord("F", 1)], 60.0, 10.0).unwrap();

        assert_eq!(song.current_step().0, 0);
        for _ in 0..19 {
            assert!(song.advance_frame().is_none());
        }

        let next = song.advance_frame().unwrap();
        assert_eq!(next.0, 1);
        assert_eq!(next.2, "F");

        for _ in 0..9 {
            assert!(song.advance_frame().is_none());
        }

        let wrapped = song.advance_frame().unwrap();
        assert_eq!(wrapped.0, 0);
        assert_eq!(wrapped.2, "C");
    }

    #[test]
    fn play_jam_track_chords_filter_empty_notes_and_clamp_tempo() {
        let mut song = JamSong::from_chords(
            vec![
                JamChordCommand {
                    notes: Vec::new(),
                    beats: 4,
                    name: Some("Muted".to_string()),
                },
                JamChordCommand {
                    notes: vec![60, 64, 67],
                    beats: 0,
                    name: Some("C".to_string()),
                },
            ],
            12.0,
            100.0,
        )
        .unwrap();

        assert_eq!(song.tempo, 40.0);
        assert_eq!(song.samples_per_beat, 150);
        assert_eq!(song.chords.len(), 1);
        assert_eq!(song.chords[0].beats, 1);
        assert_eq!(song.current_step(), (0, vec![60, 64, 67], "C".to_string()));

        for _ in 0..149 {
            assert!(song.advance_frame().is_none());
        }

        assert_eq!(
            song.advance_frame().unwrap(),
            (0, vec![60, 64, 67], "C".to_string())
        );
    }

    #[test]
    fn play_jam_track_with_no_playable_chords_is_ignored() {
        let song = JamSong::from_chords(
            vec![JamChordCommand {
                notes: Vec::new(),
                beats: 4,
                name: Some("Muted".to_string()),
            }],
            120.0,
            100.0,
        );

        assert!(song.is_none());
    }

    #[test]
    fn custom_song_parts_parse_to_jam_song_steps() {
        let song = JamSong::from_parts(
            vec!["Bb7 | F#m7b5".to_string(), "Cmaj7".to_string()],
            300.0,
            100.0,
        )
        .unwrap();

        assert_eq!(song.tempo, 240.0);
        assert_eq!(song.chords.len(), 3);
        assert_eq!(song.chords[0].label, "Bb7");
        assert_eq!(song.chords[1].label, "F#m7b5");
        assert_eq!(song.chords[2].label, "Cmaj7");
    }

    #[test]
    fn empty_jam_chord_releases_bassline_and_harmony_state() {
        let sample_rate = 100.0;
        let mut jam_voices: Vec<Voice> = (0..8).map(|_| Voice::new(sample_rate)).collect();
        let mut bass_voice = Voice::new(sample_rate);
        let mut bassline_engine = BasslineEngine::new(sample_rate, 120.0);
        let mut harmony_engine = HarmonyEngine::new();
        harmony_engine.enabled = true;

        trigger_jam_chord(
            &[60, 64, 67],
            120.0,
            &mut jam_voices,
            &mut bass_voice,
            &mut bassline_engine,
            &mut harmony_engine,
            false,
            true,
        );

        assert!(jam_voices.iter().any(|voice| voice.active));
        assert!(bass_voice.active);
        assert!(bassline_engine.current_chord.is_some());
        assert!(harmony_engine.current_chord.is_some());
        assert!(!harmony_engine.previous_voicing.is_empty());

        trigger_jam_chord(
            &[],
            120.0,
            &mut jam_voices,
            &mut bass_voice,
            &mut bassline_engine,
            &mut harmony_engine,
            false,
            true,
        );

        assert!(bassline_engine.current_chord.is_none());
        assert!(harmony_engine.current_chord.is_none());
        assert!(harmony_engine.previous_voicing.is_empty());
        assert_eq!(bass_voice.env.state, AdsrState::Release);
        assert!(jam_voices
            .iter()
            .filter(|voice| voice.active)
            .all(|voice| voice.env.state == AdsrState::Release));
    }

    #[test]
    fn silence_voices_forces_idle_even_with_sustain() {
        let mut voices = vec![Voice::new(100.0)];
        voices[0].env.sustain = 1.0;
        voices[0].note_on(60, 127);
        voices[0].env.state = AdsrState::Sustain;
        voices[0].env.value = 1.0;

        silence_voices(&mut voices);

        assert!(!voices[0].active);
        assert_eq!(voices[0].env.state, AdsrState::Idle);
        assert_eq!(voices[0].next_sample(), 0.0);
    }

    #[test]
    fn silence_jam_output_clears_engines_and_forces_voices_idle() {
        let sample_rate = 100.0;
        let mut jam_voices: Vec<Voice> = (0..8).map(|_| Voice::new(sample_rate)).collect();
        let mut bass_voice = Voice::new(sample_rate);
        let mut bassline_engine = BasslineEngine::new(sample_rate, 120.0);
        let mut harmony_engine = HarmonyEngine::new();
        harmony_engine.enabled = true;

        trigger_jam_chord(
            &[60, 64, 67],
            120.0,
            &mut jam_voices,
            &mut bass_voice,
            &mut bassline_engine,
            &mut harmony_engine,
            true,
            true,
        );

        silence_jam_output(
            &mut jam_voices,
            &mut bass_voice,
            &mut bassline_engine,
            &mut harmony_engine,
        );

        assert!(jam_voices.iter().all(|voice| !voice.active));
        assert!(!bass_voice.active);
        assert!(bassline_engine.current_chord.is_none());
        assert!(harmony_engine.current_chord.is_none());
        assert!(harmony_engine.previous_voicing.is_empty());
    }

    #[test]
    fn input_channel_selection_is_stable() {
        let frame = NativeInputFrame::from_f32_frame(&[0.4, 0.2]);

        assert_eq!(
            NativeInputFrame::from_f32_frame(&[0.8, 0.0]).sample_for_mode(1),
            0.8
        );
        assert_eq!(
            NativeInputFrame::from_f32_frame(&[0.0, -0.6]).sample_for_mode(2),
            -0.6
        );
        assert!((frame.sample_for_mode(0) - 0.3).abs() < 0.0001);
        assert_eq!(frame.sample_for_mode(8), 0.0);
    }
}

fn parse_custom_chord(token: &str) -> Option<JamSongChord> {
    let symbol = token
        .trim()
        .trim_matches(|c: char| c == '[' || c == ']' || c == '(' || c == ')');
    if symbol.is_empty() {
        return None;
    }

    let root_end = match symbol.as_bytes().get(1) {
        Some(b'#') | Some(b'b') => 2,
        _ => 1,
    };
    let root = symbol.get(..root_end)?;
    let root_index = note_index(root)?;
    let suffix = symbol[root_end..].to_ascii_lowercase();

    let intervals: Vec<i16> = if suffix.contains("m7b5") || suffix.contains("half") {
        vec![0, 3, 6, 10]
    } else if suffix.contains("dim") {
        vec![0, 3, 6]
    } else if suffix.contains("sus2") {
        vec![0, 2, 7]
    } else if suffix.contains("sus4") || suffix.contains("sus") {
        vec![0, 5, 7]
    } else if suffix.starts_with('m') && !suffix.starts_with("maj") {
        let mut notes = vec![0, 3, 7];
        if suffix.contains('7') || suffix.contains('9') {
            notes.push(10);
        }
        if suffix.contains('9') {
            notes.push(14);
        }
        notes
    } else {
        let mut notes = vec![0, 4, 7];
        if suffix.contains("maj7") {
            notes.push(11);
        } else if suffix.contains('7') {
            notes.push(10);
        }
        if suffix.contains('9') {
            notes.push(14);
        }
        notes
    };

    let base_octave_midi = if root_index >= 5 { 36 } else { 48 };
    let notes = intervals
        .into_iter()
        .map(|interval| (base_octave_midi + root_index as i16 + interval) as u8)
        .collect();

    Some(JamSongChord {
        notes,
        beats: 4,
        label: symbol.to_string(),
    })
}

fn note_index(note: &str) -> Option<u8> {
    match note.to_ascii_uppercase().as_str() {
        "C" | "B#" => Some(0),
        "C#" | "DB" => Some(1),
        "D" => Some(2),
        "D#" | "EB" => Some(3),
        "E" | "FB" => Some(4),
        "F" | "E#" => Some(5),
        "F#" | "GB" => Some(6),
        "G" => Some(7),
        "G#" | "AB" => Some(8),
        "A" => Some(9),
        "A#" | "BB" => Some(10),
        "B" | "CB" => Some(11),
        _ => None,
    }
}

fn release_jam_voices(jam_voices: &mut [Voice], bass_voice: &mut Voice) {
    for voice in jam_voices.iter_mut() {
        if voice.active {
            voice.note_off();
        }
    }
    if bass_voice.active {
        bass_voice.note_off();
    }
}

fn clear_jam_engines(bassline_engine: &mut BasslineEngine, harmony_engine: &mut HarmonyEngine) {
    bassline_engine.stop_chord();
    harmony_engine.current_chord = None;
    harmony_engine.previous_voicing.clear();
}

fn release_jam_output(
    jam_voices: &mut [Voice],
    bass_voice: &mut Voice,
    bassline_engine: &mut BasslineEngine,
    harmony_engine: &mut HarmonyEngine,
) {
    release_jam_voices(jam_voices, bass_voice);
    clear_jam_engines(bassline_engine, harmony_engine);
}

fn silence_jam_output(
    jam_voices: &mut [Voice],
    bass_voice: &mut Voice,
    bassline_engine: &mut BasslineEngine,
    harmony_engine: &mut HarmonyEngine,
) {
    silence_voices(jam_voices);
    bass_voice.silence();
    clear_jam_engines(bassline_engine, harmony_engine);
}

fn trigger_jam_chord(
    notes: &[u8],
    tempo: f32,
    jam_voices: &mut [Voice],
    bass_voice: &mut Voice,
    bassline_engine: &mut BasslineEngine,
    harmony_engine: &mut HarmonyEngine,
    bassline_enabled: bool,
    harmonics_enabled: bool,
) {
    release_jam_voices(jam_voices, bass_voice);

    if notes.is_empty() {
        clear_jam_engines(bassline_engine, harmony_engine);
        return;
    }

    bassline_engine.set_tempo(tempo);
    bassline_engine.new_chord(notes);
    harmony_engine.set_chord(notes);

    if !bassline_enabled {
        let root = notes[0];
        let bass_note = if root > 36 {
            root.saturating_sub(24)
        } else {
            root.saturating_sub(12)
        };
        bass_voice.note_on(bass_note, 90);
    }

    for (i, &note) in notes.iter().enumerate() {
        if i < jam_voices.len() {
            jam_voices[i].note_on(note, 70);
        }
    }

    if harmonics_enabled {
        let voicing_notes = harmony_engine.get_professional_harmony(notes);

        for (i, voicing_note) in voicing_notes.iter().enumerate() {
            let voice_idx = notes.len() + i;
            if voice_idx < jam_voices.len() {
                jam_voices[voice_idx].note_on(voicing_note.note, 40);
            }
        }
    }
}

// Wrapper to make cpal::Stream Send/Sync
pub struct AudioEngine {
    pub _stream: cpal::Stream,
    pub _input_stream: Option<cpal::Stream>,
    pub _event_producer: Arc<Mutex<HeapProducer<AudioThreadEvent>>>,
}

// Safety: We manage the stream lifecycle carefully within the AppState
unsafe impl Send for AudioEngine {}
unsafe impl Sync for AudioEngine {}

impl AudioEngine {
    pub fn new(
        mut command_consumer: HeapConsumer<AudioCommand>,
        event_producer: Arc<Mutex<HeapProducer<AudioThreadEvent>>>,
    ) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device"))?;
        let config = device.default_output_config()?;
        let output_stream_config: cpal::StreamConfig = config.clone().into();
        let sample_rate = output_stream_config.sample_rate.0 as f32;
        let output_channels = output_stream_config.channels.max(1) as usize;
        println!(
            "Audio: Output Device: {} @ {} Hz, {} channel(s), {:?}",
            device.name().unwrap_or("Unknown".into()),
            output_stream_config.sample_rate.0,
            output_channels,
            config.sample_format()
        );

        let event_producer_clone = event_producer.clone();

        // --- INPUT SETUP ---
        let input_rb = HeapRb::<NativeInputFrame>::new((sample_rate as usize).max(8192));
        let (mut input_prod, mut input_cons) = input_rb.split();

        let selected_input_device = host.default_input_device().or_else(|| {
            host.input_devices().ok().and_then(|devices| {
                devices
                    .filter_map(|device| {
                        let channels = device.default_input_config().ok()?.channels();
                        Some((channels, device))
                    })
                    .max_by_key(|(channels, _)| *channels)
                    .map(|(_, device)| device)
            })
        });
        let selected_input_name = selected_input_device
            .as_ref()
            .map(|device| device.name().unwrap_or("Unknown Device".into()))
            .unwrap_or("None".into());

        let input_stream = if let Some(in_device) = selected_input_device {
            println!(
                "Audio: Found Input Device: {}",
                in_device.name().unwrap_or("Unknown".into())
            );
            let default_input_config = in_device.default_input_config()?;
            let output_sample_rate = output_stream_config.sample_rate;
            let selected_input_config = in_device
                .supported_input_configs()
                .ok()
                .and_then(|configs| {
                    configs
                        .filter(|config| {
                            config.min_sample_rate().0 <= output_sample_rate.0
                                && config.max_sample_rate().0 >= output_sample_rate.0
                        })
                        .min_by_key(|config| {
                            let format_rank = input_format_rank(config.sample_format());
                            let channel_penalty =
                                u8::from(config.channels() != default_input_config.channels());
                            let default_penalty = u8::from(
                                config.sample_format() != default_input_config.sample_format(),
                            );
                            (
                                format_rank,
                                channel_penalty,
                                default_penalty,
                                config.channels(),
                            )
                        })
                        .map(|config| config.with_sample_rate(output_sample_rate))
                })
                .unwrap_or_else(|| default_input_config.clone());
            let input_sample_format = selected_input_config.sample_format();
            let input_stream_config: cpal::StreamConfig = selected_input_config.clone().into();
            let input_channels = input_stream_config.channels.max(1) as usize;
            println!(
                "Audio: Input Stream: {} Hz, {} channel(s), {:?} (default {:?})",
                input_stream_config.sample_rate.0,
                input_channels,
                input_sample_format,
                default_input_config.sample_format()
            );
            if input_stream_config.sample_rate.0 != output_stream_config.sample_rate.0 {
                eprintln!(
                    "Audio: Input/output sample-rate mismatch (input {} Hz, output {} Hz); looper capture may drift.",
                    input_stream_config.sample_rate.0, output_stream_config.sample_rate.0
                );
            }

            let stream_result = match input_sample_format {
                SampleFormat::F32 => Some(in_device.build_input_stream(
                    &input_stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        push_native_input_f32(data, input_channels, &mut input_prod);
                    },
                    |err| eprintln!("Input stream error: {}", err),
                    None,
                )),
                SampleFormat::I8 => Some(in_device.build_input_stream(
                    &input_stream_config,
                    move |data: &[i8], _: &cpal::InputCallbackInfo| {
                        push_native_input_i8(data, input_channels, &mut input_prod);
                    },
                    |err| eprintln!("Input stream error: {}", err),
                    None,
                )),
                SampleFormat::I16 => Some(in_device.build_input_stream(
                    &input_stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        push_native_input_i16(data, input_channels, &mut input_prod);
                    },
                    |err| eprintln!("Input stream error: {}", err),
                    None,
                )),
                SampleFormat::U8 => Some(in_device.build_input_stream(
                    &input_stream_config,
                    move |data: &[u8], _: &cpal::InputCallbackInfo| {
                        push_native_input_u8(data, input_channels, &mut input_prod);
                    },
                    |err| eprintln!("Input stream error: {}", err),
                    None,
                )),
                SampleFormat::U16 => Some(in_device.build_input_stream(
                    &input_stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        push_native_input_u16(data, input_channels, &mut input_prod);
                    },
                    |err| eprintln!("Input stream error: {}", err),
                    None,
                )),
                other => {
                    eprintln!("Audio: Unsupported input sample format: {:?}", other);
                    None
                }
            };

            let stream = match stream_result {
                Some(Ok(stream)) => Some(stream),
                Some(Err(err)) => {
                    eprintln!("Audio: Failed to build input stream: {}", err);
                    None
                }
                None => None,
            };

            if let Some(s) = &stream {
                s.play().ok();
            }
            stream
        } else {
            println!("Audio: No Input Device Found.");
            None
        };

        // Initialize Internal State
        let mut voices: Vec<Voice> = (0..8).map(|_| Voice::new(sample_rate)).collect();
        let mut mpc_voices: Vec<Voice> = (0..8).map(|_| Voice::new(sample_rate)).collect();
        let mut jam_voices: Vec<Voice> = (0..8).map(|_| Voice::new(sample_rate)).collect();
        let mut bass_voice = Voice::new(sample_rate); // Dedicated Bass Voice

        // Default Jam Sound: Piano
        for v in &mut jam_voices {
            v.env.attack = 0.01;
            v.env.decay = 0.4;
            v.env.sustain = 0.25;
            v.env.release = 0.4;
            v.filter.cutoff = 3000.0;
        }
        // Default Bass Sound
        bass_voice.osc1.waveform = Waveform::Triangle;
        bass_voice.osc2.waveform = Waveform::Sine;
        bass_voice.env.attack = 0.01;
        bass_voice.env.decay = 0.3;
        bass_voice.env.sustain = 0.8;
        bass_voice.env.release = 0.2;
        bass_voice.filter.cutoff = 400.0; // Low pass for bass

        let mut looper = Looper::new();
        let mut part_fx: [FxChain; LOOPER_PART_COUNT] =
            std::array::from_fn(|_| FxChain::new(sample_rate));

        // Drums
        let mut kick = Kick::new(sample_rate);
        let mut snare = Snare::new(sample_rate);
        let mut hihat = HiHat::new(sample_rate);
        let mut sampler = Sampler::new(sample_rate);
        let mut active_mpc_kit = 0u8;
        let mut mpc_steps = [[false; 16]; 16];
        let mut mpc_playing = false;
        let mut mpc_current_step = 0usize;
        let mut mpc_samples_until_step = 0u64;
        let mut mpc_bpm = 120.0f32;
        let mut mpc_swing = 0.0f32;

        // State Flags
        let mut mic_active = true;
        let mut mic_gain = 1.0;
        let mut selected_part = 0;
        let mut input_monitoring = false;
        let mut monitor_input_mode = 0usize; // 0=mix, 1..8=individual input channels
        let mut looper_part_sources = default_looper_sources();
        let mut tuner_buffer: VecDeque<f32> = VecDeque::with_capacity(2048);
        let mut tuner_emit_counter = 0u64;
        let tuner_emit_interval = (sample_rate / 8.0).max(1024.0) as u64;

        // Jam Station Enhancement Variables
        let mut bassline_enabled = false;
        let mut harmonics_enabled = false;

        // Song Sequence Tracking
        let mut prev_sequence_step: usize = usize::MAX; // sentinel for "no previous step"

        // Metronome State
        let mut metronome_enabled = false;
        let mut metronome_sample_counter = 0u64;
        let mut metronome_samples_per_beat = (sample_rate * 60.0 / 120.0) as u64;
        let mut metronome_current_beat = 0u32;

        // Create the intelligent engines
        let mut bassline_engine = crate::audio::BasslineEngine::new(sample_rate, 120.0); // Default 120 BPM
        let mut harmony_engine = crate::audio::HarmonyEngine::new();
        let mut active_jam_song: Option<JamSong> = None;

        // Send Diagnostic Info
        let input_name = selected_input_name;
        if let Ok(mut prod) = event_producer_clone.lock() {
            let _ = prod.push(AudioThreadEvent::EngineInfo {
                sample_rate,
                input_device: input_name,
            });
        }

        let stream_event_producer = event_producer_clone.clone();

        let stream = device.build_output_stream(
            &output_stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // 1. Process Commands from UI
                while let Some(cmd) = command_consumer.pop() {
                    // println!("ENGINE EXEC: {:?}", cmd);
                    match cmd {
                        AudioCommand::SetMicActive { active } => {
                            mic_active = active;
                            println!("Audio: Mic Active = {}", mic_active);
                        }
                        AudioCommand::SetMicGain { gain } => {
                            mic_gain = gain;
                        }
                        AudioCommand::SetNativeInputChannel { channel } => {
                            let mode = (channel as usize).min(MAX_NATIVE_INPUT_CHANNELS);
                            monitor_input_mode = mode;
                            println!(
                                "Monitor/tuner input channel: {}",
                                input_mode_label(mode)
                            );
                        }
                        AudioCommand::SetLooperPartInput { part_id, channel } => {
                            if part_id < LOOPER_PART_COUNT {
                                let source = legacy_channel_to_looper_source(channel);
                                looper_part_sources[part_id] = source;
                                println!(
                                    "Looper part {} source: {}",
                                    part_id,
                                    looper_source_label(source)
                                );
                            }
                        }
                        AudioCommand::SetLooperPartSource { part_id, source } => {
                            if part_id < LOOPER_PART_COUNT {
                                let source = normalize_looper_source(source);
                                looper_part_sources[part_id] = source;
                                println!(
                                    "Looper part {} source: {}",
                                    part_id,
                                    looper_source_label(source)
                                );
                            }
                        }
                        AudioCommand::SetLooperFx {
                            part_id,
                            effect_id,
                            value,
                        } => {
                            if let Some(fx) = part_fx.get_mut(part_id) {
                                fx.set_param(effect_id, value);
                            }
                        }
                        AudioCommand::ToggleLooperPartActive { part_id } => {
                            if part_id < LOOPER_PART_COUNT {
                                let was_sequence_active = looper.sequence_active;
                                if looper.toggle_part_active(part_id).is_some() {
                                    if let Ok(mut prod) = stream_event_producer.lock() {
                                        if was_sequence_active && !looper.sequence_active {
                                            let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                        }
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id,
                                            state: looper.part_state(part_id).into(),
                                        });
                                    }
                                }
                            }
                        }
                        AudioCommand::Record { part_id } => {
                            if part_id < LOOPER_PART_COUNT {
                                let sequence_was_active = looper.sequence_active;
                                looper.start_recording(part_id);
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    if sequence_was_active {
                                        let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                    }
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id,
                                        state: looper.part_state(part_id).into(),
                                    });
                                }
                            }
                        }
                        AudioCommand::Overdub { part_id } => {
                            if part_id < LOOPER_PART_COUNT {
                                let sequence_was_active = looper.sequence_active;
                                looper.start_overdub(part_id);
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    if sequence_was_active {
                                        let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                    }
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id,
                                        state: looper.part_state(part_id).into(),
                                    });
                                }
                            }
                        }
                        AudioCommand::Undo { part_id } => {
                            if part_id < LOOPER_PART_COUNT {
                                let sequence_was_active = looper.sequence_active;
                                looper.undo_part(part_id);
                                // Re-emit waveform after undo
                                let waveform = looper.parts[part_id].get_waveform(100);
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    if sequence_was_active && !looper.sequence_active {
                                        let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                    }
                                    let _ = prod.push(AudioThreadEvent::WaveformReady {
                                        part_id,
                                        data: waveform,
                                    });
                                    let state = if looper.parts[part_id].has_material() {
                                        "recorded"
                                    } else {
                                        "empty"
                                    };
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id,
                                        state: state.into(),
                                    });
                                    let _ = prod.push(AudioThreadEvent::LooperLayerCount {
                                        part_id,
                                        layers: looper.parts[part_id].layer_count(),
                                    });
                                    let _ = prod.push(AudioThreadEvent::LoopDuration {
                                        samples: looper.master_duration,
                                    });
                                    if !looper.is_any_playing() && !looper.is_any_recording() {
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id,
                                            state: "stopped".into(),
                                        });
                                    }
                                }
                            }
                        }
                        AudioCommand::ClearPart { part_id } => {
                            if part_id < LOOPER_PART_COUNT {
                                let sequence_was_active = looper.sequence_active;
                                looper.clear_part(part_id);
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    if sequence_was_active && !looper.sequence_active {
                                        let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                    }
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id,
                                        state: "empty".into(),
                                    });
                                    let _ = prod.push(AudioThreadEvent::WaveformReady {
                                        part_id,
                                        data: vec![],
                                    });
                                    let _ = prod.push(AudioThreadEvent::LooperLayerCount {
                                        part_id,
                                        layers: 0,
                                    });
                                    let _ = prod.push(AudioThreadEvent::LoopDuration {
                                        samples: looper.master_duration,
                                    });
                                    if !looper.is_any_playing() && !looper.is_any_recording() {
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id,
                                            state: "stopped".into(),
                                        });
                                    }
                                }
                            }
                        }
                        AudioCommand::NoteOn { note, velocity } => {
                            if note >= 36 && note <= 51 {
                                let pad_id = (note - 36) as usize;
                                let sample_triggered = sampler.trigger(pad_id);
                                if !sample_triggered {
                                    trigger_mpc_kit_pad(
                                        pad_id,
                                        active_mpc_kit,
                                        velocity,
                                        &mut kick,
                                        &mut snare,
                                        &mut hihat,
                                        &mut mpc_voices,
                                    );
                                }
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    let _ = prod.push(AudioThreadEvent::DrumTrigger { note });
                                }
                            } else {
                                trigger_synth_voice(&mut voices, note, velocity);
                            }
                        }
                        AudioCommand::UploadSample { pad_id, data } => {
                            sampler.load(pad_id, data);
                        }
                        AudioCommand::NoteOff { note } => {
                            for voice in voices.iter_mut() {
                                if voice.note == note && voice.active {
                                    voice.note_off();
                                }
                            }
                        }
                        AudioCommand::Play => {
                            let started = looper.play_all_recorded();
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                if started {
                                    let _ = prod.push(AudioThreadEvent::LoopDuration {
                                        samples: looper.master_duration,
                                    });
                                    for part_id in 0..LOOPER_PART_COUNT {
                                        if looper.parts[part_id].is_playing {
                                            let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                                part_id,
                                                state: looper.part_state(part_id).into(),
                                            });
                                        }
                                    }
                                } else {
                                    let _ = prod.push(AudioThreadEvent::LoopDuration { samples: 0 });
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id: selected_part,
                                        state: "stopped".into(),
                                    });
                                }
                            }
                        }
                        AudioCommand::TogglePlayback => {
                            let is_playing = looper.parts.iter().any(|p| p.is_playing);
                            if is_playing {
                                // Also stop recording if active
                                for i in 0..LOOPER_PART_COUNT {
                                    let was_recording = looper.parts[i].is_recording;
                                    looper.stop_recording(i);
                                    if was_recording {
                                        let waveform = looper.parts[i].get_waveform(100);
                                        if let Ok(mut prod) = stream_event_producer.lock() {
                                            let _ = prod.push(AudioThreadEvent::WaveformReady {
                                                part_id: i,
                                                data: waveform,
                                            });
                                            let _ = prod.push(AudioThreadEvent::LooperLayerCount {
                                                part_id: i,
                                                layers: looper.parts[i].layer_count(),
                                            });
                                            let _ = prod.push(AudioThreadEvent::LoopDuration {
                                                samples: looper.master_duration,
                                            });
                                        }
                                    }
                                }
                                looper.stop();
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    for part_id in 0..LOOPER_PART_COUNT {
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id,
                                            state: looper.part_state(part_id).into(),
                                        });
                                    }
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id: 0,
                                        state: "stopped".into(),
                                    });
                                }
                            } else {
                                let started = looper.play_all_recorded();
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    if started {
                                        let _ = prod.push(AudioThreadEvent::LoopDuration {
                                            samples: looper.master_duration,
                                        });
                                        for part_id in 0..LOOPER_PART_COUNT {
                                            if looper.parts[part_id].is_playing {
                                                let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                                    part_id,
                                                    state: looper.part_state(part_id).into(),
                                                });
                                            }
                                        }
                                    } else {
                                        let _ = prod.push(AudioThreadEvent::LoopDuration { samples: 0 });
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id: selected_part,
                                            state: "stopped".into(),
                                        });
                                    }
                                }
                            }
                        }
                        AudioCommand::Stop => {
                            stop_mpc_output(
                                &mut sampler,
                                &mut kick,
                                &mut snare,
                                &mut hihat,
                                &mut mpc_voices,
                            );
                            for i in 0..LOOPER_PART_COUNT {
                                let was_recording = looper.parts[i].is_recording;
                                looper.stop_recording(i);

                                if was_recording {
                                    let waveform = looper.parts[i].get_waveform(100);
                                    if let Ok(mut prod) = stream_event_producer.lock() {
                                        let _ = prod.push(AudioThreadEvent::WaveformReady {
                                            part_id: i,
                                            data: waveform,
                                        });
                                        let _ = prod.push(AudioThreadEvent::LooperLayerCount {
                                            part_id: i,
                                            layers: looper.parts[i].layer_count(),
                                        });
                                        let _ = prod.push(AudioThreadEvent::LoopDuration {
                                            samples: looper.master_duration,
                                        });
                                    }
                                }
                            }
                            looper.stop();
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                for part_id in 0..LOOPER_PART_COUNT {
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id,
                                        state: looper.part_state(part_id).into(),
                                    });
                                }
                                let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                    part_id: 0,
                                    state: "stopped".into(),
                                });
                            }
                        }
                        AudioCommand::StopAllSounds => {
                            println!("Audio: Stop All Sounds");
                            silence_voices(&mut voices);
                            stop_mpc_output(
                                &mut sampler,
                                &mut kick,
                                &mut snare,
                                &mut hihat,
                                &mut mpc_voices,
                            );
                            mpc_playing = false;
                            mpc_current_step = 0;
                            mpc_samples_until_step = 0;

                            active_jam_song = None;
                            silence_jam_output(
                                &mut jam_voices,
                                &mut bass_voice,
                                &mut bassline_engine,
                                &mut harmony_engine,
                            );

                            metronome_enabled = false;
                            metronome_sample_counter = 0;
                            metronome_current_beat = 0;

                            let mut finalized_recordings = Vec::new();
                            for i in 0..LOOPER_PART_COUNT {
                                let was_recording = looper.parts[i].is_recording;
                                looper.stop_recording(i);
                                if was_recording {
                                    finalized_recordings.push(i);
                                }
                            }
                            looper.stop();
                            prev_sequence_step = usize::MAX;

                            if let Ok(mut prod) = stream_event_producer.lock() {
                                for part_id in finalized_recordings {
                                    let waveform = looper.parts[part_id].get_waveform(100);
                                    let _ = prod.push(AudioThreadEvent::WaveformReady {
                                        part_id,
                                        data: waveform,
                                    });
                                    let _ = prod.push(AudioThreadEvent::LooperLayerCount {
                                        part_id,
                                        layers: looper.parts[part_id].layer_count(),
                                    });
                                }
                                let _ = prod.push(AudioThreadEvent::LoopDuration {
                                    samples: looper.master_duration,
                                });
                                for part_id in 0..LOOPER_PART_COUNT {
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id,
                                        state: looper.part_state(part_id).into(),
                                    });
                                }
                                let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                let _ = prod.push(AudioThreadEvent::MpcTransport { playing: false });
                                let _ = prod.push(AudioThreadEvent::MpcStep { step: 0 });
                                let _ = prod.push(AudioThreadEvent::JamControl { action: 1 });
                                let _ = prod.push(AudioThreadEvent::AllSoundsStopped);
                            }
                        }
                        AudioCommand::ToggleLooper { part_id } => {
                            if part_id < LOOPER_PART_COUNT {
                                let has_material = looper.parts[part_id].has_material();
                                let is_recording = looper.parts[part_id].is_recording;

                                if !has_material && !is_recording {
                                    // Empty -> Record
                                    let sequence_was_active = looper.sequence_active;
                                    looper.start_recording(part_id);
                                    if let Ok(mut prod) = stream_event_producer.lock() {
                                        if sequence_was_active {
                                            let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                        }
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id,
                                            state: looper.part_state(part_id).into(),
                                        });
                                    }
                                } else if is_recording {
                                    // Recording -> Play (Finish Rec)
                                    looper.stop_recording(part_id);
                                    if let Ok(mut prod) = stream_event_producer.lock() {
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id,
                                            state: looper.part_state(part_id).into(),
                                        });
                                        let waveform = looper.parts[part_id].get_waveform(100);
                                        let _ = prod.push(AudioThreadEvent::WaveformReady {
                                            part_id,
                                            data: waveform,
                                        });
                                        let _ = prod.push(AudioThreadEvent::LooperLayerCount {
                                            part_id,
                                            layers: looper.parts[part_id].layer_count(),
                                        });
                                        let _ = prod.push(AudioThreadEvent::LoopDuration {
                                            samples: looper.master_duration,
                                        });
                                    }
                                } else {
                                    // Recorded -> Overdub, even from stopped transport
                                    let sequence_was_active = looper.sequence_active;
                                    looper.start_overdub(part_id);
                                    if let Ok(mut prod) = stream_event_producer.lock() {
                                        if sequence_was_active {
                                            let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                        }
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id,
                                            state: looper.part_state(part_id).into(),
                                        });
                                    }
                                }
                            }
                        }
                        AudioCommand::SelectPart { part_id } => {
                            if part_id >= LOOPER_PART_COUNT {
                                continue;
                            }

                            let mut finalized_recordings = Vec::new();
                            for i in 0..LOOPER_PART_COUNT {
                                if looper.parts[i].is_recording {
                                    looper.stop_recording(i);
                                    finalized_recordings.push(i);
                                }
                            }

                            selected_part = part_id;
                            let was_playing = !looper.sequence_active && looper.is_any_playing();
                            let finalized_any = !finalized_recordings.is_empty();
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                for i in finalized_recordings {
                                    let waveform = looper.parts[i].get_waveform(100);
                                    let _ = prod.push(AudioThreadEvent::WaveformReady {
                                        part_id: i,
                                        data: waveform,
                                    });
                                    let state = if looper.parts[i].has_material() {
                                        "recorded"
                                    } else {
                                        "empty"
                                    };
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id: i,
                                        state: state.into(),
                                    });
                                    let _ = prod.push(AudioThreadEvent::LooperLayerCount {
                                        part_id: i,
                                        layers: looper.parts[i].layer_count(),
                                    });
                                }
                                if finalized_any || looper.master_duration > 0 {
                                    let _ = prod.push(AudioThreadEvent::LoopDuration {
                                        samples: looper.master_duration,
                                    });
                                }
                                let _ = prod.push(AudioThreadEvent::PartActive { part_id });
                                if was_playing {
                                    let state = if looper.is_any_playing() {
                                        "playing"
                                    } else {
                                        "stopped"
                                    };
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id,
                                        state: state.into(),
                                    });
                                }
                            }
                        }
                        AudioCommand::SetParam { id, value } => {
                            // ONLY affect the SYNTHESIZER voices, NOT jam_voices
                            for voice in &mut voices {
                                match id {
                                    0 => voice.filter.cutoff = 20.0 + (value * value * 10000.0),
                                    1 => voice.filter.resonance = value * 0.95,
                                    2 => voice.env.attack = 0.001 + (value * 2.0),
                                    3 => voice.env.decay = 0.001 + (value * 2.0),
                                    4 => voice.env.sustain = value,
                                    5 => voice.env.release = 0.001 + (value * 5.0),
                                    _ => {}
                                }
                            }
                            if selected_part < LOOPER_PART_COUNT {
                                match id {
                                    6 => part_fx[selected_part].set_param(0, value),
                                    7 => part_fx[selected_part].set_param(1, value),
                                    8 => part_fx[selected_part].set_param(4, value),
                                    9 => part_fx[selected_part].set_param(5, value),
                                    10 => part_fx[selected_part].set_param(7, value),
                                    11 => part_fx[selected_part].set_param(6, value),
                                    // Per-part gains: 12=A, 13=B, 14=C, 15=D, 16=E
                                    12..=16 => {
                                        let part_id = (id - 12) as usize;
                                        if part_id < LOOPER_PART_COUNT {
                                            looper.part_gains[part_id] = value;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::ParamChange { id, value });
                            }
                        }
                        AudioCommand::SetMpcParam { id, value } => {
                            if id == 0 {
                                mpc_swing = value.clamp(0.0, 0.5);
                            }
                            if id == 1 {
                                active_mpc_kit = normalized_mpc_kit_index(value);
                                sampler.clear_all();
                                kick.stop();
                                snare.stop();
                                hihat.stop();
                                for voice in &mut mpc_voices {
                                    voice.silence();
                                }
                            }
                            // Forward to UI
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::MpcParamChange { id, value });
                            }
                        }
                        AudioCommand::SetMpcKit { kit } => {
                            active_mpc_kit = kit.min(3);
                            sampler.clear_all();
                            kick.stop();
                            snare.stop();
                            hihat.stop();
                            for voice in &mut mpc_voices {
                                voice.silence();
                            }
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let value = active_mpc_kit as f32 / 3.0;
                                let _ =
                                    prod.push(AudioThreadEvent::MpcParamChange { id: 1, value });
                            }
                        }
                        AudioCommand::SetMpcStep {
                            pad_id,
                            step,
                            active,
                        } => {
                            if pad_id < 16 && step < 16 {
                                mpc_steps[pad_id][step] = active;
                            }
                        }
                        AudioCommand::SetMpcSampleTrim {
                            pad_id,
                            start,
                            end,
                            volume,
                            pitch,
                        } => {
                            sampler.set_params(pad_id, start, end, volume, pitch);
                        }
                        AudioCommand::StartMpcSequencer { bpm, swing } => {
                            mpc_bpm = bpm.clamp(40.0, 240.0);
                            mpc_swing = swing.clamp(0.0, 0.5);
                            mpc_current_step = 0;
                            mpc_samples_until_step = 0;
                            mpc_playing = true;
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::MpcTransport { playing: true });
                            }
                        }
                        AudioCommand::StopMpcSequencer => {
                            mpc_playing = false;
                            mpc_current_step = 0;
                            mpc_samples_until_step = 0;
                            stop_mpc_output(
                                &mut sampler,
                                &mut kick,
                                &mut snare,
                                &mut hihat,
                                &mut mpc_voices,
                            );
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::MpcTransport { playing: false });
                                let _ = prod.push(AudioThreadEvent::MpcStep { step: 0 });
                            }
                        }
                        AudioCommand::SetJamParam { id, value } => {
                            // ONLY affect jam_voices, NOT synthesizer voices
                            for voice in &mut jam_voices {
                                match id {
                                    0 => voice.filter.cutoff = 20.0 + (value * value * 10000.0),
                                    1 => voice.filter.resonance = value * 0.95,
                                    2 => voice.env.attack = 0.001 + (value * 2.0),
                                    3 => voice.env.decay = 0.001 + (value * 2.0),
                                    4 => voice.env.sustain = value,
                                    5 => voice.env.release = 0.001 + (value * 5.0),
                                    _ => {}
                                }
                            }
                            // Also affect bass voice
                            match id {
                                0 => bass_voice.filter.cutoff = 20.0 + (value * value * 10000.0),
                                1 => bass_voice.filter.resonance = value * 0.95,
                                2 => bass_voice.env.attack = 0.001 + (value * 2.0),
                                3 => bass_voice.env.decay = 0.001 + (value * 2.0),
                                4 => bass_voice.env.sustain = value,
                                5 => bass_voice.env.release = 0.001 + (value * 5.0),
                                _ => {}
                            }
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::JamParamChange { id, value });
                            }
                        }
                        AudioCommand::SaveProject { path: _ } => {
                            let buffers = looper
                                .parts
                                .iter()
                                .map(|part| part.buffer.clone())
                                .collect();
                            let layers = looper
                                .parts
                                .iter()
                                .map(|part| part.layers.clone())
                                .collect();
                            let sources = looper_part_sources.to_vec();
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::ProjectSnapshot {
                                    buffers,
                                    layers,
                                    sources,
                                    sample_rate: sample_rate.round() as u32,
                                });
                            }
                        }
                        AudioCommand::LoadPartBuffer { part_id, data } => {
                            if part_id < LOOPER_PART_COUNT {
                                looper.load_part_buffer(part_id, data);
                            }
                        }
                        AudioCommand::LoadPartLayers { part_id, layers } => {
                            if part_id < LOOPER_PART_COUNT {
                                looper.load_part_layers(part_id, layers);
                            }
                        }
                        AudioCommand::LoadProjectDone { all_empty } => {
                            looper.recalculate_master_duration();
                            // Emit waveforms for all parts
                            for i in 0..LOOPER_PART_COUNT {
                                let waveform = looper.parts[i].get_waveform(100);
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    let _ = prod.push(AudioThreadEvent::WaveformReady {
                                        part_id: i,
                                        data: waveform,
                                    });
                                    let state = if looper.parts[i].has_material() {
                                        "recorded"
                                    } else {
                                        "empty"
                                    };
                                    let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                        part_id: i,
                                        state: state.into(),
                                    });
                                    let _ = prod.push(AudioThreadEvent::LooperLayerCount {
                                        part_id: i,
                                        layers: looper.parts[i].layer_count(),
                                    });
                                }
                            }
                            if !all_empty && looper.master_duration > 0 {
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    let _ = prod.push(AudioThreadEvent::LoopDuration {
                                        samples: looper.master_duration,
                                    });
                                }
                            } else if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::LoopDuration { samples: 0 });
                            }
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::ProjectLoaded {
                                    all_empty,
                                    samples: looper.master_duration,
                                });
                            }
                            println!(
                                "LoadProject complete. Master duration: {} samples",
                                looper.master_duration
                            );
                        }
                        AudioCommand::SetMetronome { enabled, bpm } => {
                            metronome_enabled = enabled;
                            let safe_bpm = if bpm.is_finite() {
                                bpm.clamp(40.0, 240.0)
                            } else {
                                120.0
                            };
                            metronome_samples_per_beat = (sample_rate * 60.0 / safe_bpm) as u64;
                            metronome_sample_counter = 0;
                            metronome_current_beat = 0;
                            if enabled {
                                println!("Metronome enabled at {} BPM", safe_bpm);
                            } else {
                                println!("Metronome disabled");
                            }
                        }
                        AudioCommand::PlayChord { notes, tempo } => {
                            active_jam_song = None;
                            trigger_jam_chord(
                                &notes,
                                tempo,
                                &mut jam_voices,
                                &mut bass_voice,
                                &mut bassline_engine,
                                &mut harmony_engine,
                                bassline_enabled,
                                harmonics_enabled,
                            );
                        }
                        AudioCommand::PlayJamTrack { chords, tempo } => {
                            active_jam_song = JamSong::from_chords(chords, tempo, sample_rate);

                            if let Some(song) = active_jam_song.as_ref() {
                                let (index, notes, label) = song.current_step();
                                let tempo = song.tempo;
                                trigger_jam_chord(
                                    &notes,
                                    tempo,
                                    &mut jam_voices,
                                    &mut bass_voice,
                                    &mut bassline_engine,
                                    &mut harmony_engine,
                                    bassline_enabled,
                                    harmonics_enabled,
                                );

                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    let _ = prod.push(AudioThreadEvent::JamChordStep {
                                        index,
                                        notes,
                                        label,
                                    });
                                }
                            } else {
                                trigger_jam_chord(
                                    &[],
                                    tempo,
                                    &mut jam_voices,
                                    &mut bass_voice,
                                    &mut bassline_engine,
                                    &mut harmony_engine,
                                    bassline_enabled,
                                    harmonics_enabled,
                                );
                            }
                        }
                        AudioCommand::StopChord => {
                            active_jam_song = None;
                            release_jam_output(
                                &mut jam_voices,
                                &mut bass_voice,
                                &mut bassline_engine,
                                &mut harmony_engine,
                            );
                        }
                        AudioCommand::SetJamSound { preset_id } => {
                            for v in &mut jam_voices {
                                match preset_id {
                                    0 => {
                                        // Grand Piano
                                        v.osc1.waveform = Waveform::Triangle;
                                        v.osc2.waveform = Waveform::Sine;
                                        v.env.attack = 0.004;
                                        v.env.decay = 0.55;
                                        v.env.sustain = 0.22;
                                        v.env.release = 0.45;
                                        v.filter.cutoff = 5200.0;
                                        v.filter.resonance = 0.05;

                                        // Bass: Upright
                                        bass_voice.osc1.waveform = Waveform::Sine;
                                        bass_voice.osc2.waveform = Waveform::Triangle;
                                        bass_voice.env.attack = 0.01;
                                        bass_voice.env.decay = 0.35;
                                        bass_voice.env.sustain = 0.72;
                                        bass_voice.env.release = 0.6;
                                        bass_voice.filter.cutoff = 420.0;
                                    }
                                    1 => {
                                        // E-Piano
                                        v.osc1.waveform = Waveform::Sine;
                                        v.osc2.waveform = Waveform::Triangle;
                                        v.env.attack = 0.006;
                                        v.env.decay = 1.0;
                                        v.env.sustain = 0.28;
                                        v.env.release = 0.9;
                                        v.filter.cutoff = 2800.0;
                                        v.filter.resonance = 0.16;

                                        // Bass: Smooth
                                        bass_voice.osc1.waveform = Waveform::Triangle;
                                        bass_voice.osc2.waveform = Waveform::Sine;
                                        bass_voice.env.attack = 0.006;
                                        bass_voice.env.decay = 0.25;
                                        bass_voice.env.sustain = 0.72;
                                        bass_voice.env.release = 0.3;
                                        bass_voice.filter.cutoff = 650.0;
                                    }
                                    2 => {
                                        // Organ
                                        v.osc1.waveform = Waveform::Triangle;
                                        v.osc2.waveform = Waveform::Square;
                                        v.env.attack = 0.003;
                                        v.env.decay = 0.08;
                                        v.env.sustain = 0.85;
                                        v.env.release = 0.18;
                                        v.filter.cutoff = 6200.0;
                                        v.filter.resonance = 0.08;

                                        // Bass: Organ Pedal
                                        bass_voice.osc1.waveform = Waveform::Triangle;
                                        bass_voice.osc2.waveform = Waveform::Square;
                                        bass_voice.env.attack = 0.004;
                                        bass_voice.env.decay = 0.08;
                                        bass_voice.env.sustain = 0.8;
                                        bass_voice.env.release = 0.18;
                                        bass_voice.filter.cutoff = 520.0;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        AudioCommand::JamControl { action } => {
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::JamControl { action });
                            }
                        }
                        AudioCommand::SetBasslineEnabled { enabled } => {
                            bassline_enabled = enabled;
                            println!("Bassline enabled: {}", enabled);
                        }
                        AudioCommand::SetHarmonicsEnabled { enabled } => {
                            harmonics_enabled = enabled;
                            harmony_engine.enabled = enabled;
                            println!("Harmonics enabled: {}", enabled);
                        }
                        AudioCommand::SetBasslineStyle { style } => {
                            let pattern = match style {
                                0 => crate::audio::BasslinePattern::Root,
                                1 => crate::audio::BasslinePattern::Octave,
                                2 => crate::audio::BasslinePattern::Walking,
                                3 => crate::audio::BasslinePattern::Rhythmic,
                                _ => crate::audio::BasslinePattern::Root,
                            };
                            bassline_engine.set_pattern(pattern);
                            println!(
                                "Bassline style: {}",
                                match style {
                                    0 => "Root",
                                    1 => "Octave",
                                    2 => "Walking",
                                    3 => "Rhythmic",
                                    _ => "Unknown",
                                }
                            );
                        }
                        AudioCommand::SetBasslinePreset { preset_id } => {
                            // Enhanced Professional Bass Synthesis
                            match preset_id {
                                0 => {
                                    // Upright Acoustic Bass
                                    // Warm, organic acoustic bass sound
                                    bass_voice.osc1.waveform = Waveform::Sine; // Fundamental
                                    bass_voice.osc2.waveform = Waveform::Triangle; // Harmonic warmth

                                    // Smooth acoustic attack with body resonance
                                    bass_voice.env.attack = 0.02; // Slightly slower attack for bow/pluck
                                    bass_voice.env.decay = 0.4; // Medium decay for body resonance
                                    bass_voice.env.sustain = 0.65; // Good sustain for acoustic body
                                    bass_voice.env.release = 0.8; // Long release for natural decay

                                    // Warm low-pass filtering simulating acoustic body
                                    bass_voice.filter.cutoff = 350.0; // Darker, warmer
                                    bass_voice.filter.resonance = 0.2; // Slight body resonance
                                }
                                1 => {
                                    // Electric Bass (Fender-style)
                                    // Punchy electric bass with midrange presence
                                    bass_voice.osc1.waveform = Waveform::Saw; // Bright harmonics
                                    bass_voice.osc2.waveform = Waveform::Square; // Punch and growl

                                    // Quick electric bass attack with punch
                                    bass_voice.env.attack = 0.003; // Very fast attack for pick attack
                                    bass_voice.env.decay = 0.2; // Quick decay for punch
                                    bass_voice.env.sustain = 0.75; // Good sustain for electric
                                    bass_voice.env.release = 0.4; // Medium release

                                    // Brighter filtering with midrange presence
                                    bass_voice.filter.cutoff = 800.0; // Brighter than acoustic
                                    bass_voice.filter.resonance = 0.35; // More aggressive resonance
                                }
                                2 => {
                                    // Modern Synth Bass
                                    // Aggressive modern synth bass with complex harmonics
                                    bass_voice.osc1.waveform = Waveform::Saw; // Bright sawtooth
                                    bass_voice.osc2.waveform = Waveform::Square; // Aggressive square wave

                                    // Punchy synth attack with filter envelope
                                    bass_voice.env.attack = 0.001; // Instant attack for synth punch
                                    bass_voice.env.decay = 0.15; // Fast decay for transient
                                    bass_voice.env.sustain = 0.6; // Medium sustain
                                    bass_voice.env.release = 0.25; // Quick release for tightness

                                    // Dynamic filtering with more resonance for character
                                    bass_voice.filter.cutoff = 1200.0; // Brighter and more present
                                    bass_voice.filter.resonance = 0.5; // High resonance for synth character
                                }
                                _ => {}
                            }
                            println!(
                                "Professional bassline preset: {}",
                                match preset_id {
                                    0 => "Upright Acoustic Bass",
                                    1 => "Electric Bass (Fender-style)",
                                    2 => "Modern Synth Bass",
                                    _ => "Unknown",
                                }
                            );
                        }
                        AudioCommand::SetHarmonicsPreset { preset_id } => {
                            harmony_engine.voicing_type = preset_id;
                            println!(
                                "Harmonics voicing: {}",
                                match preset_id {
                                    0 => "Close Position (Classical)",
                                    1 => "Open Position (Contemporary)",
                                    2 => "Drop-2 (Jazz)",
                                    3 => "Quartal (Modern)",
                                    4 => "Extensions (Advanced Jazz)",
                                    _ => "Close Position",
                                }
                            );
                        }
                        AudioCommand::PlayCustomSong { parts, tempo } => {
                            active_jam_song = JamSong::from_parts(parts.clone(), tempo, sample_rate);

                            if let Some(song) = active_jam_song.as_ref() {
                                let (index, notes, label) = song.current_step();
                                let tempo = song.tempo;
                                trigger_jam_chord(
                                    &notes,
                                    tempo,
                                    &mut jam_voices,
                                    &mut bass_voice,
                                    &mut bassline_engine,
                                    &mut harmony_engine,
                                    bassline_enabled,
                                    harmonics_enabled,
                                );
                                println!(
                                    "Custom song playback started at {} BPM with {} chords",
                                    tempo,
                                    song.chords.len()
                                );
                                if let Ok(mut prod) = stream_event_producer.lock() {
                                    let _ = prod.push(AudioThreadEvent::JamChordStep {
                                        index,
                                        notes,
                                        label,
                                    });
                                }
                            } else {
                                trigger_jam_chord(
                                    &[],
                                    tempo,
                                    &mut jam_voices,
                                    &mut bass_voice,
                                    &mut bassline_engine,
                                    &mut harmony_engine,
                                    bassline_enabled,
                                    harmonics_enabled,
                                );
                                println!(
                                    "Custom song playback ignored at {} BPM: no valid chords in {:?}",
                                    tempo, parts
                                );
                            }
                        }
                        AudioCommand::PlaySequence { parts } => {
                            let started = looper.start_sequence(parts);
                            prev_sequence_step = if started {
                                looper.sequence_step
                            } else {
                                usize::MAX
                            };
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                if started {
                                    if let Some(part_id) = looper.current_sequence_part() {
                                        let _ = prod.push(AudioThreadEvent::SequenceStep {
                                            step: 0,
                                            part_id,
                                        });
                                        let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                            part_id,
                                            state: "playing".into(),
                                        });
                                    }
                                } else {
                                    let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                }
                            }
                        }
                        AudioCommand::StopSequence => {
                            looper.stop_sequence();
                            prev_sequence_step = usize::MAX;
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::SequenceFinished);
                                let _ = prod.push(AudioThreadEvent::LooperStateChange {
                                    part_id: 0,
                                    state: "stopped".into(),
                                });
                            }
                        }
                        AudioCommand::SetInputMonitoring { active } => {
                            input_monitoring = active;
                            println!("Input Monitoring: {}", active);
                        }
                        _ => {}
                    }
                }

                // --- Song Sequence Step Advance ---
                // Check if looper advanced to a new sequence step (do this before processing)
                if looper.sequence_active {
                    let current_step = looper.sequence_step;
                    if current_step != prev_sequence_step {
                        let previous_step = prev_sequence_step;
                        prev_sequence_step = current_step;
                        // Stop previous part, start current part
                        if previous_step != usize::MAX {
                            // Stop all parts first
                            for p in &mut looper.parts {
                                p.is_playing = false;
                                p.play_ptr = 0;
                            }
                        }
                        // Start the new step's part
                        if let Some(&part_id) = looper.sequence.get(current_step) {
                            if part_id < LOOPER_PART_COUNT && looper.parts[part_id].has_material() {
                                looper.parts[part_id].is_playing = true;
                                looper.parts[part_id].play_ptr = 0;
                            }
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::SequenceStep {
                                    step: current_step,
                                    part_id,
                                });
                            }
                        }
                    }
                } else {
                    prev_sequence_step = usize::MAX; // Reset sentinel
                }

                // 2. Audio Processing Loop
                for frame in data.chunks_mut(output_channels) {
                    if mpc_playing {
                        if mpc_samples_until_step == 0 {
                            if let Ok(mut prod) = stream_event_producer.lock() {
                                let _ = prod.push(AudioThreadEvent::MpcStep {
                                    step: mpc_current_step,
                                });

                                for pad_id in 0..16 {
                                    if mpc_steps[pad_id][mpc_current_step] {
                                        let note = 36 + pad_id as u8;
                                        let sample_triggered = sampler.trigger(pad_id);
                                        if !sample_triggered {
                                            trigger_mpc_kit_pad(
                                                pad_id,
                                                active_mpc_kit,
                                                100,
                                                &mut kick,
                                                &mut snare,
                                                &mut hihat,
                                                &mut mpc_voices,
                                            );
                                        }
                                        let _ = prod.push(AudioThreadEvent::DrumTrigger { note });
                                    }
                                }
                            }

                            mpc_samples_until_step = mpc_step_interval_samples(
                                mpc_current_step,
                                mpc_bpm,
                                mpc_swing,
                                sample_rate,
                            );
                            mpc_current_step = (mpc_current_step + 1) % 16;
                        }
                        mpc_samples_until_step = mpc_samples_until_step.saturating_sub(1);
                    }

                    let jam_chord_change = active_jam_song
                        .as_mut()
                        .and_then(|song| {
                            song.advance_frame()
                                .map(|(index, notes, label)| (index, notes, label, song.tempo))
                        });

                    if let Some((index, notes, label, tempo)) = jam_chord_change {
                        trigger_jam_chord(
                            &notes,
                            tempo,
                            &mut jam_voices,
                            &mut bass_voice,
                            &mut bassline_engine,
                            &mut harmony_engine,
                            bassline_enabled,
                            harmonics_enabled,
                        );
                        if let Ok(mut prod) = stream_event_producer.lock() {
                            let _ = prod.push(AudioThreadEvent::JamChordStep {
                                index,
                                notes,
                                label,
                            });
                        }
                    }

                    // Process intelligent bassline (only if enabled)
                    if bassline_enabled {
                        bassline_engine.process_sample(&mut bass_voice);
                    }

                    let mut synth_bus = 0.0;

                    // Sum Voices (Lead)
                    for voice in &mut voices {
                        synth_bus += voice.next_sample();
                    }

                    let mut mpc_bus = 0.0;
                    for voice in &mut mpc_voices {
                        mpc_bus += voice.next_sample();
                    }

                    // Sum Jam Voices (Backing + Bass)
                    let mut jam_bus = 0.0;
                    for voice in &mut jam_voices {
                        jam_bus += voice.next_sample() * 0.6;
                    }
                    jam_bus += bass_voice.next_sample() * 0.8; // Bass slightly louder

                    // Sum Drums
                    mpc_bus += kick.process();
                    mpc_bus += snare.process();
                    mpc_bus += hihat.process();
                    mpc_bus += sampler.process();

                    // Retrieve native inputs once per audio frame. The tuner/monitor read the
                    // selected monitor channel, while each looper part can record a different
                    // interface input channel.
                    let input_frame = input_cons.pop().unwrap_or_default();
                    let native_input = input_frame.sample_for_mode(monitor_input_mode);

                    let mut mic_in = 0.0;
                    if mic_active {
                        mic_in = (native_input * mic_gain).clamp(-1.0, 1.0);
                    }

                    if tuner_buffer.len() == tuner_buffer.capacity() {
                        tuner_buffer.pop_front();
                    }
                    tuner_buffer.push_back(native_input);
                    tuner_emit_counter = tuner_emit_counter.saturating_add(1);

                    if tuner_emit_counter >= tuner_emit_interval
                        && tuner_buffer.len() == tuner_buffer.capacity()
                    {
                        tuner_emit_counter = 0;
                        let samples: Vec<f32> = tuner_buffer.iter().copied().collect();
                        let reading_rms = rms(&samples);
                        let frequency =
                            detect_tuner_frequency(&samples, sample_rate).unwrap_or(0.0);
                        if let Ok(mut prod) = stream_event_producer.lock() {
                            let _ = prod.push(AudioThreadEvent::TunerReading {
                                frequency,
                                rms: reading_rms,
                            });
                        }
                    }

                    let buses = AudioBuses {
                        native_input: input_frame,
                        synth: synth_bus,
                        mpc: mpc_bus,
                        jam: jam_bus,
                    };

                    // Looper capture is routed per part. The global MIC toggle controls monitoring,
                    // but it must not create silent recorded loops.
                    let mut looper_record_inputs = [0.0f32; LOOPER_PART_COUNT];
                    for i in 0..LOOPER_PART_COUNT {
                        looper_record_inputs[i] =
                            sample_for_looper_source(looper_part_sources[i], buses);
                    }
                    let per_part = looper.process(looper_record_inputs);

                    // Per-Part FX Processing
                    let mut looper_playback_bus = 0.0f32;
                    for i in 0..LOOPER_PART_COUNT {
                        if per_part[i].abs() > 0.0 {
                            looper_playback_bus += part_fx[i].process(per_part[i]);
                        }
                    }

                    // Metronome Synthesis — only when looper is actively playing
                    let mut metronome_sample = 0.0f32;
                    let looper_active = looper.is_any_playing() || looper.is_any_recording();
                    if metronome_enabled && looper_active {
                        let samples_per_beat = metronome_samples_per_beat;
                        let is_beat_one = metronome_current_beat == 0;

                        // Generate click: sine wave burst with fast decay
                        let click_freq = if is_beat_one { 1200.0 } else { 800.0 };
                        let click_amplitude = if is_beat_one { 0.4 } else { 0.2 };

                        let phase_increment = click_freq / sample_rate;
                        let click_duration_samples = (sample_rate * 0.03) as u64; // 30ms click
                        let beat_phase = metronome_sample_counter % samples_per_beat;

                        if beat_phase < click_duration_samples {
                            let t = beat_phase as f32 / click_duration_samples as f32;
                            let envelope = if t < 0.1 {
                                1.0
                            } else {
                                1.0 - ((t - 0.1) / 0.9)
                            };
                            let phase = (metronome_sample_counter as f32 * phase_increment)
                                * 2.0
                                * std::f32::consts::PI;
                            let sine_val = phase.sin();
                            metronome_sample = sine_val * click_amplitude * envelope.max(0.0);
                        }

                        metronome_sample_counter += 1;
                        if metronome_sample_counter % samples_per_beat == 0 {
                            metronome_current_beat = (metronome_current_beat + 1) % 4;
                        }
                    }

                    // Input monitoring: mix mic to output so user can hear themselves
                    let monitoring_mix = if input_monitoring { mic_in } else { 0.0 };

                    // Final mix: direct instruments, recorded loops, metronome, and optional monitoring.
                    let mixed = synth_bus
                        + mpc_bus
                        + jam_bus
                        + looper_playback_bus
                        + metronome_sample
                        + monitoring_mix;

                    let mut out = mixed.clamp(-1.0, 1.0);
                    if !out.is_finite() {
                        out = 0.0;
                    } // NaN Protection

                    for sample in frame {
                        *sample = out;
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(Self {
            _stream: stream,
            _input_stream: input_stream,
            _event_producer: event_producer_clone,
        })
    }
}
