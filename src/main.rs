slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    window.on_save_clicked({
        let weak = window.as_weak();
        move || {
            if let Some(w) = weak.upgrade() {
                w.hide().ok();
            }
        }
    });
    window.run()
}
