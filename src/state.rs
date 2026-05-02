use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, Timer, TimerMode, Weak};

use crate::settings::{self, Settings};
use crate::{MainWindow, ReminderWindow};

pub struct AppState {
    pub settings: Settings,
    pub settings_path: PathBuf,
    pub settings_window: Weak<MainWindow>,
    pub reminder_window: Option<ReminderWindow>,
    pub timer: Timer,
}

impl AppState {
    pub fn new(settings_path: PathBuf, settings: Settings, settings_window: Weak<MainWindow>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            settings,
            settings_path,
            settings_window,
            reminder_window: None,
            timer: Timer::default(),
        }))
    }

    pub fn restart_timer(state: &Rc<RefCell<Self>>) {
        let interval_minutes = state.borrow().settings.interval_minutes;
        let weak_state = Rc::downgrade(state);
        state.borrow().timer.start(
            TimerMode::Repeated,
            Duration::from_secs(interval_minutes as u64 * 60),
            move || {
                if let Some(s) = weak_state.upgrade() {
                    show_reminder(&s);
                }
            },
        );
    }

    pub fn stop_timer(&self) {
        self.timer.stop();
    }

    pub fn save(&self) {
        settings::save(&self.settings_path, &self.settings);
    }
}

pub fn show_settings(state: &Rc<RefCell<AppState>>) {
    let Some(window) = state.borrow().settings_window.upgrade() else { return };
    window.set_interval_minutes(state.borrow().settings.interval_minutes as i32);
    window.show().ok();
}

pub fn show_reminder(state: &Rc<RefCell<AppState>>) {
    if state.borrow().reminder_window.is_some() {
        if let Some(w) = state.borrow().reminder_window.as_ref().map(|w| w.as_weak()) {
            if let Some(w) = w.upgrade() {
                w.show().ok();
            }
        }
        return;
    }

    state.borrow().stop_timer();

    let window = match ReminderWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[break_reminder_rs] failed to create reminder window: {e}");
            AppState::restart_timer(state);
            return;
        }
    };

    if state.borrow().settings.play_sound {
        crate::sound::play_alert();
    }

    window.on_dismissed({
        let weak_state = Rc::downgrade(state);
        let weak_window = window.as_weak();
        move || {
            if let Some(w) = weak_window.upgrade() {
                w.hide().ok();
            }
            if let Some(state) = weak_state.upgrade() {
                state.borrow_mut().reminder_window = None;
                AppState::restart_timer(&state);
            }
        }
    });

    window.show().ok();
    state.borrow_mut().reminder_window = Some(window);
}
