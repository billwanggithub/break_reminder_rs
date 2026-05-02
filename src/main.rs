mod settings;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let path = settings::settings_path();
    let mut current = settings::load(&path);

    let window = MainWindow::new()?;
    window.set_interval_minutes(current.interval_minutes as i32);

    window.on_save_clicked({
        let weak = window.as_weak();
        let path = path.clone();
        move || {
            let Some(w) = weak.upgrade() else { return };
            current_set_and_save(w.get_interval_minutes(), &path, &mut current.clone());
            w.hide().ok();
        }
    });
    window.run()
}

fn current_set_and_save(value: i32, path: &std::path::Path, current: &mut settings::Settings) {
    current.interval_minutes = value.max(1) as u32;
    settings::save(path, current);
}
