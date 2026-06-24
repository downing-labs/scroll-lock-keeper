// Scroll Lock Keeper
// Checkpoint 3: settings cog + persisted interval.
#![windows_subsystem = "windows"]

extern crate native_windows_gui as nwg;
use nwg::NativeUi;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

mod scroll_lock;
mod settings;
mod update_check;

#[derive(Default)]
pub struct App {
    window: nwg::Window,
    layout: nwg::GridLayout,
    minimize_btn: nwg::Button,
    settings_btn: nwg::Button,
    exit_btn: nwg::Button,
    kofi_btn: nwg::Button,
    update_btn: nwg::Button,
    update_bitmap_upgrade: nwg::Bitmap,
    update_timer: nwg::AnimationTimer,
    tray: nwg::TrayNotification,
    tray_menu: nwg::Menu,
    tray_menu_show: nwg::MenuItem,
    tray_menu_exit: nwg::MenuItem,
    icon: nwg::Icon,

    settings_window: nwg::Window,
    settings_layout: nwg::GridLayout,
    interval_label: nwg::Label,
    interval_input: nwg::NumberSelect,
    settings_save_btn: nwg::Button,
    settings_cancel_btn: nwg::Button,

    help_btn: nwg::Button,
    help_window: nwg::Window,
    help_layout: nwg::GridLayout,
    help_text: nwg::Label,
    help_footer: nwg::Label,
    help_close_btn: nwg::Button,

    interval_secs: Arc<AtomicU64>,
    update_status: update_check::UpdateStatus,
}

impl App {
    fn minimize_to_tray(&self) {
        self.window.set_visible(false);
    }

