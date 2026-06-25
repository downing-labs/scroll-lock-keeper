// Settings persistence: a small per-user JSON file under %APPDATA%.
//
// This deliberately does NOT live next to the exe. The app is meant to be
// copied straight into the Windows Startup folder (no shortcut, no registry
// entry) -- and Windows tries to "open" every file sitting in Startup at
// logon. A settings.json next to the exe would get opened in Notepad (or
// whatever .json is associated with) on every login. %APPDATA% is the
// standard per-user writable location that's independent of wherever the
// exe itself happens to be placed.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_INTERVAL_SECS: u64 = 240;
const SETTINGS_FILE: &str = "settings.json";
const SETTINGS_DIR: &str = "ScrollLockKeeper";

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
    let dir = std::env::var("APPDATA")
        .map(|p| PathBuf::from(p).join(SETTINGS_DIR))
        .unwrap_or_else(|_| PathBuf::from(SETTINGS_DIR));
    let _ = fs::create_dir_all(&dir);
    dir.join(SETTINGS_FILE)
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
