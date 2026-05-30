use crate::audio::{LooperSource, MidiMap, LOOPER_PART_COUNT};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "shed-power";
const APP_STATE_FILE: &str = "app_state.json";
pub const APP_SCHEMA_VERSION: u32 = 1;

pub fn default_looper_sources() -> Vec<LooperSource> {
    vec![
        LooperSource::InputChannel(1),
        LooperSource::InputChannel(2),
        LooperSource::InputMix,
        LooperSource::Silent,
        LooperSource::Silent,
    ]
}

pub fn normalize_looper_sources(sources: &[LooperSource]) -> Vec<LooperSource> {
    let defaults = default_looper_sources();
    (0..LOOPER_PART_COUNT)
        .map(|idx| {
            let source = sources.get(idx).copied().unwrap_or(defaults[idx]);
            match source {
                LooperSource::InputChannel(0) => LooperSource::InputMix,
                LooperSource::InputChannel(channel) => LooperSource::InputChannel(channel.min(8)),
                other => other,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub mic_active: bool,
    pub midi_active: bool,
    pub active_module: String,
    pub metronome_enabled: bool,
    pub metronome_bpm: f32,
    #[serde(default = "default_looper_sources")]
    pub looper_sources: Vec<LooperSource>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mic_active: true,
            midi_active: true,
            active_module: "looper".to_string(),
            metronome_enabled: false,
            metronome_bpm: 120.0,
            looper_sources: default_looper_sources(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSpark {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub spark_data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLesson {
    pub id: String,
    pub title: String,
    pub mode: String,
    pub volume_id: String,
    pub musician_id: String,
    pub musician_name: String,
    pub duration: u32,
    pub theory: String,
    pub drill: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeSession {
    pub lesson_id: String,
    pub title: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PracticeState {
    pub active_session: Option<PracticeSession>,
    pub sessions: Vec<PracticeSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub path: String,
    pub action: String,
    pub recorded_at: String,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPersistence {
    pub schema_version: u32,
    pub midi_map: MidiMap,
    pub saved_sparks: Vec<SavedSpark>,
    pub saved_lessons: Vec<SavedLesson>,
    pub practice_state: PracticeState,
    pub settings: AppSettings,
    pub recent_projects: Vec<ProjectRecord>,
    #[serde(default)]
    pub module_state: serde_json::Map<String, serde_json::Value>,
}

impl Default for AppPersistence {
    fn default() -> Self {
        Self {
            schema_version: APP_SCHEMA_VERSION,
            midi_map: MidiMap::new(),
            saved_sparks: Vec::new(),
            saved_lessons: Vec::new(),
            practice_state: PracticeState::default(),
            settings: AppSettings::default(),
            recent_projects: Vec::new(),
            module_state: serde_json::Map::new(),
        }
    }
}

pub fn app_data_dir() -> PathBuf {
    if let Ok(value) = env::var("APPDATA") {
        return PathBuf::from(value).join(APP_DIR_NAME);
    }

    if let Ok(value) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(value).join(APP_DIR_NAME);
    }

    if let Ok(value) = env::var("HOME") {
        return PathBuf::from(value).join(format!(".{}", APP_DIR_NAME));
    }

    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(APP_DIR_NAME)
}

pub fn app_state_path() -> PathBuf {
    app_data_dir().join(APP_STATE_FILE)
}

pub fn load_app_persistence() -> AppPersistence {
    let path = app_state_path();
    match fs::read_to_string(&path) {
        Ok(contents) => {
            let mut data: AppPersistence = serde_json::from_str(&contents).unwrap_or_else(|err| {
                eprintln!("Failed to parse app persistence at {:?}: {}", path, err);
                AppPersistence::default()
            });
            data.settings.looper_sources = normalize_looper_sources(&data.settings.looper_sources);
            data
        }
        Err(_) => AppPersistence::default(),
    }
}

pub fn save_app_persistence(data: &AppPersistence) -> Result<(), String> {
    validate_app_persistence(data)?;

    let path = app_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create app data dir {:?}: {}", parent, err))?;
    }

    let json = serde_json::to_string_pretty(data)
        .map_err(|err| format!("Failed to serialize app persistence: {}", err))?;
    fs::write(&path, json).map_err(|err| format!("Failed to write {:?}: {}", path, err))
}

pub fn update_app_persistence(
    update: impl FnOnce(&mut AppPersistence),
) -> Result<AppPersistence, String> {
    let mut data = load_app_persistence();
    update(&mut data);
    save_app_persistence(&data)?;
    Ok(data)
}

pub fn validate_app_persistence(data: &AppPersistence) -> Result<(), String> {
    if data.schema_version != APP_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported app state schema {}. Expected {}.",
            data.schema_version, APP_SCHEMA_VERSION
        ));
    }

    for project in &data.recent_projects {
        if project.schema_version != 1 && project.schema_version != 2 {
            return Err(format!(
                "Unsupported recent project schema {} for {}.",
                project.schema_version, project.path
            ));
        }
    }

    if data.settings.looper_sources.len() != LOOPER_PART_COUNT {
        return Err(format!(
            "Expected {} looper source settings, got {}.",
            LOOPER_PART_COUNT,
            data.settings.looper_sources.len()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_include_five_looper_sources() {
        let settings = AppSettings::default();

        assert_eq!(settings.looper_sources.len(), LOOPER_PART_COUNT);
        assert_eq!(settings.looper_sources[0], LooperSource::InputChannel(1));
        assert_eq!(settings.looper_sources[1], LooperSource::InputChannel(2));
        assert_eq!(settings.looper_sources[2], LooperSource::InputMix);
        assert_eq!(settings.looper_sources[3], LooperSource::Silent);
        assert_eq!(settings.looper_sources[4], LooperSource::Silent);
    }

    #[test]
    fn looper_source_normalization_migrates_missing_and_zero_channel_values() {
        let sources =
            normalize_looper_sources(&[LooperSource::InputChannel(0), LooperSource::Synth]);

        assert_eq!(sources.len(), LOOPER_PART_COUNT);
        assert_eq!(sources[0], LooperSource::InputMix);
        assert_eq!(sources[1], LooperSource::Synth);
        assert_eq!(sources[2], LooperSource::InputMix);
        assert_eq!(sources[3], LooperSource::Silent);
        assert_eq!(sources[4], LooperSource::Silent);
    }
}
