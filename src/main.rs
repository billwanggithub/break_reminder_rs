mod settings;
mod state;

use slint::ComponentHandle;
use state::AppState;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let path = settings::settings_path();
    let loaded = settings::load(&path);

    let settings_window = MainWindow::new()?;
    let app_state = AppState::new(path, loaded, settings_window.as_weak());

    settings_window.set_interval_minutes(app_state.borrow().settings.interval_minutes as i32);

    settings_window.on_save_clicked({
        let weak_state = std::rc::Rc::downgrade(&app_state);
        let weak_window = settings_window.as_weak();
        move || {
            let Some(state) = weak_state.upgrade() else { return };
            let Some(window) = weak_window.upgrade() else { return };
            let new_value = window.get_interval_minutes().max(1) as u32;
            state.borrow_mut().settings.interval_minutes = new_value;
            state.borrow().save();
            AppState::restart_timer(&state);
            window.hide().ok();
        }
    });

    AppState::restart_timer(&app_state);

    // Temporary: open settings window for development. Tray will replace this in Task 5.
    settings_window.show()?;

    slint::run_event_loop()
}
