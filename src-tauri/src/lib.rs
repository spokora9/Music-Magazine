// SHED POWER: Rust Audio Engine Entry Point
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod audio_engine;
mod events;
mod persistence;

use crate::audio::{
    AudioCommand, AudioThreadEvent, DeviceManager, LooperSource, MidiHandler, MidiMap, MidiTarget,
    LOOPER_PART_COUNT, LOOPER_PART_NAMES,
};
use crate::audio_engine::AudioEngine;
use crate::events::WaveformPayload;
use crate::persistence::{AppPersistence, PracticeSession, ProjectRecord, SavedLesson, SavedSpark};
use ringbuf::HeapRb;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{Emitter, State};

const PROJECT_SCHEMA_VERSION: u32 = 2;
const LEGACY_PROJECT_SCHEMA_VERSION: u32 = 1;
const PROJECT_APP: &str = "shed-power";
const PROJECT_KIND: &str = "looper_project";
const PROJECT_MANIFEST_FILE: &str = "shed_project.json";

// --- APP STATE ---
struct AppState {
    command_producer: Arc<Mutex<ringbuf::HeapProducer<AudioCommand>>>,
    event_producer: Arc<Mutex<ringbuf::HeapProducer<AudioThreadEvent>>>,
    pending_save_path: Arc<Mutex<Option<String>>>,
    midi_map: Arc<Mutex<MidiMap>>,
    midi_active: Arc<AtomicBool>,
    _engine: AudioEngine,
    _midi: MidiHandler,
    device_manager: DeviceManager,
}

// --- TAURI COMMANDS (Frontend Callable) ---

#[tauri::command]
fn set_midi_active(active: bool, state: State<'_, AppState>) {
    state.midi_active.store(active, Ordering::Relaxed);
    let _ = persistence::update_app_persistence(|data| {
        data.settings.midi_active = active;
    });
    if let Ok(mut prod) = state.event_producer.lock() {
        let _ = prod.push(AudioThreadEvent::MidiActive { active });
    }
    println!("MIDI Input Active: {}", active);
}

#[tauri::command]
fn start_midi_learn(target_type: String, id: u8, state: State<'_, AppState>) {
    let target = match target_type.as_str() {
        "param" => Some(MidiTarget::Param(id)),
        "transport" => Some(MidiTarget::Transport(id)),
        "looper_rec" => Some(MidiTarget::LooperRecord(id as usize)),
        "looper_overdub" => Some(MidiTarget::LooperOverdub(id as usize)),
        "looper_toggle" => Some(MidiTarget::LooperToggle(id as usize)),
        "looper_select" => Some(MidiTarget::LooperSelect(id as usize)),
        "looper_undo" => Some(MidiTarget::LooperUndo(id as usize)),
        "looper_clear" => Some(MidiTarget::LooperClear(id as usize)),
        "looper_active" | "looper_active_toggle" => {
            Some(MidiTarget::LooperActiveToggle(id as usize))
        }
        "mic_gain" => Some(MidiTarget::MicGain),
        "note" => Some(MidiTarget::Note(id)),
        "mpc_param" => Some(MidiTarget::MpcParam(id)),
        "jam_play" => Some(MidiTarget::JamPlay),
        "jam_stop" => Some(MidiTarget::JamStop),
        "jam_next" => Some(MidiTarget::JamNext),
        "jam_prev" => Some(MidiTarget::JamPrev),
        other => parse_looper_source_learn_target(other, id),
    };

    if let Some(t) = target {
        if let Ok(mut lock) = state._midi.learning_target.lock() {
            *lock = Some(t);
            println!("MIDI LEARN ARMED: Waiting for signal...");
        }
    }
}

fn parse_looper_source_learn_target(target_type: &str, part_id: u8) -> Option<MidiTarget> {
    let suffix = target_type
        .strip_prefix("looper_source_")
        .or_else(|| target_type.strip_prefix("looper_input_"))?;

    let source = match suffix {
        "mix" | "input_mix" => LooperSource::InputMix,
        "synth" => LooperSource::Synth,
        "mpc" => LooperSource::Mpc,
        "jam" => LooperSource::Jam,
        "instrument" | "instrument_mix" => LooperSource::InstrumentMix,
        "silent" => LooperSource::Silent,
        _ => LooperSource::InputChannel(suffix.parse::<u8>().ok()?.min(8)),
    };

    Some(MidiTarget::LooperSource {
        part_id: part_id as usize,
        source,
    })
}

fn legacy_channel_to_looper_source(channel: u8) -> LooperSource {
    if channel == 0 {
        LooperSource::InputMix
    } else {
        LooperSource::InputChannel(channel.min(8))
    }
}

fn persist_looper_source(part_id: usize, source: LooperSource) {
    let _ = persistence::update_app_persistence(|data| {
        data.settings.looper_sources =
            persistence::normalize_looper_sources(&data.settings.looper_sources);
        if let Some(slot) = data.settings.looper_sources.get_mut(part_id) {
            *slot = source;
        }
    });
}

fn persist_looper_sources(sources: &[Option<LooperSource>]) {
    let _ = persistence::update_app_persistence(|data| {
        data.settings.looper_sources =
            persistence::normalize_looper_sources(&data.settings.looper_sources);
        for (part_id, source) in sources.iter().enumerate().take(LOOPER_PART_COUNT) {
            if let Some(source) = source {
                data.settings.looper_sources[part_id] = *source;
            }
        }
    });
}

#[tauri::command]
fn set_midi_context(ctx: String, state: State<'_, AppState>) {
    if let Ok(mut map) = state.midi_map.lock() {
        map.active_context = ctx.clone();
        let snapshot = map.clone();
        let _ = persistence::update_app_persistence(|data| {
            data.midi_map = snapshot;
            data.settings.active_module = ctx.clone();
        });
        println!("MIDI Context Switched to: {}", ctx);
    }
}

#[tauri::command]
fn get_persistence() -> AppPersistence {
    persistence::load_app_persistence()
}

#[tauri::command]
fn get_persistence_path() -> String {
    persistence::app_state_path().to_string_lossy().to_string()
}