    fn restore_from_tray(&self) {
        self.window.set_visible(true);
        self.window.restore();
        self.window.set_focus();
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn open_settings(&self) {
        let current_minutes = (self.interval_secs.load(Ordering::Relaxed) / 60).max(1);
        self.interval_input.set_data(nwg::NumberSelectData::Int {
            value: current_minutes as i64,
            step: 1,
            max: 120,
            min: 1,
        });
        self.settings_window.set_visible(true);
        self.settings_window.set_focus();
    }

    fn save_settings(&self) {
        let minutes = match self.interval_input.data() {
            nwg::NumberSelectData::Int { value, .. } => value.max(1) as u64,
            _ => 4,
        };
        let secs = minutes * 60;
        self.interval_secs.store(secs, Ordering::Relaxed);
        let _ = settings::save(&settings::Settings { interval_secs: secs });
        self.settings_window.set_visible(false);
    }

    fn cancel_settings(&self) {
        self.settings_window.set_visible(false);
    }

    fn open_help(&self) {
        self.help_window.set_visible(true);
        self.help_window.set_focus();
    }

    fn view_release(&self) {
        let url = self
            .update_status
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .map(|info| info.url)
            .unwrap_or_else(|| "https://github.com/downing-labs/scroll-lock-keeper/releases".to_string());
        open_url(&url);
    }

    fn close_help(&self) {
        self.help_window.set_visible(false);
    }

    fn open_kofi_link(&self) {
        open_url("https://ko-fi.com/hackpig1974");
    }
}

/// Opens a URL in the user's default browser via the same Win32 shell API
/// Explorer itself uses -- no extra process spawning, no extra crate.
fn open_url(url: &str) {
    use windows::core::HSTRING;
    let operation = HSTRING::from("open");
    let file = HSTRING::from(url);
    let empty = HSTRING::from("");
    unsafe {
        let _ = windows::Win32::UI::Shell::ShellExecuteW(
            None,
            &operation,
            &file,
            &empty,
            &empty,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
    }
}

mod app_ui {
    use super::*;

    pub struct AppUi {
        inner: Rc<App>,
        handlers: RefCell<Vec<nwg::EventHandler>>,
    }

    impl nwg::NativeUi<AppUi> for App {
        fn build_ui(mut data: App) -> Result<AppUi, nwg::NwgError> {
            let loaded = settings::load();
            data.interval_secs.store(loaded.interval_secs, Ordering::Relaxed);

            let embed = nwg::EmbedResource::load(None)?;
            data.icon = embed.icon(2, None).unwrap_or_else(|| {
                let mut fallback = nwg::Icon::default();
                let _ = nwg::Icon::builder()
                    .source_system(Some(nwg::OemIcon::WinLogo))
                    .build(&mut fallback);
                fallback
            });
            data.update_bitmap_upgrade = embed.bitmap(3, None).unwrap_or_default();

            nwg::Window::builder()
                .size((168, 58))
                .position((400, 300))
                .title("Scroll Lock Keeper")
                .icon(Some(&data.icon))
                .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
                .build(&mut data.window)?;

            nwg::Button::builder()
                .text("Minimize")
                .position((4, 4))
                .size((58, 22))
                .parent(&data.window)
                .build(&mut data.minimize_btn)?;

            nwg::Button::builder()
                .text("⚙")
                .position((65, 4))
                .size((24, 22))
                .parent(&data.window)
                .build(&mut data.settings_btn)?;

            nwg::Button::builder()
                .text("?")
                .position((92, 4))
                .size((24, 22))
                .parent(&data.window)
                .build(&mut data.help_btn)?;

            nwg::Button::builder()
                .text("Exit")
                .position((119, 4))
                .size((40, 22))
                .parent(&data.window)
                .build(&mut data.exit_btn)?;

            nwg::Button::builder()
                .text("☕ Support")
                .position((4, 28))
                .size((85, 22))
                .parent(&data.window)
                .build(&mut data.kofi_btn)?;

            nwg::Button::builder()
                .text(&format!("v{}", env!("CARGO_PKG_VERSION")))
                .position((92, 28))
                .size((67, 22))
                .parent(&data.window)
                .build(&mut data.update_btn)?;

            nwg::AnimationTimer::builder()
                .parent(&data.window)
                .interval(std::time::Duration::from_millis(2000))
                .active(true)
                .build(&mut data.update_timer)?;

            nwg::Menu::builder()
                .popup(true)
                .parent(&data.window)
                .build(&mut data.tray_menu)?;

            nwg::MenuItem::builder()
                .text("Show")
                .parent(&data.tray_menu)
                .build(&mut data.tray_menu_show)?;

            nwg::MenuItem::builder()
                .text("Exit")
                .parent(&data.tray_menu)
                .build(&mut data.tray_menu_exit)?;

            nwg::TrayNotification::builder()
                .parent(&data.window)
                .icon(Some(&data.icon))
                .tip(Some("Scroll Lock Keeper"))
                .build(&mut data.tray)?;

            nwg::Window::builder()
                .size((180, 116))
                .position((440, 340))
                .title("Settings")
                .icon(Some(&data.icon))
                .parent(Some(&data.window))
                .flags(nwg::WindowFlags::WINDOW)
                .build(&mut data.settings_window)?;

            nwg::Label::builder()
                .text("Toggle every (min):")
                .position((8, 12))
                .size((90, 32))
                .parent(&data.settings_window)
                .build(&mut data.interval_label)?;

            nwg::NumberSelect::builder()
                .value_int(4)
                .step_int(1)
                .min_int(1)
                .max_int(120)
                .position((100, 8))
                .size((68, 24))
                .parent(&data.settings_window)
                .build(&mut data.interval_input)?;

            nwg::Button::builder()
                .text("Save")
                .position((8, 58))
                .size((78, 24))
                .parent(&data.settings_window)
                .build(&mut data.settings_save_btn)?;

            nwg::Button::builder()
                .text("Cancel")
                .position((90, 58))
                .size((78, 24))
                .parent(&data.settings_window)
                .build(&mut data.settings_cancel_btn)?;

            let help_body = format!(
                "Scroll Lock Keeper v{}\r\n\r\n\
                This keeps your screen from locking or showing a screensaver during things like video calls and presentations. It works by tapping Scroll Lock twice every few minutes -- toggling it on, then back off -- which resets Windows' idle timer without changing your actual Scroll Lock state. The interval is configurable from the gear icon.\r\n\r\n\
                Minimize sends this to the tray. Exit closes it completely.",
                env!("CARGO_PKG_VERSION")
            );

            nwg::Window::builder()
                .size((280, 260))
                .position((420, 300))
                .title("Help")
                .icon(Some(&data.icon))
                .parent(Some(&data.window))
                .flags(nwg::WindowFlags::WINDOW)
                .build(&mut data.help_window)?;

            nwg::Label::builder()
                .text(&help_body)
                .position((8, 8))
                .size((264, 166))
                .parent(&data.help_window)
                .build(&mut data.help_text)?;

            nwg::Label::builder()
                .text("Developed by Downing-Labs.org 2026")
                .position((8, 182))
                .size((264, 18))
                .parent(&data.help_window)
                .build(&mut data.help_footer)?;

            nwg::Button::builder()
                .text("OK")
                .position((110, 208))
                .size((60, 24))
                .parent(&data.help_window)
                .build(&mut data.help_close_btn)?;

            let inner = Rc::new(data);

            let h1_ui = Rc::downgrade(&inner);
            let handler1 = nwg::full_bind_event_handler(&inner.window.handle, move |evt, _data, handle| {
                if let Some(evt_ui) = h1_ui.upgrade() {
                    route_event(&evt_ui, evt, handle);
                }
            });

            let h2_ui = Rc::downgrade(&inner);
            let handler2 = nwg::full_bind_event_handler(&inner.settings_window.handle, move |evt, _data, handle| {
                if let Some(evt_ui) = h2_ui.upgrade() {
                    route_event(&evt_ui, evt, handle);
                }
            });

            let h3_ui = Rc::downgrade(&inner);
            let handler3 = nwg::full_bind_event_handler(&inner.help_window.handle, move |evt, _data, handle| {
                if let Some(evt_ui) = h3_ui.upgrade() {
                    route_event(&evt_ui, evt, handle);
                }
            });

            Ok(AppUi {
                inner,
                handlers: RefCell::new(vec![handler1, handler2, handler3]),
            })
        }
    }

    fn route_event(evt_ui: &Rc<App>, evt: nwg::Event, handle: nwg::ControlHandle) {
        use nwg::Event as E;
        match evt {
            E::OnButtonClick => {
                if handle == evt_ui.minimize_btn {
                    App::minimize_to_tray(evt_ui);
                } else if handle == evt_ui.settings_btn {
                    App::open_settings(evt_ui);
                } else if handle == evt_ui.exit_btn {
                    App::exit(evt_ui);
                } else if handle == evt_ui.settings_save_btn {
                    App::save_settings(evt_ui);
                } else if handle == evt_ui.settings_cancel_btn {
                    App::cancel_settings(evt_ui);
                } else if handle == evt_ui.help_btn {
                    App::open_help(evt_ui);
                } else if handle == evt_ui.help_close_btn {
                    App::close_help(evt_ui);
                } else if handle == evt_ui.kofi_btn {
                    App::open_kofi_link(evt_ui);
                } else if handle == evt_ui.update_btn {
                    App::view_release(evt_ui);
                }
            }
            E::OnMenuItemSelected => {
                if handle == evt_ui.tray_menu_show {
                    App::restore_from_tray(evt_ui);
                } else if handle == evt_ui.tray_menu_exit {
                    App::exit(evt_ui);
                }
            }
            E::OnMousePress(nwg::MousePressEvent::MousePressLeftUp) => {
                if handle == evt_ui.tray {
                    App::restore_from_tray(evt_ui);
                }
            }
            E::OnTimerTick => {
                if handle == evt_ui.update_timer {
                    if evt_ui.update_status.lock().ok().map(|g| g.is_some()).unwrap_or(false) {
                        // The default state is a plain native-text button (so
                        // it actually looks/feels clickable). Switching to a
                        // colored face requires BS_BITMAP, which isn't set
                        // until now -- added at runtime via GWL_STYLE.
                        const BS_BITMAP: isize = 0x0000_0080;
                        if let Some(raw_hwnd) = evt_ui.update_btn.handle.hwnd() {
                            unsafe {
                                use windows::Win32::Foundation::HWND;
                                use windows::Win32::UI::WindowsAndMessaging::{
                                    GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE,
                                };
                                let win_hwnd = HWND(raw_hwnd as *mut std::ffi::c_void);
                                let style = GetWindowLongPtrW(win_hwnd, GWL_STYLE);
                                SetWindowLongPtrW(win_hwnd, GWL_STYLE, style | BS_BITMAP);
                            }
                        }
                        evt_ui.update_btn.set_bitmap(Some(&evt_ui.update_bitmap_upgrade));
                        evt_ui.update_timer.stop();
                        // The app mostly runs minimized/tray-only -- surface
                        // itself so the update isn't missed.
                        App::restore_from_tray(evt_ui);
                    }
                }
            }
            E::OnContextMenu => {
                if handle == evt_ui.tray {
                    let (x, y) = nwg::GlobalCursor::position();
                    evt_ui.tray_menu.popup(x, y);
                }
            }
            E::OnWindowClose => {
                if handle == evt_ui.window {
                    App::minimize_to_tray(evt_ui);
                } else if handle == evt_ui.settings_window {
                    App::cancel_settings(evt_ui);
                } else if handle == evt_ui.help_window {
                    App::close_help(evt_ui);
                }
            }
            _ => {}
        }
    }

    impl Drop for AppUi {
        fn drop(&mut self) {
            for handler in self.handlers.borrow().iter() {
                nwg::unbind_event_handler(handler);
            }
        }
    }

    impl Deref for AppUi {
        type Target = App;
        fn deref(&self) -> &App {
            &self.inner
        }
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .family("Segoe UI")
        .size(13)
        .build(&mut font)
        .expect("Failed to build default font");
    nwg::Font::set_global_default(Some(font));

    let app = App::build_ui(Default::default()).expect("Failed to build UI");
    scroll_lock::start(app.interval_secs.clone());
    update_check::check_async(app.update_status.clone(), app.interval_secs.clone());
    nwg::dispatch_thread_events();
}
