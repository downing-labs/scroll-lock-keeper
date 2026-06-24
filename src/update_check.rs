// Re-checks GitHub Releases every time the shared polling interval refreshes
// (the same interval the scroll-lock toggle uses), so a long-running
// minimized instance eventually notices a new release without needing a
// restart. Stops once an update is found -- no point re-checking after that.
// Result surfaces on the main window when it's found. Silent failure on any
// single network/parse issue -- just retries on the next interval.

use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const OWNER: &str = "downing-labs";
const REPO: &str = "scroll-lock-keeper";

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    html_url: String,
}

#[derive(Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

pub type UpdateStatus = Arc<Mutex<Option<UpdateInfo>>>;

/// Spawns the polling loop on a background thread: checks immediately at
/// startup, then re-checks every time the interval refreshes (the interval
/// can change live via Settings). Stops once an update is found.
pub fn check_async(status: UpdateStatus, interval_secs: Arc<AtomicU64>) {
    thread::spawn(move || loop {
        if let Some(info) = check_once() {
            if let Ok(mut guard) = status.lock() {
                *guard = Some(info);
            }
            break;
        }
        let secs = interval_secs.load(Ordering::Relaxed).max(1);
        thread::sleep(Duration::from_secs(secs));
    });
}

fn check_once() -> Option<UpdateInfo> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        OWNER, REPO
    );

    // GitHub's API rejects requests with no User-Agent.
    let mut response = ureq::get(&url)
        .header("User-Agent", "scroll-lock-keeper-update-checker")
        .config()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .call()
        .ok()?;

    let body_text: String = response.body_mut().read_to_string().ok()?;
    let body: ReleaseResponse = serde_json::from_str(&body_text).ok()?;
    let remote = body.tag_name.trim_start_matches('v');
    let local = env!("CARGO_PKG_VERSION");

    if is_newer(remote, local) {
        Some(UpdateInfo {
            version: remote.to_string(),
            url: body.html_url,
        })
    } else {
        None
    }
}

/// Plain major.minor.patch comparison -- no pre-release/build metadata
/// handling, which is fine since releases here always follow vX.Y.Z.
fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> { s.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let r = parse(remote);
    let l = parse(local);
    for i in 0..3 {
        let rv = r.get(i).copied().unwrap_or(0);
        let lv = l.get(i).copied().unwrap_or(0);
        if rv != lv {
            return rv > lv;
        }
    }
    false
}