#[tauri::command]
fn export_persistence(path: String) -> Result<(), String> {
    let data = persistence::load_app_persistence();
    let json = serde_json::to_string_pretty(&data)
        .map_err(|err| format!("Failed to serialize app persistence: {}", err))?;
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create export directory {:?}: {}", parent, err))?;
    }
    fs::write(&path, json)
        .map_err(|err| format!("Failed to export persistence to {:?}: {}", path, err))
}

#[tauri::command]
fn import_persistence_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<AppPersistence, String> {
    let path = PathBuf::from(path);
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read persistence import {:?}: {}", path, err))?;
    let mut data: AppPersistence = serde_json::from_str(&contents)
        .map_err(|err| format!("Failed to parse persistence import {:?}: {}", path, err))?;
    data.settings.looper_sources =
        persistence::normalize_looper_sources(&data.settings.looper_sources);
    persistence::validate_app_persistence(&data)?;

    persistence::save_app_persistence(&data)?;

    if let Ok(mut midi_map) = state.midi_map.lock() {
        *midi_map = data.midi_map.clone();
    }
    state
        .midi_active
        .store(data.settings.midi_active, Ordering::Relaxed);

    Ok(data)
}

fn build_sample_waveform(samples: &[f32], buckets: usize) -> Vec<f32> {
    if samples.is_empty() || buckets == 0 {
        return vec![];
    }

    let bucket_size = (samples.len() as f32 / buckets as f32).ceil() as usize;
    samples
        .chunks(bucket_size.max(1))
        .take(buckets)
        .map(|chunk| {
            chunk
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
                .clamp(0.0, 1.0)
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct AppSettingsUpdate {
    mic_active: Option<bool>,
    midi_active: Option<bool>,
    active_module: Option<String>,
    metronome_enabled: Option<bool>,
    metronome_bpm: Option<f32>,
    looper_sources: Option<Vec<LooperSource>>,
}

fn apply_app_settings_update(data: &mut AppPersistence, settings: AppSettingsUpdate) {
    if let Some(mic_active) = settings.mic_active {
        data.settings.mic_active = mic_active;
    }
    if let Some(midi_active) = settings.midi_active {
        data.settings.midi_active = midi_active;
    }
    if let Some(active_module) = settings.active_module {
        data.settings.active_module = active_module;
    }
    if let Some(metronome_enabled) = settings.metronome_enabled {
        data.settings.metronome_enabled = metronome_enabled;
    }
    if let Some(metronome_bpm) = settings.metronome_bpm {
        data.settings.metronome_bpm = metronome_bpm;
    }

    data.settings.looper_sources = match settings.looper_sources {
        Some(looper_sources) => persistence::normalize_looper_sources(&looper_sources),
        None => persistence::normalize_looper_sources(&data.settings.looper_sources),
    };
}

#[tauri::command]
fn save_app_settings(settings: AppSettingsUpdate) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        apply_app_settings_update(data, settings);
    })
}

#[tauri::command]
fn save_module_state(module: String, state: serde_json::Value) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        data.module_state.insert(module, state);
    })
}

#[tauri::command]
fn save_spark(spark: SavedSpark) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        data.saved_sparks.retain(|existing| existing.id != spark.id);
        data.saved_sparks.insert(0, spark);
    })
}

#[tauri::command]
fn delete_spark(id: String) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        data.saved_sparks.retain(|spark| spark.id != id);
    })
}

#[tauri::command]
fn save_lesson(lesson: SavedLesson) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        data.saved_lessons
            .retain(|existing| existing.id != lesson.id);
        data.saved_lessons.insert(0, lesson);
    })
}

#[tauri::command]
fn delete_lesson(id: String) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        data.saved_lessons.retain(|lesson| lesson.id != id);
        if data
            .practice_state
            .active_session
            .as_ref()
            .map(|session| session.lesson_id.as_str())
            == Some(id.as_str())
        {
            data.practice_state.active_session = None;
        }
    })
}

#[tauri::command]
fn start_practice_session(
    lesson: SavedLesson,
    started_at: String,
) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        let session = PracticeSession {
            lesson_id: lesson.id.clone(),
            title: lesson.title.clone(),
            started_at,
            completed_at: None,
            duration: lesson.duration,
        };
        data.saved_lessons
            .retain(|existing| existing.id != lesson.id);
        data.saved_lessons.insert(0, lesson);
        data.practice_state.active_session = Some(session);
    })
}

#[tauri::command]
fn finish_practice_session(
    lesson_id: String,
    completed_at: String,
) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        if let Some(mut session) = data.practice_state.active_session.take() {
            if session.lesson_id == lesson_id {
                session.completed_at = Some(completed_at);
                data.practice_state.sessions.insert(0, session);
            } else {
                data.practice_state.active_session = Some(session);
            }
        }
    })
}

