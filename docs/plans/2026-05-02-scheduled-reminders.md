# Scheduled Reminders Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add WPF-parity scheduled reminders: a list of named reminders that fire on selected days at specific times, each with a custom message shown in the reminder window.

**Architecture:** A new `schedule` module owns the model + JSON serde + due-check logic. A second `slint::Timer` runs every 30 seconds. The reminder window's message becomes a property; `show_reminder` accepts `Option<&str>`. Settings UI gains a scrollable editable list with add/delete.

**Tech Stack:** `bitflags 2` for the day-mask, `chrono 0.4` (default-features off) for time/date types and arithmetic. Slint `VecModel` for the dynamic UI list.

**Spec:** [`docs/specs/2026-05-02-scheduled-reminders-design.md`](../specs/2026-05-02-scheduled-reminders-design.md)

**Note on testing:** No automated tests, matching prior features. Manual checklist runs at the end of Task 7.

---

## Task 1: Add `bitflags` and `chrono` dependencies

**Files:**
- Modify: `Cargo.toml`

This task only adds dependencies. The new modules using them come in later tasks. Build verifies the deps resolve.

- [ ] **Step 1: Append `bitflags` and `chrono` to `[dependencies]`**

Open `d:\github\break_reminder_rs\Cargo.toml`. Append two new lines at the end of the `[dependencies]` block (after `winreg = "0.52"`):
```toml
bitflags = "2"
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
```

The full `[dependencies]` block should now be:
```toml
[dependencies]
slint = "1.8"
tray-icon = "0.19"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"
image = { version = "0.25", default-features = false, features = ["ico"] }
winreg = "0.52"
bitflags = "2"
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
```

- [ ] **Step 2: Build to verify deps resolve and compile**

Run from `d:\github\break_reminder_rs`:
```powershell
cargo build
```
Expected: succeeds with zero warnings. First build pulls and compiles `bitflags`, `chrono`, and a few transitive deps (~30-60 seconds — use 180000ms timeout).

If hard errors occur, report verbatim and STOP.

- [ ] **Step 3: Commit**

```powershell
git add .
git commit -m "build: add bitflags and chrono dependencies for scheduled reminders"
```

---

## Task 2: Create the `schedule` module skeleton with `DayOfWeekMask`

**Files:**
- Create: `src/schedule.rs`
- Modify: `src/main.rs` (add `mod schedule;`)

Introduce the bitflags type and its custom serde. Nothing else uses it yet, but adding `mod schedule;` keeps it part of the compilation so syntax errors get caught immediately.

- [ ] **Step 1: Create `src/schedule.rs`**

