// Settings persistence: a flat JSON file next to the exe (portable, no
// registry, no AppData) -- consistent with the personal-app JSON pattern.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_INTERVAL_SECS: u64 = 240;
const SETTINGS_FILE: &str = "settings.json";

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub interval_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { interval_secs: DEFAULT_INTERVAL_SECS }
    }
}

fn settings_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(SETTINGS_FILE)))
        .unwrap_or_else(|| PathBuf::from(SETTINGS_FILE))
}

pub fn load() -> Settings {
    fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(settings)
        .unwrap_or_else(|_| "{}".to_string());
    fs::write(settings_path(), json)
}