#[tauri::command]
fn import_legacy_browser_state(payload: serde_json::Value) -> Result<AppPersistence, String> {
    persistence::update_app_persistence(|data| {
        if let Some(midi_map) = payload
            .get("shed_midi_map")
            .and_then(|value| value.as_object())
        {
            for (key, value) in midi_map {
                let Some(target_name) = value.as_str() else {
                    continue;
                };
                let Some((context, kind, number)) = parse_legacy_midi_key(key) else {
                    continue;
                };
                let Some(target) = legacy_midi_target(target_name) else {
                    continue;
                };
                if kind == "cc" {
                    data.midi_map
                        .cc_maps
                        .entry(context)
                        .or_default()
                        .insert(number, target);
                } else if kind == "note" {
                    data.midi_map
                        .note_maps
                        .entry(context)
                        .or_default()
                        .insert(number, target);
                }
            }
        }

        if let Some(sparks) = payload
            .get("theshed_sparks")
            .and_then(|value| value.as_array())
        {
            for spark in sparks {
                let id = legacy_id(spark, "legacy-spark", data.saved_sparks.len());
                if data.saved_sparks.iter().any(|existing| existing.id == id) {
                    continue;
                }
                let spark_data = spark
                    .get("sparkData")
                    .or_else(|| spark.get("spark_data"))
                    .cloned()
                    .unwrap_or_else(|| spark.clone());
                data.saved_sparks.push(SavedSpark {
                    id,
                    title: legacy_string(spark, "title", "Legacy Spark"),
                    created_at: "legacy-import".to_string(),
                    spark_data,
                });
            }
        }

        if let Some(active) = payload.get("theshed_active_mixer_spark") {
            if !active.is_null() {
                let id = active
                    .get("id")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| "legacy-active-mixer-spark".to_string());
                if !data.saved_sparks.iter().any(|existing| existing.id == id) {
                    let spark_data = active
                        .get("sparkData")
                        .or_else(|| active.get("spark_data"))
                        .cloned()
                        .unwrap_or_else(|| active.clone());
                    data.saved_sparks.push(SavedSpark {
                        id,
                        title: legacy_string(active, "title", "Legacy Mixer Spark"),
                        created_at: "legacy-import".to_string(),
                        spark_data,
                    });
                }
            }
        }

        if let Some(lessons) = payload
            .get("theshed_saved")
            .and_then(|value| value.as_array())
        {
            for lesson in lessons {
                let id = legacy_id(lesson, "legacy-lesson", data.saved_lessons.len());
                if data.saved_lessons.iter().any(|existing| existing.id == id) {
                    continue;
                }
                data.saved_lessons.push(SavedLesson {
                    id,
                    title: legacy_string(lesson, "title", "Legacy Lesson"),
                    mode: legacy_string(lesson, "mode", "legacy"),
                    volume_id: legacy_string(lesson, "volumeId", "legacy"),
                    musician_id: legacy_string(lesson, "artistId", "legacy"),
                    musician_name: legacy_string(lesson, "artistName", "Legacy"),
                    duration: lesson
                        .get("duration")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(10) as u32,
                    theory: legacy_string(lesson, "theory", ""),
                    drill: legacy_string(lesson, "drill", ""),
                });
            }
        }
    })
}

fn legacy_id(value: &serde_json::Value, prefix: &str, index: usize) -> String {
    value
        .get("id")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}-{}", prefix, index))
}

fn legacy_string(value: &serde_json::Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or(fallback)
        .to_string()
}

fn parse_legacy_midi_key(key: &str) -> Option<(String, &'static str, u8)> {
    let parts: Vec<&str> = key.split('_').collect();
    match parts.as_slice() {
        ["looper", "cc", number] => Some(("looper".to_string(), "cc", number.parse().ok()?)),
        ["looper", "note", number] => Some(("looper".to_string(), "note", number.parse().ok()?)),
        ["mpc", "cc", number] => Some(("mpc".to_string(), "cc", number.parse().ok()?)),
        ["mpc", "note", number] => Some(("mpc".to_string(), "note", number.parse().ok()?)),
        ["cc", number] => Some(("looper".to_string(), "cc", number.parse().ok()?)),
        ["note", number] => Some(("looper".to_string(), "note", number.parse().ok()?)),
        [number] => Some(("looper".to_string(), "cc", number.parse().ok()?)),
        _ => None,
    }
}

fn legacy_midi_target(target: &str) -> Option<MidiTarget> {
    match target {
        "looper_rec" => Some(MidiTarget::LooperRecord(0)),
        "looper_toggle" => Some(MidiTarget::LooperToggle(0)),
        "looper_undo" => Some(MidiTarget::LooperUndo(0)),
        "looper_clear" => Some(MidiTarget::LooperClear(0)),
        "looper_active" | "looper_active_toggle" => Some(MidiTarget::LooperActiveToggle(0)),
        "looper_stop" => Some(MidiTarget::Transport(1)),
        "play_sequence" => Some(MidiTarget::Transport(0)),
        "part_a" => Some(MidiTarget::LooperSelect(0)),
        "part_b" => Some(MidiTarget::LooperSelect(1)),
        "part_c" => Some(MidiTarget::LooperSelect(2)),
        "part_d" => Some(MidiTarget::LooperSelect(3)),
        "part_e" => Some(MidiTarget::LooperSelect(4)),
        "mic_gain" => Some(MidiTarget::MicGain),
        "mpc_swing" => Some(MidiTarget::MpcParam(0)),
        "mpc_kit" => Some(MidiTarget::MpcParam(1)),
        _ => parse_legacy_param_target(target),
    }
}

fn parse_legacy_param_target(target: &str) -> Option<MidiTarget> {
    let id = target
        .strip_prefix("param_")
        .or_else(|| target.strip_prefix("p"))
        .and_then(|value| value.parse::<u8>().ok())?;
    Some(MidiTarget::Param(id))
}

#[tauri::command]
fn load_sample(pad_id: usize, path: String, state: State<'_, AppState>) -> Result<(), String> {
    println!("Loading sample for Pad {}: {}", pad_id, path);

    // Run file IO on a separate thread to avoid blocking the main thread (though Tauri commands are async by default in frontend, they run on thread pool here? No, better safe)
    let path_clone = path.clone();
    let producer = state.command_producer.clone();
    let event_producer = state.event_producer.clone();

    std::thread::spawn(move || {
        match hound::WavReader::open(&path_clone) {
            Ok(mut reader) => {
                let spec = reader.spec();
                let samples: Vec<f32> = match spec.sample_format {
                    hound::SampleFormat::Int => {
                        // Assuming 16-bit for simplicity, but could be 24/32
                        reader
                            .samples::<i16>()
                            .filter_map(Result::ok)
                            .map(|s| s as f32 / 32768.0)
                            .collect()
                    }
                    hound::SampleFormat::Float => {
                        reader.samples::<f32>().filter_map(Result::ok).collect()
                    }
                };

                // Handle Stereo to Mono (take left channel or average?)
                // If channels > 1, we might need to interleave or just take first channel.
                // Simple approach: Take everything. The Sampler expects mono for now?
                // The Sampler code I wrote just loops through the Vec.
                // If it's stereo, it will play at half speed effectively if we treat it as mono stream?
                // Wait, audio engine loop chunks in 2s (Stereo).
                // But the sampler::process returns a single f32 (Mono).
                // So if we load stereo data into a mono sampler, we should probably mix down to mono.

                let final_samples = if spec.channels > 1 {
                    let mut mono = Vec::with_capacity(samples.len() / spec.channels as usize);
                    for chunk in samples.chunks(spec.channels as usize) {
                        let sum: f32 = chunk.iter().sum();
                        mono.push(sum / spec.channels as f32);
                    }
                    mono
                } else {
                    samples
                };

                let sample_count = final_samples.len();
                let waveform = build_sample_waveform(&final_samples, 96);

                if let Ok(mut prod) = producer.lock() {
                    let _ = prod.push(AudioCommand::UploadSample {
                        pad_id,
                        data: final_samples,
                    });
                    println!("Sample uploaded to Pad {}", pad_id);
                }

                if let Ok(mut prod) = event_producer.lock() {
                    let _ = prod.push(AudioThreadEvent::MpcSampleLoaded {
                        pad_id,
                        sample_rate: spec.sample_rate,
                        samples: sample_count,
                        waveform,
                    });
                }
            }
            Err(e) => eprintln!("Failed to load WAV: {}", e),
        }
    });

    Ok(())
}