Create `d:\github\break_reminder_rs\src\schedule.rs` with EXACTLY:
```rust
use bitflags::bitflags;
use chrono::Datelike;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DayOfWeekMask: u8 {
        const SUN = 1 << 0;
        const MON = 1 << 1;
        const TUE = 1 << 2;
        const WED = 1 << 3;
        const THU = 1 << 4;
        const FRI = 1 << 5;
        const SAT = 1 << 6;

        const WEEKDAYS = Self::MON.bits() | Self::TUE.bits() | Self::WED.bits() | Self::THU.bits() | Self::FRI.bits();
        const ALL = Self::SUN.bits() | Self::MON.bits() | Self::TUE.bits() | Self::WED.bits()
                  | Self::THU.bits() | Self::FRI.bits() | Self::SAT.bits();
    }
}

impl DayOfWeekMask {
    /// True if the mask contains the given calendar weekday.
    pub fn matches(&self, weekday: chrono::Weekday) -> bool {
        let bit = match weekday {
            chrono::Weekday::Sun => Self::SUN,
            chrono::Weekday::Mon => Self::MON,
            chrono::Weekday::Tue => Self::TUE,
            chrono::Weekday::Wed => Self::WED,
            chrono::Weekday::Thu => Self::THU,
            chrono::Weekday::Fri => Self::FRI,
            chrono::Weekday::Sat => Self::SAT,
        };
        self.contains(bit)
    }

    fn to_wpf_string(self) -> String {
        if self.is_empty() {
            return "None".to_string();
        }
        if self == Self::ALL {
            return "All".to_string();
        }
        if self == Self::WEEKDAYS {
            return "Weekdays".to_string();
        }
        // Calendar order: Sun, Mon, Tue, Wed, Thu, Fri, Sat.
        let parts: Vec<&str> = [
            (Self::SUN, "Sun"),
            (Self::MON, "Mon"),
            (Self::TUE, "Tue"),
            (Self::WED, "Wed"),
            (Self::THU, "Thu"),
            (Self::FRI, "Fri"),
            (Self::SAT, "Sat"),
        ]
        .into_iter()
        .filter_map(|(bit, name)| if self.contains(bit) { Some(name) } else { None })
        .collect();
        parts.join(", ")
    }

    fn from_wpf_string(s: &str) -> Self {
        let mut mask = Self::empty();
        // Tolerant: accept comma OR pipe separator (WPF C# enum.ToString uses "|").
        for token in s.split(|c: char| c == ',' || c == '|') {
            let t = token.trim();
            match t {
                "" => {}
                "None" => {} // explicit None: leave mask empty
                "All" => return Self::ALL,
                "Weekdays" => mask |= Self::WEEKDAYS,
                "Sun" => mask |= Self::SUN,
                "Mon" => mask |= Self::MON,
                "Tue" => mask |= Self::TUE,
                "Wed" => mask |= Self::WED,
                "Thu" => mask |= Self::THU,
                "Fri" => mask |= Self::FRI,
                "Sat" => mask |= Self::SAT,
                other => eprintln!("[break_reminder_rs] unknown day token '{other}', skipping"),
            }
        }
        mask
    }
}

impl Serialize for DayOfWeekMask {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_wpf_string())
    }
}

impl<'de> Deserialize<'de> for DayOfWeekMask {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        Ok(Self::from_wpf_string(&s))
    }
}

// ScheduledReminder, seed_defaults, and check_due_reminders come in later tasks.

// Suppress dead-code warnings until later tasks consume these.
#[allow(dead_code)]
pub(crate) fn _ensure_compile() {
    let _ = chrono::Local::now().date_naive().weekday();
}
```

The `_ensure_compile` helper exists only to make `chrono` import paths active, since later tasks rely on them and we want syntax issues caught early. It will be removed in Task 5.

- [ ] **Step 2: Add `mod schedule;` to `src/main.rs`**

Open `src/main.rs`. Find the `mod` block:
```rust
mod settings;
mod state;
mod tray;
mod autostart;
mod sound;
```
Replace with:
```rust
mod settings;
mod state;
mod tray;
mod autostart;
mod sound;
mod schedule;
```

- [ ] **Step 3: Build**

Run:
```powershell
cargo build
```
Expected: succeeds. Possible warnings on unused items in `schedule` — fine for now.

If errors, report verbatim and STOP.

- [ ] **Step 4: Commit**

```powershell
git add .
git commit -m "feat: add schedule module skeleton with DayOfWeekMask + WPF serde"
```

---

## Task 3: Add `ScheduledReminder` struct + seed defaults

**Files:**
- Modify: `src/schedule.rs`

Add the reminder struct, its custom (de)serializer that produces WPF-compatible JSON, and a `seed_defaults` function returning the three default reminders.

- [ ] **Step 1: Append the reminder struct and seed function to `schedule.rs`**

