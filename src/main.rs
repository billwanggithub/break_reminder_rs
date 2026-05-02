#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
mod state;
mod tray;
mod autostart;
mod sound;

use slint::ComponentHandle;
use state::AppState;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let path = settings::settings_path();
    let loaded = settings::load(&path);

    if let Ok(exe) = std::env::current_exe() {
        autostart::apply(loaded.auto_start, &exe);
    } else {
        eprintln!("[break_reminder_rs] could not determine current exe path; skipping autostart sync");
    }

    let settings_window = MainWindow::new()?;
    let app_state = AppState::new(path, loaded, settings_window.as_weak());

    settings_window.set_interval_minutes(app_state.borrow().settings.interval_minutes as i32);
    settings_window.set_auto_start(app_state.borrow().settings.auto_start);

    settings_window.on_save_clicked({
        let weak_state = std::rc::Rc::downgrade(&app_state);
        let weak_window = settings_window.as_weak();
        move || {
            let Some(state) = weak_state.upgrade() else { return };
            let Some(window) = weak_window.upgrade() else { return };
            let new_interval = window.get_interval_minutes().max(1) as u32;
            let new_auto_start = window.get_auto_start();
            {
                let mut s = state.borrow_mut();
                s.settings.interval_minutes = new_interval;
                s.settings.auto_start = new_auto_start;
            }
            state.borrow().save();
            AppState::restart_timer(&state);
            if let Ok(exe) = std::env::current_exe() {
                autostart::apply(new_auto_start, &exe);
            }
            window.hide().ok();
        }
    });

    let _tray = tray::build();
    tray::install_state(app_state.clone());

    AppState::restart_timer(&app_state);

    // This is a tray-resident app: hiding the settings or reminder window
    // must not exit the process.
    slint::run_event_loop_until_quit()
}