fn read_wav_mono(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .filter_map(Result::ok)
            .map(|s| s as f32 / 32768.0)
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().filter_map(Result::ok).collect(),
    };

    if spec.channels > 1 {
        let mut mono = Vec::with_capacity(samples.len() / spec.channels as usize);
        for chunk in samples.chunks(spec.channels as usize) {
            let sum: f32 = chunk.iter().sum();
            mono.push(sum / spec.channels as f32);
        }
        Ok(mono)
    } else {
        Ok(samples)
    }
}

fn write_wav_mono(path: &Path, buffer: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &sample in buffer {
        let amp = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(amp).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

fn part_mixed_path(project_path: &Path, part_name: &str) -> PathBuf {
    project_path.join(format!("part_{}.wav", part_name))
}

fn part_layer_path(project_path: &Path, part_name: &str, layer_index: usize) -> PathBuf {
    project_path.join(format!("part_{}_layer_{}.wav", part_name, layer_index))
}

fn remove_stale_part_audio(project_path: &Path, part_name: &str) {
    let _ = fs::remove_file(part_mixed_path(project_path, part_name));

    let Ok(entries) = fs::read_dir(project_path) else {
        return;
    };
    let prefix = format!("part_{}_layer_", part_name);
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name.starts_with(&prefix) && file_name.ends_with(".wav") {
            let _ = fs::remove_file(path);
        }
    }
}

#[derive(Debug, Clone)]
struct ProjectManifestInfo {
    schema_version: u32,
    part_count: usize,
    sources: Vec<Option<LooperSource>>,
}

fn infer_project_part_count(manifest: &serde_json::Value) -> usize {
    let explicit_count = manifest
        .get("part_count")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize);
    let manifest_part_count = manifest
        .get("parts")
        .and_then(|value| value.as_array())
        .map(Vec::len);

    explicit_count
        .or(manifest_part_count)
        .unwrap_or(3)
        .clamp(1, LOOPER_PART_COUNT)
}

fn parse_looper_source_metadata(value: &serde_json::Value) -> Option<LooperSource> {
    if let Ok(source) = serde_json::from_value::<LooperSource>(value.clone()) {
        return Some(source);
    }

    let kind = value.get("kind").and_then(|value| value.as_str())?;
    match kind {
        "input_channel" => {
            let channel = value
                .get("channel")
                .and_then(|value| value.as_u64())
                .unwrap_or(1)
                .min(8) as u8;
            Some(LooperSource::InputChannel(channel))
        }
        "input_mix" => Some(LooperSource::InputMix),
        "synth" => Some(LooperSource::Synth),
        "mpc" => Some(LooperSource::Mpc),
        "jam" => Some(LooperSource::Jam),
        "instrument_mix" => Some(LooperSource::InstrumentMix),
        "silent" => Some(LooperSource::Silent),
        _ => None,
    }
}

fn parse_project_part_sources(manifest: &serde_json::Value) -> Vec<Option<LooperSource>> {
    let mut sources = vec![None; LOOPER_PART_COUNT];
    if let Some(parts) = manifest.get("parts").and_then(|value| value.as_array()) {
        for (idx, part) in parts.iter().take(LOOPER_PART_COUNT).enumerate() {
            sources[idx] = part.get("source").and_then(parse_looper_source_metadata);
        }
    }
    sources
}

fn validate_project_manifest(project_path: &Path) -> Result<ProjectManifestInfo, String> {
    if !project_path.exists() {
        return Err(format!("Project folder does not exist: {:?}", project_path));
    }
    if !project_path.is_dir() {
        return Err(format!("Project path is not a folder: {:?}", project_path));
    }

    let manifest_path = project_path.join(PROJECT_MANIFEST_FILE);
    let contents = fs::read_to_string(&manifest_path).map_err(|err| {
        format!(
            "Missing or unreadable project manifest {:?}: {}",
            manifest_path, err
        )
    })?;
    let manifest: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|err| format!("Invalid project manifest {:?}: {}", manifest_path, err))?;

    let schema_version = manifest
        .get("schema_version")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            format!(
                "Project manifest {:?} is missing schema_version.",
                manifest_path
            )
        })?;
    let schema_version = schema_version as u32;
    if schema_version != LEGACY_PROJECT_SCHEMA_VERSION && schema_version != PROJECT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported project schema {} in {:?}. Expected {} or {}.",
            schema_version, manifest_path, LEGACY_PROJECT_SCHEMA_VERSION, PROJECT_SCHEMA_VERSION
        ));
    }

    if manifest.get("app").and_then(|value| value.as_str()) != Some(PROJECT_APP) {
        return Err(format!(
            "Project manifest {:?} is not a SHED POWER project.",
            manifest_path
        ));
    }
    if manifest.get("kind").and_then(|value| value.as_str()) != Some(PROJECT_KIND) {
        return Err(format!(
            "Project manifest {:?} is not a looper project.",
            manifest_path
        ));
    }

    Ok(ProjectManifestInfo {
        schema_version,
        part_count: infer_project_part_count(&manifest),
        sources: parse_project_part_sources(&manifest),
    })
}