Open `d:\github\break_reminder_rs\src\schedule.rs`. Find this block at the bottom:
```rust
// ScheduledReminder, seed_defaults, and check_due_reminders come in later tasks.

// Suppress dead-code warnings until later tasks consume these.
#[allow(dead_code)]
pub(crate) fn _ensure_compile() {
    let _ = chrono::Local::now().date_naive().weekday();
}
```
Replace with EXACTLY:
```rust
use chrono::{NaiveDate, NaiveTime};

#[derive(Debug, Clone)]
pub struct ScheduledReminder {
    pub enabled: bool,
    pub time: NaiveTime,
    pub days: DayOfWeekMask,
    pub message: String,
    pub last_fired_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize)]
struct ReminderJson {
    #[serde(rename = "Enabled", default)]
    enabled: bool,
    #[serde(rename = "Time", default)]
    time: String,
    #[serde(rename = "Days", default)]
    days: DayOfWeekMask,
    #[serde(rename = "Message", default)]
    message: String,
    #[serde(rename = "LastFiredDate", default, skip_serializing_if = "Option::is_none")]
    last_fired_date: Option<String>,
}

impl Serialize for ScheduledReminder {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let json = ReminderJson {
            enabled: self.enabled,
            time: self.time.format("%H:%M").to_string(),
            days: self.days,
            message: self.message.clone(),
            last_fired_date: self.last_fired_date.map(|d| d.format("%Y-%m-%d").to_string()),
        };
        json.serialize(ser)
    }
}

impl<'de> Deserialize<'de> for ScheduledReminder {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let json = ReminderJson::deserialize(de)?;
        let time = NaiveTime::parse_from_str(&json.time, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let last_fired_date = json
            .last_fired_date
            .as_deref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        Ok(Self {
            enabled: json.enabled,
            time,
            days: json.days,
            message: json.message,
            last_fired_date,
        })
    }
}

pub fn seed_defaults() -> Vec<ScheduledReminder> {
    vec![
        ScheduledReminder {
            enabled: true,
            time: NaiveTime::from_hms_opt(11, 55, 0).unwrap(),
            days: DayOfWeekMask::WEEKDAYS,
            message: "吃飯囉！".to_string(),
            last_fired_date: None,
        },
        ScheduledReminder {
            enabled: true,
            time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
            days: DayOfWeekMask::WEEKDAYS,
            message: "下班時間到！".to_string(),
            last_fired_date: None,
        },
        ScheduledReminder {
            enabled: true,
            time: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            days: DayOfWeekMask::ALL,
            message: "該睡覺了！".to_string(),
            last_fired_date: None,
        },
    ]
}
```

- [ ] **Step 2: Build**

Run:
```powershell
cargo build
```
Expected: succeeds. Possible `dead_code` warnings on `seed_defaults` and methods of `ScheduledReminder` — fine until later tasks.

If hard errors appear, report verbatim and STOP.

- [ ] **Step 3: Commit**

```powershell
git add .
git commit -m "feat: add ScheduledReminder struct with WPF-compatible JSON serde"
```

---

## Task 4: Add `scheduled_reminders` field to `Settings`

**Files:**
- Modify: `src/settings.rs`

Wire the new field into `Settings`. Default value is empty `Vec`; load logic in `main.rs` will detect "no field, no entries" and seed defaults via `schedule::seed_defaults()`. We do NOT seed inside `Default::default()` because we want to distinguish "fresh install" from "user explicitly cleared the list" later — but per spec, we treat both the same way (seed only if the field was missing OR the vec was empty AT FIRST LOAD; subsequent saves of an empty list persist as empty).

Actually the simpler rule from the spec: if the JSON is missing the field, seed defaults; if the JSON has the field as `[]`, keep it empty. `#[serde(default)]` on the field gives `vec![]` in both cases — we can't distinguish. We'll resolve this at the load callsite in `main.rs` (Task 7) by passing the raw JSON through `serde_json::Value::get` to detect missing field. For Task 4, just add the field with `#[serde(default)]`.

- [ ] **Step 1: Update `Settings` struct and `Default` impl**

Open `d:\github\break_reminder_rs\src\settings.rs`. Locate the `Settings` struct (currently with three fields). Replace ONLY the struct + the `Default` impl.

After the change, the top of the file (everything before `pub fn settings_path`) should be EXACTLY:
```rust
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::schedule::ScheduledReminder;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(rename = "IntervalMinutes")]
    pub interval_minutes: u32,
    #[serde(rename = "AutoStart", default = "default_auto_start")]
    pub auto_start: bool,
    #[serde(rename = "PlaySound", default)]
    pub play_sound: bool,
    #[serde(rename = "ScheduledReminders", default)]
    pub scheduled_reminders: Vec<ScheduledReminder>,
}

fn default_auto_start() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_minutes: 45,
            auto_start: true,
            play_sound: false,
            scheduled_reminders: Vec::new(),
        }
    }
}
```

The rest of `settings.rs` (`settings_path`, `load`, `save`) remains unchanged.

- [ ] **Step 2: Build**

Run:
```powershell
cargo build
```
Expected: succeeds with zero new warnings.

- [ ] **Step 3: Commit**

```powershell
git add .
git commit -m "feat: add ScheduledReminders field to Settings"
```

---

## Task 5: Add `check_due_reminders` and the schedule timer

**Files:**
- Modify: `src/schedule.rs` (add `check_due_reminders`)
- Modify: `src/state.rs` (add `schedule_timer` field + `restart_schedule_timer`)
- Modify: `src/main.rs` (call `check_due_reminders` on startup; start the schedule timer)

