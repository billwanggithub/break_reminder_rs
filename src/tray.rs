use std::cell::RefCell;
use std::rc::Rc;

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use crate::state::{show_reminder, show_settings, AppState};

thread_local! {
    static APP_STATE: RefCell<Option<Rc<RefCell<AppState>>>> = const { RefCell::new(None) };
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

pub struct Tray {
    _icon: TrayIcon,
}

pub fn build() -> Tray {
    let icon = load_icon();

    let menu = Menu::new();
    let settings_item = MenuItem::new("設定 (Settings)", true, None);
    let break_item = MenuItem::new("立刻休息 (Break Now)", true, None);
    let exit_item = MenuItem::new("離開 (Exit)", true, None);
    menu.append(&settings_item).unwrap();
    menu.append(&break_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&exit_item).unwrap();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("休息提醒小幫手 (雙擊開啟設定)")
        .with_icon(icon)
        .build()
        .expect("failed to build tray icon");

    let settings_id = settings_item.id().clone();
    let break_id = break_item.id().clone();
    let exit_id = exit_item.id().clone();
    let tray_id = tray.id().clone();

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let settings_id = settings_id.clone();
        let break_id = break_id.clone();
        let exit_id = exit_id.clone();
        slint::invoke_from_event_loop(move || {
            with_state(|state| {
                if event.id == settings_id {
                    show_settings(state);
                } else if event.id == break_id {
                    show_reminder(state);
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
