// Single-shot, non-blocking GitHub release check at startup. Adapts the
// Electron UPDATE_SYSTEM_STANDARD pattern for a native app: no banner UI
// (there's barely a UI to put one in), result surfaces in the Help dialog
// when it's opened. Silent failure on any network/parse issue.

use serde::Deserialize;
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

/// Fires the check on a background thread. Fire-and-forget -- the result
/// lands in `status` whenever it lands; nothing waits on it.
pub fn check_async(status: UpdateStatus) {
    thread::spawn(move || {
        if let Some(info) = check_once() {
            if let Ok(mut guard) = status.lock() {
                *guard = Some(info);
            }
        }
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
