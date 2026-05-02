#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
mod state;
mod tray;
mod autostart;
mod sound;
mod schedule;

use std::rc::Rc;

use chrono::{NaiveTime, Timelike};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use schedule::{DayOfWeekMask, ScheduledReminder};
use state::AppState;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let path = settings::settings_path();
    let mut loaded = settings::load(&path);

    // If the JSON didn't include any scheduled reminders (fresh install or
    // missing field), seed defaults. We can't distinguish "empty array" from
    // "missing field" through serde alone — both deserialise to vec![] — but
    // the user-facing behaviour is the same: an empty list is unusable, so
    // seed defaults whenever it's empty.
    if loaded.scheduled_reminders.is_empty() {
        loaded.scheduled_reminders = schedule::seed_defaults();
        settings::save(&path, &loaded);
    }

    if let Ok(exe) = std::env::current_exe() {
        autostart::apply(loaded.auto_start, &exe);
    } else {
        eprintln!("[break_reminder_rs] could not determine current exe path; skipping autostart sync");
    }

    let settings_window = MainWindow::new()?;
    let app_state = AppState::new(path, loaded, settings_window.as_weak());

    let reminders_model: Rc<VecModel<ScheduledReminderRow>> = Rc::new(VecModel::from(
        rows_from_reminders(&app_state.borrow().settings.scheduled_reminders),
    ));
    settings_window.set_reminders(ModelRc::from(reminders_model.clone()));

    settings_window.set_interval_minutes(app_state.borrow().settings.interval_minutes as i32);
    settings_window.set_auto_start(app_state.borrow().settings.auto_start);
    settings_window.set_play_sound(app_state.borrow().settings.play_sound);

    settings_window.on_add_reminder({
        let model = reminders_model.clone();
        move || {
            model.push(ScheduledReminderRow {
                enabled: true,
                time_text: "12:00".into(),
                mon: true, tue: true, wed: true, thu: true, fri: true,
                sat: false, sun: false,
                message: "提醒訊息".into(),
            });
        }
    });

    settings_window.on_delete_reminder({
        let model = reminders_model.clone();
        move |index: i32| {
            if index < 0 { return; }
            let i = index as usize;
            if i < model.row_count() {
                model.remove(i);
            }
        }
    });

    settings_window.on_update_row({
        let model = reminders_model.clone();
        move |index: i32, row: ScheduledReminderRow| {
            if index < 0 { return; }
            let i = index as usize;
            if i < model.row_count() {
                model.set_row_data(i, row);
            }
        }
    });

    settings_window.on_save_clicked({
        let weak_state = Rc::downgrade(&app_state);
        let weak_window = settings_window.as_weak();
        let model = reminders_model.clone();
        move || {
            let Some(state) = weak_state.upgrade() else { return };
            let Some(window) = weak_window.upgrade() else { return };
            let new_interval = window.get_interval_minutes().max(1) as u32;
            let new_auto_start = window.get_auto_start();
            let new_play_sound = window.get_play_sound();
            let new_reminders = reminders_from_model(&model);
            {
                let mut s = state.borrow_mut();
                s.settings.interval_minutes = new_interval;
                s.settings.auto_start = new_auto_start;
                s.settings.play_sound = new_play_sound;
                s.settings.scheduled_reminders = new_reminders;
            }
            state.borrow().save();
            AppState::restart_timer(&state);
            AppState::restart_schedule_timer(&state);
            schedule::check_due_reminders(&state);
            if let Ok(exe) = std::env::current_exe() {
                autostart::apply(new_auto_start, &exe);
            }
            window.hide().ok();
        }
    });

    let _tray = tray::build(app_state.borrow().settings.muted);
    tray::install_state(app_state.clone());

    AppState::restart_timer(&app_state);
    AppState::restart_schedule_timer(&app_state);
    schedule::check_due_reminders(&app_state);

    slint::run_event_loop_until_quit()
}

fn rows_from_reminders(reminders: &[ScheduledReminder]) -> Vec<ScheduledReminderRow> {
    reminders
        .iter()
        .map(|r| ScheduledReminderRow {
            enabled: r.enabled,
            time_text: format!("{:02}:{:02}", r.time.hour(), r.time.minute()).into(),
            mon: r.days.contains(DayOfWeekMask::MON),
            tue: r.days.contains(DayOfWeekMask::TUE),
            wed: r.days.contains(DayOfWeekMask::WED),
            thu: r.days.contains(DayOfWeekMask::THU),
            fri: r.days.contains(DayOfWeekMask::FRI),
            sat: r.days.contains(DayOfWeekMask::SAT),
            sun: r.days.contains(DayOfWeekMask::SUN),
            message: r.message.clone().into(),
        })
        .collect()
}

fn reminders_from_model(model: &VecModel<ScheduledReminderRow>) -> Vec<ScheduledReminder> {
    let mut out = Vec::with_capacity(model.row_count());
    for i in 0..model.row_count() {
        let Some(row) = model.row_data(i) else { continue };
        let time = NaiveTime::parse_from_str(row.time_text.as_str(), "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let mut days = DayOfWeekMask::empty();
        if row.mon { days |= DayOfWeekMask::MON; }
        if row.tue { days |= DayOfWeekMask::TUE; }
        if row.wed { days |= DayOfWeekMask::WED; }
        if row.thu { days |= DayOfWeekMask::THU; }
        if row.fri { days |= DayOfWeekMask::FRI; }
        if row.sat { days |= DayOfWeekMask::SAT; }
        if row.sun { days |= DayOfWeekMask::SUN; }
        out.push(ScheduledReminder {
            enabled: row.enabled,
            time,
            days,
            message: row.message.to_string(),
            last_fired_date: None,
        });
    }
    out
}