After this task, scheduled reminders fire at their set times. The Settings UI is still missing — the user can only edit JSON by hand. Task 7 adds the UI.

This task is large because the three files form one logical unit (model + timer + main loop wiring). Splitting it would mean a state with `schedule_timer` declared but never started, which is odd to read.

- [ ] **Step 1: Add `check_due_reminders` to `schedule.rs`**

Open `d:\github\break_reminder_rs\src\schedule.rs`. Append at the end of the file:
```rust

use std::cell::RefCell;
use std::rc::Rc;

use crate::state::{show_reminder, AppState};

/// Walk the reminder list. Fire each reminder that is due NOW and hasn't already
/// fired today. Marks `last_fired_date` and persists settings on the first match.
pub fn check_due_reminders(state: &Rc<RefCell<AppState>>) {
    let now = chrono::Local::now();
    let today = now.date_naive();
    let current_time = now.time();

    // Collect indices to fire so we don't borrow `state` mutably while inside
    // the per-reminder iteration (show_reminder borrows state too).
    let mut due_indices: Vec<usize> = Vec::new();
    {
        let s = state.borrow();
        for (i, r) in s.settings.scheduled_reminders.iter().enumerate() {
            if !r.enabled { continue; }
            if r.last_fired_date == Some(today) { continue; }
            if !r.days.matches(today.weekday()) { continue; }
            if current_time < r.time { continue; }
            let elapsed = current_time.signed_duration_since(r.time);
            if elapsed > chrono::Duration::minutes(2) { continue; }
            due_indices.push(i);
        }
    }

    if due_indices.is_empty() {
        return;
    }

    // Mark + save first, then show reminders. This way even if show_reminder
    // panics or a window fails to open, last_fired_date is committed and we
    // won't fire again in 30s.
    let messages: Vec<String> = {
        let mut s = state.borrow_mut();
        let mut msgs = Vec::with_capacity(due_indices.len());
        for &i in &due_indices {
            s.settings.scheduled_reminders[i].last_fired_date = Some(today);
            msgs.push(s.settings.scheduled_reminders[i].message.clone());
        }
        msgs
    };
    state.borrow().save();

    // Show each due reminder. The dedup logic in show_reminder means only the
    // first one that opens a fresh window will visually display; subsequent
    // ones still go through but the "already-shown" path drops their message.
    // This matches the spec's documented limitation (no pending queue).
    for msg in messages {
        show_reminder(state, Some(&msg));
    }
}
```

- [ ] **Step 2: Remove the temporary `_ensure_compile` helper**

Find this block in `schedule.rs`:
```rust
// Suppress dead-code warnings until later tasks consume these.
#[allow(dead_code)]
pub(crate) fn _ensure_compile() {
    let _ = chrono::Local::now().date_naive().weekday();
}
```
Delete it. The `chrono::Local` etc. are now used by `check_due_reminders`.

- [ ] **Step 3: Update `state.rs` — `AppState` struct + `restart_schedule_timer` + new `show_reminder` signature**

Open `d:\github\break_reminder_rs\src\state.rs`. The current `AppState` struct has 5 fields. Replace ONLY the struct and its `new` method to add a sixth field `schedule_timer`.

Find:
```rust
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
```
Replace with:
```rust
pub struct AppState {
    pub settings: Settings,
    pub settings_path: PathBuf,
    pub settings_window: Weak<MainWindow>,
    pub reminder_window: Option<ReminderWindow>,
    pub timer: Timer,
    pub schedule_timer: Timer,
}

impl AppState {
    pub fn new(settings_path: PathBuf, settings: Settings, settings_window: Weak<MainWindow>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            settings,
            settings_path,
            settings_window,
            reminder_window: None,
            timer: Timer::default(),
            schedule_timer: Timer::default(),
        }))
    }
```

- [ ] **Step 4: Add `restart_schedule_timer` method below `restart_timer`**