fn build_project_manifest(
    buffers: &[Vec<f32>],
    layers: &[Vec<Vec<f32>>],
    sources: &[LooperSource],
    sample_rate: u32,
) -> serde_json::Value {
    let parts: Vec<serde_json::Value> = LOOPER_PART_NAMES
        .iter()
        .enumerate()
        .map(|(i, part_name)| {
            let part_layers = layers.get(i).map(Vec::as_slice).unwrap_or(&[]);
            let layer_files: Vec<String> = part_layers
                .iter()
                .enumerate()
                .filter(|(_, layer)| !layer.is_empty())
                .map(|(layer_index, _)| format!("part_{}_layer_{}.wav", part_name, layer_index))
                .collect();

            let mut part = serde_json::json!({
                "id": part_name,
                "mixed_file": format!("part_{}.wav", part_name),
                "samples": buffers.get(i).map(Vec::len).unwrap_or(0),
                "layer_count": part_layers.len(),
                "layers": part_layers.len(),
                "layer_files": layer_files,
            });

            if let Some(source) = sources.get(i) {
                part["source"] = looper_source_metadata(*source);
            }

            part
        })
        .collect();

    serde_json::json!({
        "schema_version": PROJECT_SCHEMA_VERSION,
        "app": PROJECT_APP,
        "kind": PROJECT_KIND,
        "saved_at": timestamp_string(),
        "sample_rate": sample_rate,
        "part_count": LOOPER_PART_COUNT,
        "parts": parts
    })
}

fn looper_source_metadata(source: LooperSource) -> serde_json::Value {
    serde_json::to_value(source).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
fn load_project(path: String, state: State<'_, AppState>) -> Result<(), String> {
    println!("Loading project from: {}", path);
    let project_path = PathBuf::from(&path);
    let manifest = validate_project_manifest(&project_path)?;

    let _ = persistence::update_app_persistence(|data| {
        data.recent_projects.retain(|project| project.path != path);
        data.recent_projects.insert(
            0,
            ProjectRecord {
                path: path.clone(),
                action: "load".to_string(),
                recorded_at: timestamp_string(),
                schema_version: manifest.schema_version,
            },
        );
        data.recent_projects.truncate(10);
    });
    persist_looper_sources(&manifest.sources);

    let path_clone = path.clone();
    let producer = state.command_producer.clone();

    std::thread::spawn(move || {
        let project_path = PathBuf::from(path_clone);
        let mut all_empty = true;

        for (i, part_name) in LOOPER_PART_NAMES.iter().enumerate() {
            if let Some(Some(source)) = manifest.sources.get(i) {
                if let Ok(mut prod) = producer.lock() {
                    let _ = prod.push(AudioCommand::SetLooperPartSource {
                        part_id: i,
                        source: *source,
                    });
                }
            }

            if i >= manifest.part_count {
                if let Ok(mut prod) = producer.lock() {
                    let _ = prod.push(AudioCommand::LoadPartLayers {
                        part_id: i,
                        layers: vec![],
                    });
                }
                continue;
            }

            let mut layers = Vec::new();
            for layer_index in 0.. {
                let layer_path = part_layer_path(&project_path, part_name, layer_index);
                if !layer_path.exists() {
                    break;
                }
                match read_wav_mono(&layer_path) {
                    Ok(layer) => {
                        if !layer.is_empty() {
                            layers.push(layer);
                        }
                    }
                    Err(e) => println!(
                        "Could not load layer {} for Part {}: {}",
                        layer_index, part_name, e
                    ),
                }
            }

            if !layers.is_empty() {
                all_empty = false;
                let layer_count = layers.len();
                if let Ok(mut prod) = producer.lock() {
                    let _ = prod.push(AudioCommand::LoadPartLayers { part_id: i, layers });
                    println!("Loaded {} layer(s) for Part {}", layer_count, part_name);
                }
                continue;
            }

            let mixed_path = part_mixed_path(&project_path, part_name);
            match read_wav_mono(&mixed_path) {
                Ok(final_samples) => {
                    if !final_samples.is_empty() {
                        all_empty = false;
                    }
                    let sample_count = final_samples.len();
                    if let Ok(mut prod) = producer.lock() {
                        let _ = prod.push(AudioCommand::LoadPartBuffer {
                            part_id: i,
                            data: final_samples,
                        });
                    }
                    println!(
                        "Loaded mixed buffer for Part {} ({} samples)",
                        part_name, sample_count
                    );
                }
                Err(e) => {
                    println!("No buffer found for Part {}: {}", part_name, e);
                    if let Ok(mut prod) = producer.lock() {
                        let _ = prod.push(AudioCommand::LoadPartLayers {
                            part_id: i,
                            layers: vec![],
                        });
                    }
                }
            }
        }

        // Signal that all buffers are loaded
        if let Ok(mut prod) = producer.lock() {
            let _ = prod.push(AudioCommand::LoadProjectDone { all_empty });
        }
    });

    Ok(())
}

#[tauri::command]
fn scan_devices(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    match state.device_manager.scan_devices() {
        Ok(devices) => {
            let device_names: Vec<String> = devices
                .iter()
                .map(|d| format!("{}: {} ({:?})", d.id, d.name, d.device_type))
                .collect();
            Ok(device_names)
        }
        Err(e) => Err(format!("Device scan failed: {}", e)),
    }
}

#[tauri::command]
fn refresh_midi(state: State<'_, AppState>) -> Result<String, String> {
    match state.device_manager.refresh_midi() {
        Ok(_) => Ok("MIDI refreshed successfully".to_string()),
        Err(e) => Err(format!("MIDI refresh failed: {}", e)),
    }
}

#[tauri::command]
fn refresh_audio(state: State<'_, AppState>) -> Result<String, String> {
    match state.device_manager.refresh_audio() {
        Ok(_) => Ok("Audio devices scanned successfully".to_string()),
        Err(e) => Err(format!("Audio refresh failed: {}", e)),
    }
}

#[tauri::command]
fn send_audio_command(command: String, state: State<'_, AppState>) -> Result<(), String> {
    let cmd = parse_audio_command(&command).map_err(|err| {
        eprintln!("{}", err);
        err
    })?;

    // Intercept Save Logic
    if let AudioCommand::SaveProject { ref path } = cmd {
        let mut lock = state.pending_save_path.lock().map_err(|_| {
            "Failed to prepare save command: pending save path lock poisoned".to_string()
        })?;
        *lock = Some(path.clone());

        let project_path = path.clone();
        let _ = persistence::update_app_persistence(|data| {
            data.recent_projects
                .retain(|project| project.path != project_path);
            data.recent_projects.insert(
                0,
                ProjectRecord {
                    path: project_path,
                    action: "save".to_string(),
                    recorded_at: timestamp_string(),
                    schema_version: PROJECT_SCHEMA_VERSION,
                },
            );
            data.recent_projects.truncate(10);
        });
    }

    match &cmd {
        AudioCommand::SetLooperPartSource { part_id, source } => {
            persist_looper_source(*part_id, *source);
        }
        AudioCommand::SetLooperPartInput { part_id, channel } => {
            persist_looper_source(*part_id, legacy_channel_to_looper_source(*channel));
        }
        _ => {}
    }

    enqueue_audio_command(&state.command_producer, cmd).map_err(|err| {
        eprintln!("{}", err);
        err
    })
}

fn parse_audio_command(command: &str) -> Result<AudioCommand, String> {
    serde_json::from_str::<AudioCommand>(command).map_err(|err| {
        format!(
            "Failed to parse audio command: {}. Payload: {}",
            err,
            truncate_for_log(command, 240)
        )
    })
}

fn enqueue_audio_command(
    producer: &Arc<Mutex<ringbuf::HeapProducer<AudioCommand>>>,
    cmd: AudioCommand,
) -> Result<(), String> {
    let command_preview = truncate_for_log(&format!("{:?}", cmd), 240);
    let mut producer = producer.lock().map_err(|_| {
        format!(
            "Failed to enqueue audio command: queue lock poisoned while sending {}",
            command_preview
        )
    })?;
    producer.push(cmd).map_err(|_| {
        format!(
            "Failed to enqueue audio command: queue full while sending {}",
            command_preview
        )
    })
}

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let preview: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", preview)
    } else {
        preview
    }
}

