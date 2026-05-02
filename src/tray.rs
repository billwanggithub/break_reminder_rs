use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use crate::state::{show_reminder, show_settings, AppState};

// `tray-icon`'s event handlers require `Fn + Send + Sync + 'static`, but
// `Rc<RefCell<AppState>>` is `!Send + !Sync`. Handler closures capture only
// `MenuId`/`TrayIconId` (which are Send + Sync), then hop to the UI thread
// via `slint::invoke_from_event_loop` and recover state from this thread_local.
thread_local! {
    static APP_STATE: RefCell<Option<Rc<RefCell<AppState>>>> = const { RefCell::new(None) };
    static MUTE_VISUALS: RefCell<Option<MuteVisuals>> = const { RefCell::new(None) };
}

struct MuteVisuals {
    mute_item: Arc<CheckMenuItem>,
    tray: Arc<TrayIcon>,
}

pub fn install_state(state: Rc<RefCell<AppState>>) {
    APP_STATE.with(|cell| *cell.borrow_mut() = Some(state));
}

fn with_state<F: FnOnce(&Rc<RefCell<AppState>>)>(f: F) {
    APP_STATE.with(|cell| {
        if let Some(state) = cell.borrow().as_ref() {
            f(state);
        }
    });
}

/// Update the tray's mute checkmark and tooltip to match the given state.
/// Called from the tray menu handler AND from main.rs's save closure.
pub fn refresh_mute_visuals(muted: bool) {
    MUTE_VISUALS.with(|cell| {
        if let Some(v) = cell.borrow().as_ref() {
            v.mute_item.set_checked(muted);
            let tooltip = if muted {
                "休息提醒小幫手 (請勿打擾中)"
            } else {
                "休息提醒小幫手 (雙擊開啟設定)"
            };
            let _ = v.tray.set_tooltip(Some(tooltip));
        }
    });
}

pub struct Tray {
    _icon: Arc<TrayIcon>,
}

pub fn build(initial_muted: bool) -> Tray {
    let icon = load_icon();

    let menu = Menu::new();
    let mute_item = CheckMenuItem::new("請勿打擾", true, initial_muted, None);
    let settings_item = MenuItem::new("設定 (Settings)", true, None);
    let break_item = MenuItem::new("立刻休息 (Break Now)", true, None);
    let exit_item = MenuItem::new("離開 (Exit)", true, None);
    menu.append(&mute_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&settings_item).unwrap();
    menu.append(&break_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&exit_item).unwrap();

    let tooltip = if initial_muted {
        "休息提醒小幫手 (請勿打擾中)"
    } else {
        "休息提醒小幫手 (雙擊開啟設定)"
    };

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(tooltip)
        .with_icon(icon)
        .build()
        .expect("failed to build tray icon");

    let tray = Arc::new(tray);
    let mute_item = Arc::new(mute_item);

    MUTE_VISUALS.with(|cell| {
        *cell.borrow_mut() = Some(MuteVisuals {
            mute_item: mute_item.clone(),
            tray: tray.clone(),
        });
    });

    let mute_id = mute_item.id().clone();
    let settings_id = settings_item.id().clone();
    let break_id = break_item.id().clone();
    let exit_id = exit_item.id().clone();
    let tray_id = tray.id().clone();

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        // The outer handler is `Fn`, so each invocation must clone the ids
        // afresh before moving them into the `FnOnce` posted to the UI thread.
        let mute_id = mute_id.clone();
        let settings_id = settings_id.clone();
        let break_id = break_id.clone();
        let exit_id = exit_id.clone();
        slint::invoke_from_event_loop(move || {
            with_state(|state| {
                if event.id == mute_id {
                    let new_value = !state.borrow().settings.muted;
                    state.borrow_mut().settings.muted = new_value;
                    state.borrow().save();
                    AppState::restart_timer(state);
                    AppState::restart_schedule_timer(state);
                    if !new_value {
                        crate::schedule::check_due_reminders(state);
                    }
                    refresh_mute_visuals(new_value);
                } else if event.id == settings_id {
                    show_settings(state);
                } else if event.id == break_id {
                    show_reminder(state, None);
                } else if event.id == exit_id {
                    slint::quit_event_loop().ok();
                }
            });
        }).ok();
    }));

    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        let TrayIconEvent::DoubleClick { id, button: MouseButton::Left, .. } = event else { return };
        if id != tray_id { return };
        slint::invoke_from_event_loop(move || {
            with_state(|state| show_settings(state));
        }).ok();
    }));

    Tray { _icon: tray }
}

fn load_icon() -> Icon {
    let bytes = include_bytes!("../assets/app.ico");
    let image = image::load_from_memory(bytes)
        .expect("failed to decode app.ico")
        .into_rgba8();
    let (w, h) = image.dimensions();
    Icon::from_rgba(image.into_raw(), w, h).expect("failed to build tray icon")
}
