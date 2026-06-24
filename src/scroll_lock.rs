// Real OS-level keypress injection via SendInput. This is a Win32 API call,
// not a window message -- it works even while minimized/unfocused, because
// it injects at the same level a physical keyboard does.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_SCROLL,
};

/// One full key-down + key-up cycle for Scroll Lock. A single tap toggles
/// whatever the current Scroll Lock indicator state is.
fn tap_scroll_lock() {
    let mut down = INPUT::default();
    down.r#type = INPUT_KEYBOARD;
    down.Anonymous = INPUT_0 {
        ki: KEYBDINPUT {
            wVk: VK_SCROLL,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        },
    };

    let mut up = INPUT::default();
    up.r#type = INPUT_KEYBOARD;
    up.Anonymous = INPUT_0 {
        ki: KEYBDINPUT {
            wVk: VK_SCROLL,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        },
    };

    unsafe {
        SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
    }
}

/// Taps Scroll Lock twice in a row -- net zero change to the indicator
/// state -- which is enough to reset the system's idle/lock timer.
/// The gap between taps isn't load-bearing; 100ms is a safe default.
fn double_tap_scroll_lock() {
    tap_scroll_lock();
    thread::sleep(Duration::from_millis(100));
    tap_scroll_lock();
}

/// Spawns the background timer thread for the life of the process.
/// `interval_secs` is shared with the UI thread -- changing it takes effect
/// after the current sleep finishes (not instantly), which is fine for a
/// "every few minutes" idle-reset and avoids needing a wake signal.
pub fn start(interval_secs: Arc<AtomicU64>) {
    thread::spawn(move || loop {
        let secs = interval_secs.load(Ordering::Relaxed).max(1);
        thread::sleep(Duration::from_secs(secs));
        double_tap_scroll_lock();
    });
}