In `state.rs`, find:
```rust
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
```
Replace with:
```rust
    pub fn restart_timer(state: &Rc<RefCell<Self>>) {
        let interval_minutes = state.borrow().settings.interval_minutes;
        let weak_state = Rc::downgrade(state);
        state.borrow().timer.start(
            TimerMode::Repeated,
            Duration::from_secs(interval_minutes as u64 * 60),
            move || {
                if let Some(s) = weak_state.upgrade() {
                    show_reminder(&s, None);
                }
            },
        );
    }

    pub fn restart_schedule_timer(state: &Rc<RefCell<Self>>) {
        let weak_state = Rc::downgrade(state);
        state.borrow().schedule_timer.start(
            TimerMode::Repeated,
            Duration::from_secs(30),
            move || {
                if let Some(s) = weak_state.upgrade() {
                    crate::schedule::check_due_reminders(&s);
                }
            },
        );
    }
```

Note the `show_reminder(&s, None)` change inside `restart_timer` — Step 5 below changes the signature.

- [ ] **Step 5: Update `show_reminder` signature and callsites in `state.rs`**

Find the function:
```rust
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
```

Replace ONLY the function signature and the body up to the `if state.borrow().settings.play_sound` block:
```rust
pub fn show_reminder(state: &Rc<RefCell<AppState>>, message: Option<&str>) {
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

    if let Some(m) = message {
        window.set_message(m.into());
    }

    if state.borrow().settings.play_sound {
        crate::sound::play_alert();
    }

    window.on_dismissed({
```

The rest of the function (the `on_dismissed` closure and `window.show().ok();` + `state.borrow_mut().reminder_window = Some(window);`) stays unchanged.

The new `set_message(m.into())` call requires the Slint-generated setter, which exists once Task 6 adds `in property <string> message` to `ReminderWindow.slint`. This task therefore depends on Task 6 having compiled successfully. We'll do Task 6 first as Step 6 below.

Wait — re-reading: this task as written changes `state.rs` to call `set_message` BEFORE Task 6 adds the property. That breaks compilation. Reorder: do Step 6 (add Slint property) before Step 5 (call set_message).

Move the Slint update earlier:

- [ ] **Step 5 (corrected order): Update `ReminderWindow.slint` first (Slint property)**

Open `d:\github\break_reminder_rs\ui\ReminderWindow.slint`. Replace its full content with EXACTLY:
```slint
import { Button, VerticalBox } from "std-widgets.slint";

export component ReminderWindow inherits Window {
    title: "該休息囉！";
    width: 500px;
    height: 300px;
    always-on-top: true;

    in property <string> message: "現在立刻離開座位，喝杯水，讓眼睛過濾一下吧！";
    callback dismissed();

    VerticalBox {
        padding: 20px;
        spacing: 16px;

        VerticalLayout {
            alignment: center;
            spacing: 20px;
            Text {
                text: "⏰ 時間到！";
                font-size: 48px;
                horizontal-alignment: center;
            }
            Text {
                text: root.message;
                font-size: 18px;
                horizontal-alignment: center;
                wrap: word-wrap;
            }
        }

        Button {
            text: "我知道了";
            height: 50px;
            clicked => { root.dismissed(); }
        }
    }
}
```

Differences from previous version:
- New `in property <string> message` with the original hardcoded text as default.
- The body Text now binds `text: root.message;` instead of hardcoding the string.

- [ ] **Step 6: Now apply the `state.rs` changes from Steps 3-5 above**

If you haven't already done Steps 3-5 (state.rs struct, restart_schedule_timer, show_reminder signature), do them now in order. Each step in `state.rs` is shown above; apply them sequentially.

- [ ] **Step 7a: Update `tray.rs` to pass `None` to `show_reminder`**

`show_reminder` now takes `Option<&str>`. The tray's "立刻休息" handler must pass `None` (matches WPF behaviour: manual breaks use the default message).

Open `d:\github\break_reminder_rs\src\tray.rs`. Find the menu-event handler block:
```rust
            } else if event.id == break_id {
                show_reminder(&menu_state);
            } else if event.id == exit_id {
```
Replace with:
```rust
            } else if event.id == break_id {
                show_reminder(&menu_state, None);
            } else if event.id == exit_id {
```