fn timestamp_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    seconds.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_audio_command_accepts_note_payload() {
        let cmd = parse_audio_command(r#"{"NoteOn":{"note":60,"velocity":110}}"#).unwrap();

        match cmd {
            AudioCommand::NoteOn { note, velocity } => {
                assert_eq!(note, 60);
                assert_eq!(velocity, 110);
            }
            other => panic!("expected NoteOn command, got {:?}", other),
        }
    }

    #[test]
    fn parse_audio_command_accepts_stop_all_sounds() {
        let cmd = parse_audio_command(r#"{"StopAllSounds":null}"#).unwrap();

        assert!(matches!(cmd, AudioCommand::StopAllSounds));
    }

    #[test]
    fn parse_audio_command_reports_payload_errors() {
        let err = parse_audio_command(r#"{"NoteOn":{"note":null,"velocity":110}}"#).unwrap_err();

        assert!(err.contains("Failed to parse audio command"));
        assert!(err.contains("note"));
        assert!(err.contains("NoteOn"));
    }

    #[test]
    fn enqueue_audio_command_reports_full_queue() {
        let rb = HeapRb::<AudioCommand>::new(1);
        let (producer, _consumer) = rb.split();
        let producer = Arc::new(Mutex::new(producer));

        enqueue_audio_command(&producer, AudioCommand::Play).unwrap();
        let err = enqueue_audio_command(&producer, AudioCommand::Stop).unwrap_err();

        assert!(err.contains("queue full"));
        assert!(err.contains("Stop"));
    }

    #[test]
    fn project_manifest_v2_lists_all_five_parts() {
        let buffers = vec![vec![0.0; 8], vec![], vec![], vec![0.0; 4], vec![]];
        let layers = vec![
            vec![vec![0.0; 8]],
            vec![],
            vec![],
            vec![vec![0.0; 4], vec![0.1; 4]],
            vec![],
        ];
        let sources = vec![
            LooperSource::InputChannel(1),
            LooperSource::InputChannel(2),
            LooperSource::InputMix,
            LooperSource::Synth,
            LooperSource::Silent,
        ];

        let manifest = build_project_manifest(&buffers, &layers, &sources, 48_000);

        assert_eq!(manifest["schema_version"].as_u64(), Some(2));
        assert_eq!(manifest["part_count"].as_u64(), Some(5));
        assert_eq!(manifest["parts"].as_array().map(Vec::len), Some(5));
        assert_eq!(manifest["parts"][0]["id"].as_str(), Some("A"));
        assert_eq!(
            manifest["parts"][4]["mixed_file"].as_str(),
            Some("part_E.wav")
        );
        assert_eq!(manifest["parts"][3]["layer_count"].as_u64(), Some(2));
        assert_eq!(manifest["parts"][3]["source"].as_str(), Some("Synth"));
    }

    #[test]
    fn legacy_project_manifest_defaults_to_three_parts() {
        let manifest = serde_json::json!({
            "schema_version": 1,
            "app": "shed-power",
            "kind": "looper_project",
            "parts": [
                { "id": "A" },
                { "id": "B" },
                { "id": "C" }
            ]
        });

        assert_eq!(infer_project_part_count(&manifest), 3);
        assert_eq!(
            parse_project_part_sources(&manifest),
            vec![None; LOOPER_PART_COUNT]
        );
    }

    #[test]
    fn missing_part_count_without_parts_is_legacy_three_part_project() {
        let manifest = serde_json::json!({
            "schema_version": 1,
            "app": "shed-power",
            "kind": "looper_project"
        });

        assert_eq!(infer_project_part_count(&manifest), 3);
    }

    #[test]
    fn app_settings_update_preserves_looper_sources_when_omitted() {
        let mut data = AppPersistence::default();
        let preserved_sources = vec![
            LooperSource::Synth,
            LooperSource::Mpc,
            LooperSource::Jam,
            LooperSource::InstrumentMix,
            LooperSource::InputChannel(8),
        ];
        data.settings.looper_sources = preserved_sources.clone();

        apply_app_settings_update(
            &mut data,
            AppSettingsUpdate {
                mic_active: Some(false),
                midi_active: Some(false),
                active_module: Some("synth".to_string()),
                metronome_enabled: Some(true),
                metronome_bpm: Some(96.0),
                looper_sources: None,
            },
        );

        assert!(!data.settings.mic_active);
        assert!(!data.settings.midi_active);
        assert_eq!(data.settings.active_module, "synth");
        assert!(data.settings.metronome_enabled);
        assert_eq!(data.settings.metronome_bpm, 96.0);
        assert_eq!(data.settings.looper_sources, preserved_sources);
    }

    #[test]
    fn app_settings_update_replaces_looper_sources_when_supplied() {
        let mut data = AppPersistence::default();
        data.settings.looper_sources = vec![LooperSource::Silent; LOOPER_PART_COUNT];

        apply_app_settings_update(
            &mut data,
            AppSettingsUpdate {
                mic_active: None,
                midi_active: None,
                active_module: None,
                metronome_enabled: None,
                metronome_bpm: None,
                looper_sources: Some(vec![LooperSource::InputChannel(0), LooperSource::Mpc]),
            },
        );

        assert_eq!(data.settings.looper_sources.len(), LOOPER_PART_COUNT);
        assert_eq!(data.settings.looper_sources[0], LooperSource::InputMix);
        assert_eq!(data.settings.looper_sources[1], LooperSource::Mpc);
        assert_eq!(data.settings.looper_sources[2], LooperSource::InputMix);
        assert_eq!(data.settings.looper_sources[3], LooperSource::Silent);
        assert_eq!(data.settings.looper_sources[4], LooperSource::Silent);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("SHED POWER: Igniting Engine...");

    let cmd_rb = HeapRb::<AudioCommand>::new(1024);
    let (cmd_prod, cmd_cons) = cmd_rb.split();
    let command_producer = Arc::new(Mutex::new(cmd_prod));

    let event_rb = HeapRb::<AudioThreadEvent>::new(1024);
    let (event_prod, mut event_cons) = event_rb.split();
    let event_producer = Arc::new(Mutex::new(event_prod));

    let debug_rb = HeapRb::<AudioThreadEvent>::new(1024);
    let (debug_prod, mut midi_debug_cons) = debug_rb.split();
    let midi_debug_producer = Arc::new(Mutex::new(debug_prod));

    let engine = match AudioEngine::new(cmd_cons, event_producer.clone()) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("CRITICAL ERROR: Failed to start Audio Engine: {}", err);
            return;
        }
    };

    let persisted = persistence::load_app_persistence();
    let persisted_looper_sources =
        persistence::normalize_looper_sources(&persisted.settings.looper_sources);
    if let Ok(mut producer) = command_producer.lock() {
        for (part_id, source) in persisted_looper_sources.iter().copied().enumerate() {
            let _ = producer.push(AudioCommand::SetLooperPartSource { part_id, source });
        }
    }

    let midi_map = Arc::new(Mutex::new(persisted.midi_map.clone()));
    let midi_active = Arc::new(AtomicBool::new(persisted.settings.midi_active));

    let midi = match MidiHandler::new(
        command_producer.clone(),
        midi_map.clone(),
        midi_active.clone(),
        midi_debug_producer,
    ) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("MIDI Init Warning: {}", e);
            MidiHandler::dummy()
        }
    };

    let pending_save_path = Arc::new(Mutex::new(None));
    let save_path_clone = pending_save_path.clone();

    // Initialize Device Manager
    let device_manager = DeviceManager::new(
        command_producer.clone(),
        event_producer.clone(),
        midi_map.clone(),
        midi_active.clone(),
    );

    // Initial device scan
    if let Err(e) = device_manager.refresh_midi() {
        println!("Initial MIDI scan warning: {}", e);
    }

    tauri::Builder::default()
        .manage(AppState {
            command_producer,
            event_producer,
            pending_save_path,
            midi_map,
            midi_active,
            _engine: engine,
            _midi: midi,
            device_manager: device_manager.clone(),
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let handle_clone = handle.clone();

            thread::spawn(move || {
                loop {
                    // 1. Read Audio Engine Events
                    while let Some(event) = event_cons.pop() {
                        match event {
                            AudioThreadEvent::EngineInfo { sample_rate, input_device } => {
                                let _ = handle.emit("engine-info", serde_json::json!({ "sample_rate": sample_rate, "input_device": input_device }));
                            }
                            AudioThreadEvent::TunerReading { frequency, rms } => {
                                let _ = handle.emit("tuner-reading", serde_json::json!({ "frequency": frequency, "rms": rms }));
                            }
                            AudioThreadEvent::WaveformReady { part_id, data } => {
                                let payload = WaveformPayload { part_id, data };
                                let _ = handle.emit("waveform-ready", payload);
                            }
                            AudioThreadEvent::ParamChange { id, value } => {
                                let _ = handle.emit("param-change", serde_json::json!({ "id": id, "value": value }));
                            }
                            AudioThreadEvent::MpcParamChange { id, value } => {
                                let _ = handle.emit("mpc-param-change", serde_json::json!({ "id": id, "value": value }));
                            }
                            AudioThreadEvent::JamParamChange { id, value } => {
                                let _ = handle.emit("jam-param-change", serde_json::json!({ "id": id, "value": value }));
                            }
                            AudioThreadEvent::PartActive { part_id } => {
                                let _ = handle.emit("part-active", part_id);
                            }
                            AudioThreadEvent::JamControl { action } => {
                                let _ = handle.emit("jam-control", action);
                            }
                            AudioThreadEvent::JamChordStep {
                                index,
                                notes,
                                label,
                            } => {
                                let _ = handle.emit(
                                    "jam-chord-step",
                                    serde_json::json!({
                                        "index": index,
                                        "notes": notes,
                                        "label": label
                                    }),
                                );
                            }
                            AudioThreadEvent::DrumTrigger { note } => {
                                let _ = handle.emit("drum-trigger", note);
                            }
                            AudioThreadEvent::MpcStep { step } => {
                                let _ = handle.emit("mpc-step", step);
                            }
                            AudioThreadEvent::MpcTransport { playing } => {
                                let _ = handle.emit("mpc-transport", playing);
                            }
                            AudioThreadEvent::AllSoundsStopped => {
                                let _ = handle.emit("all-sounds-stopped", true);
                            }
                            AudioThreadEvent::MpcSampleLoaded {
                                pad_id,
                                sample_rate,
                                samples,
                                waveform,
                            } => {
                                let _ = handle.emit(
                                    "mpc-sample-loaded",
                                    serde_json::json!({
                                        "pad_id": pad_id,
                                        "sample_rate": sample_rate,
                                        "samples": samples,
                                        "waveform": waveform
                                    }),
                                );
                            }
                            AudioThreadEvent::LooperStateChange { part_id, state } => {
                                let _ = handle.emit("looper-state", serde_json::json!({ "part_id": part_id, "state": state }));
                            }
                            AudioThreadEvent::LooperLayerCount { part_id, layers } => {
                                let _ = handle.emit("looper-layers", serde_json::json!({ "part_id": part_id, "layers": layers }));
                            }
                            AudioThreadEvent::MidiActive { active } => {
                                let _ = handle.emit("midi-active-state", active);
                            }
                            AudioThreadEvent::LoopDuration { samples } => {
                                let _ = handle.emit("loop-duration", samples);
                            }
                            AudioThreadEvent::SequenceStep { step, part_id } => {
                                let _ = handle.emit("sequence-step", serde_json::json!({ "step": step, "part_id": part_id }));
                            }
                            AudioThreadEvent::SequenceFinished => {
                                let _ = handle.emit("sequence-finished", true);
                            }
                            AudioThreadEvent::ProjectLoaded { all_empty, samples } => {
                                let _ = handle.emit("load-complete", serde_json::json!({ "all_empty": all_empty, "samples": samples }));
                            }
                            AudioThreadEvent::DeviceListUpdate => {
                                let _ = handle.emit("device-list-update", true);
                            }
                            AudioThreadEvent::ProjectSnapshot { buffers, layers, sources, sample_rate } => {
                                // Handle Save
                                if let Ok(mut lock) = save_path_clone.lock() {
                                    if let Some(path) = lock.take() {
                                        println!("Saving Project to: {}", path);
                                        let project_path = PathBuf::from(&path);
                                        if let Err(err) = fs::create_dir_all(&project_path) {
                                            eprintln!("Failed to create project directory {}: {}", path, err);
                                            let _ = handle.emit("save-complete", false);
                                            continue;
                                        }

                                        let manifest = build_project_manifest(&buffers, &layers, &sources, sample_rate);
                                        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
                                            let _ = fs::write(project_path.join(PROJECT_MANIFEST_FILE), json);
                                        }

                                        for (i, part_name) in LOOPER_PART_NAMES.iter().enumerate() {
                                            remove_stale_part_audio(&project_path, part_name);

                                            if buffers.get(i).is_some_and(|buffer| !buffer.is_empty()) {
                                                let mixed_path = part_mixed_path(&project_path, part_name);
                                                if let Err(e) = write_wav_mono(&mixed_path, &buffers[i], sample_rate) {
                                                    eprintln!("Failed to save mixed Part {}: {}", part_name, e);
                                                }
                                            }

                                            let part_layers = layers.get(i).map(Vec::as_slice).unwrap_or(&[]);
                                            for (layer_index, layer) in part_layers.iter().enumerate() {
                                                if layer.is_empty() {
                                                    continue;
                                                }
                                                let layer_path = part_layer_path(&project_path, part_name, layer_index);
                                                if let Err(e) = write_wav_mono(&layer_path, layer, sample_rate) {
                                                    eprintln!(
                                                        "Failed to save Part {} layer {}: {}",
                                                        part_name, layer_index, e
                                                    );
                                                }
                                            }
                                        }
                                        let _ = handle.emit("save-complete", true);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // 2. Read MIDI Debug Events
                    while let Some(event) = midi_debug_cons.pop() {
                         if let AudioThreadEvent::MidiDebug { status, data1, data2 } = event {
                             let _ = handle.emit("midi-debug", serde_json::json!({ "status": status, "data1": data1, "data2": data2 }));
                         }
                    }

                    thread::sleep(Duration::from_millis(16));
                }
            });

            // Device monitoring thread
            let device_manager_clone = device_manager.clone();
            let device_handle = handle_clone;
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_millis(2000)); // Check every 2 seconds

                    if let Ok((new_devices, removed_devices)) = device_manager_clone.check_for_changes() {
                        // Emit new device events
                        for device in new_devices {
                            println!("Device connected: {} ({:?})", device.name, device.device_type);
                            let device_type_str = match device.device_type {
                                crate::audio::DeviceType::MidiInput => "midi_input",
                                crate::audio::DeviceType::AudioInput => "audio_input",
                                crate::audio::DeviceType::AudioOutput => "audio_output",
                            };
                            let _ = device_handle.emit("device-connected", serde_json::json!({
                                "name": device.name,
                                "type": device_type_str
                            }));

                            // Auto-connect MIDI devices if needed
                            if matches!(device.device_type, crate::audio::DeviceType::MidiInput) {
                                if device.name.to_lowercase().contains("mpk") ||
                                   device.name.to_lowercase().contains("akai") {
                                    println!("Auto-connecting to priority MIDI device: {}", device.name);
                                    let _ = device_manager_clone.refresh_midi();
                                }
                            }
                        }

                        // Emit removed device events
                        for device in removed_devices {
                            println!("Device disconnected: {} ({:?})", device.name, device.device_type);
                            let device_type_str = match device.device_type {
                                crate::audio::DeviceType::MidiInput => "midi_input",
                                crate::audio::DeviceType::AudioInput => "audio_input",
                                crate::audio::DeviceType::AudioOutput => "audio_output",
                            };
                            let _ = device_handle.emit("device-disconnected", serde_json::json!({
                                "name": device.name,
                                "type": device_type_str
                            }));
                        }
                    }
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            send_audio_command,
            start_midi_learn,
            set_midi_context,
            set_midi_active,
            load_sample,
            load_project,
            scan_devices,
            refresh_midi,
            refresh_audio,
            get_persistence,
            get_persistence_path,
            export_persistence,
            import_persistence_file,
            save_app_settings,
            save_module_state,
            save_spark,
            delete_spark,
            save_lesson,
            delete_lesson,
            start_practice_session,
            finish_practice_session,
            import_legacy_browser_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
