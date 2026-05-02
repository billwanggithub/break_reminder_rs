use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::schedule::ScheduledReminder;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(rename = "IntervalMinutes")]
    pub interval_minutes: u32,
    #[serde(rename = "AutoStart", default = "default_auto_start")]
    pub auto_start: bool,
    #[serde(rename = "PlaySound", default)]
    pub play_sound: bool,
    #[serde(rename = "Muted", default)]
    pub muted: bool,
    #[serde(rename = "ScheduledReminders", default)]
    pub scheduled_reminders: Vec<ScheduledReminder>,
}

fn default_auto_start() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_minutes: 45,
            auto_start: true,
            play_sound: false,
            muted: false,
            scheduled_reminders: Vec::new(),
        }
    }
}

pub fn settings_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .expect("LocalAppData unavailable")
        .join("BreakReminderRs");
    let _ = std::fs::create_dir_all(&base);
    base.join("settings.json")
}

pub fn load(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(path: &Path, settings: &Settings) {
    match serde_json::to_string_pretty(settings) {
        Ok(json) => {
            if let Err(e) = std::fs::write(path, json) {
                eprintln!("[break_reminder_rs] failed to write settings: {e}");
            }
        }
        Err(e) => eprintln!("[break_reminder_rs] failed to serialize settings: {e}"),
    }
}