There is exactly one `show_reminder` call in `tray.rs` (the menu's "Break Now" item). The `with_state` helper used inside the closure passes `state` as the first arg; we add `None` as the second.

If the file structure has changed and you find a second `show_reminder` callsite (the tray double-click DOES NOT invoke `show_reminder` — it invokes `show_settings`), STOP and report.

- [ ] **Step 7b: Wire startup-time `check_due_reminders` and `restart_schedule_timer` in `main.rs`**

Open `d:\github\break_reminder_rs\src\main.rs`. Find:
```rust
    AppState::restart_timer(&app_state);

    // This is a tray-resident app: hiding the settings or reminder window
    // must not exit the process.
    slint::run_event_loop_until_quit()
}
```
Replace with:
```rust
    AppState::restart_timer(&app_state);
    AppState::restart_schedule_timer(&app_state);

    // Catch any reminders that should have fired before the app launched today.
    schedule::check_due_reminders(&app_state);

    // This is a tray-resident app: hiding the settings or reminder window
    // must not exit the process.
    slint::run_event_loop_until_quit()
}
```

`schedule::check_due_reminders` runs synchronously after both timers start, so any startup-time reminder fires immediately.

- [ ] **Step 8: Build**

Run:
```powershell
cargo build
```
Expected: succeeds with zero new warnings. If you see "method `set_message` not found", verify Step 5 (the .slint update) was done and rerun `cargo build`.

If hard errors, report verbatim and STOP.

- [ ] **Step 9: Commit**

```powershell
git add .
git commit -m "feat: add schedule timer and check_due_reminders dispatch"
```

---

## Task 6: (Merged into Task 5)

Skipped. Both the Slint property and `show_reminder` signature update are inside Task 5 because they must land together. Renumbering remaining tasks accordingly.

---

## Task 7: Settings UI for editing scheduled reminders

**Files:**
- Modify: `ui/MainWindow.slint`
- Modify: `src/main.rs`

This is the largest task because the UI list, the Slint Model wiring, the add/delete/save callbacks, and the JSON-empty-vs-missing seed logic all hang together. After this, the feature is fully usable end-to-end.

- [ ] **Step 1: Overwrite `ui/MainWindow.slint` with the full UI**

Use the Write tool to overwrite `d:\github\break_reminder_rs\ui\MainWindow.slint` with EXACTLY:
```slint
import { SpinBox, Button, CheckBox, LineEdit, ScrollView, VerticalBox } from "std-widgets.slint";

export struct ScheduledReminderRow {
    enabled: bool,
    time-text: string,
    mon: bool, tue: bool, wed: bool, thu: bool, fri: bool, sat: bool, sun: bool,
    message: string,
}

export component MainWindow inherits Window {
    title: "設定 - 休息提醒小幫手";
    width: 720px;
    height: 560px;

    in-out property <int> interval-minutes: 45;
    in-out property <bool> auto-start: true;
    in-out property <bool> play-sound: false;
    in-out property <[ScheduledReminderRow]> reminders;
    callback save-clicked();
    callback add-reminder();
    callback delete-reminder(int);

    VerticalBox {
        padding: 20px;
        spacing: 12px;

        HorizontalLayout {
            spacing: 8px;
            alignment: start;
            Text { text: "休息間隔："; vertical-alignment: center; font-size: 14px; }
            SpinBox {
                minimum: 1;
                maximum: 240;
                value <=> root.interval-minutes;
            }
            Text { text: "分鐘"; vertical-alignment: center; font-size: 14px; }
        }

        CheckBox {
            text: "登入時自動啟動";
            checked <=> root.auto-start;
        }

        CheckBox {
            text: "播放提示音";
            checked <=> root.play-sound;
        }

        Text { text: "排程提醒"; font-size: 14px; font-weight: 700; }

        // Header row.
        HorizontalLayout {
            spacing: 6px;
            Text { text: "啟用"; width: 36px; horizontal-alignment: center; font-size: 11px; color: gray; }
            Text { text: "時間"; width: 60px; horizontal-alignment: center; font-size: 11px; color: gray; }
            Text { text: "一  二  三  四  五  六  日"; width: 224px; horizontal-alignment: center; font-size: 11px; color: gray; }
            Text { text: "訊息"; horizontal-alignment: left; font-size: 11px; color: gray; horizontal-stretch: 1; }
            Text { text: ""; width: 32px; }
        }

        ScrollView {
            height: 200px;
            VerticalLayout {
                spacing: 4px;
                for r[i] in root.reminders : HorizontalLayout {
                    spacing: 6px;
                    CheckBox { width: 36px; checked <=> r.enabled; }
                    LineEdit { width: 60px; text <=> r.time-text; }
                    HorizontalLayout {
                        width: 224px;
                        spacing: 2px;
                        CheckBox { width: 30px; checked <=> r.mon; }
                        CheckBox { width: 30px; checked <=> r.tue; }
                        CheckBox { width: 30px; checked <=> r.wed; }
                        CheckBox { width: 30px; checked <=> r.thu; }
                        CheckBox { width: 30px; checked <=> r.fri; }
                        CheckBox { width: 30px; checked <=> r.sat; }
                        CheckBox { width: 30px; checked <=> r.sun; }
                    }
                    LineEdit { text <=> r.message; horizontal-stretch: 1; }
                    Button { text: "✕"; width: 32px; clicked => { root.delete-reminder(i); } }
                }
            }
        }

        Button {
            text: "+ 新增提醒";
            clicked => { root.add-reminder(); }
        }

        Button {
            text: "儲存並隱藏";
            clicked => { root.save-clicked(); }
        }
    }
}
```

Width grew 360→720 to fit the multi-column rows; height grew 250→560 for the list section.

- [ ] **Step 2: Update `src/main.rs` to wire the reminder list**

Open `d:\github\break_reminder_rs\src\main.rs`. Replace its full content with EXACTLY:
```rust
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
    // seed defaults whenever it's empty. Once the user explicitly deletes
    // every entry, they'll be reseeded next launch; that's an acceptable
    // edge case for a tray app.
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

    let _tray = tray::build();
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
```

The two helper functions stay at the bottom of `main.rs`. They convert between Slint's `ScheduledReminderRow` (one bool per day, time as string) and Rust's `ScheduledReminder` (bitmask, NaiveTime).

Note Slint's auto-snake_case on field names: Slint `time-text` becomes Rust `time_text`. Same applies to all hyphenated identifiers.

- [ ] **Step 3: Build**

Run:
```powershell
cargo build
```
Expected: succeeds with zero warnings. The biggest risk is Slint property names diverging from what Rust expects (e.g., `set_reminders` vs `set_reminder_list`); the Slint codegen is strict about kebab→snake.

If errors, report verbatim and STOP.

- [ ] **Step 4: Manual end-to-end verification (HUMAN required)**

Run from `d:\github\break_reminder_rs`:
```powershell
cargo run
```

Verify each item:
- [ ] Tray icon appears.
- [ ] Double-click tray → Settings window shows the existing controls (interval, two checkboxes) AND a "排程提醒" section with three default reminders (11:55 / 19:00 / 00:00).
- [ ] Each row shows: enable checkbox, time field, 7 day checkboxes (一/二/三/四/五/六/日), message field, ✕ delete button.
- [ ] Default rows: weekday rows have Mon-Fri checked, Sun-Sat unchecked. Sleep row has all 7 checked.
- [ ] Edit a reminder: change time to (now + 1 minute) — e.g., if it's 14:32, set to 14:33. Click 儲存並隱藏.
- [ ] Wait until that minute. Reminder window appears with the **custom message** ("吃飯囉！" etc., depending on which row you edited).
- [ ] Click 我知道了. Reopen Settings → that row's `LastFiredDate` is preserved IF the user didn't edit it before saving. (Spec: not preserved across saves; OK if it's reset.)
- [ ] Restart app within the same minute → does NOT fire again (last_fired_date is today).
- [ ] Click "+ 新增提醒". A new row appears with default values (12:00, weekdays, "提醒訊息").
- [ ] Click ✕ on a row. The row disappears immediately. Save. Reopen → it's gone.
- [ ] Inspect `settings.json`:
  ```powershell
  Get-Content "$env:LOCALAPPDATA\BreakReminderRs\settings.json"
  ```
  Verify the `ScheduledReminders` array reflects the edits.
- [ ] Tray right-click → 立刻休息 → reminder uses the **default** message (not a custom one), matching WPF.
- [ ] Tray right-click → 離開 → process exits.

If any check fails, do NOT commit until fixed.

- [ ] **Step 5: Commit**

```powershell
git add .
git commit -m "feat: add scheduled reminders Settings UI with add/delete"
```

- [ ] **Step 6: Tag**

```powershell
git tag -a v0.4.0-scheduled-reminders -m "v0.4.0: scheduled reminders with WPF parity"
git log --oneline | head -10
git tag -l
```

Expected: HEAD on the new commit; four tags total: `v0.1.0-mvp`, `v0.2.0-autostart`, `v0.3.0-play-sound`, `v0.4.0-scheduled-reminders`.
