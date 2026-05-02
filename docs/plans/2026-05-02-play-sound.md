# Play Sound on Reminder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a reminder fires, optionally play the Windows "Exclamation" system sound, controlled by a Settings checkbox. Default off; behaviour matches the WPF `SystemSounds.Exclamation.Play()`.

**Architecture:** A 4-line `sound` module wrapping `MessageBeep(MB_ICONEXCLAMATION)` via `extern "system"`. A new `play_sound: bool` field on `Settings` (JSON key `PlaySound`, default `false`) is read in `state::show_reminder` to gate the call. UI gains a "播放提示音" checkbox below the existing "登入時自動啟動" checkbox.

**Tech Stack:** Direct Win32 `MessageBeep` via `extern "system"` — no new dependencies.

**Spec:** [`docs/specs/2026-05-02-play-sound-design.md`](../specs/2026-05-02-play-sound-design.md)

**Note on testing:** No automated tests, matching v0.1.0 / v0.2.0. Manual sound check at the end of Task 4 is the only verification step that needs human ears.

---

## Task 1: Create the `sound` module

**Files:**
- Create: `src/sound.rs`

The module is a single `pub fn play_alert()` wrapping `MessageBeep`. It's not yet wired up — Task 3 calls it from `state.rs`. After this task, the file is on disk but not yet declared as a `mod` in `main.rs`, so no compiler warnings about dead code.

- [ ] **Step 1: Create `src/sound.rs`**

Create `d:\github\break_reminder_rs\src\sound.rs` with EXACTLY:
```rust
// Windows system "Exclamation" sound — matches WPF SystemSounds.Exclamation.Play().
//
// MessageBeep returns immediately; the sound plays asynchronously on a system
// thread. The return value is the BOOL result; we discard it because the only
// realistic failure is "the user set Exclamation to (None) in Sound Settings",
// which is silence-by-design, not an app error.

const MB_ICONEXCLAMATION: u32 = 0x00000030;

extern "system" {
    fn MessageBeep(utype: u32) -> i32;
}

pub fn play_alert() {
    unsafe {
        let _ = MessageBeep(MB_ICONEXCLAMATION);
    }
}
```

- [ ] **Step 2: Verify the file is on disk**

Use Read to confirm `d:\github\break_reminder_rs\src\sound.rs` matches the content above byte-for-byte.

Do NOT add `mod sound;` to `main.rs` yet — Task 3 does that. Running `cargo build` now will succeed and produce no new warnings (the file is not yet part of the crate compilation, exactly like Task 1 of the autostart feature).

- [ ] **Step 3: Build to confirm nothing else regressed**

Run from `d:\github\break_reminder_rs`:
```powershell
cargo build
```
Expected: succeeds with zero warnings. The `sound.rs` file is invisible to the compiler because no `mod sound;` declaration exists yet.

- [ ] **Step 4: Commit**

```powershell
git add .
git commit -m "feat: add sound module wrapping Win32 MessageBeep"
```

---

## Task 2: Add `play_sound` field to `Settings`

**Files:**
- Modify: `src/settings.rs`

Mirrors the autostart Task 2 pattern but with `default = false` (the simple bool default), no helper function needed.

- [ ] **Step 1: Update `Settings` struct and `Default` impl**

Open `d:\github\break_reminder_rs\src\settings.rs`. Find the `Settings` struct and `Default` impl. Replace ONLY those two items.

After the change, the top of the file (everything before `pub fn settings_path`) should be EXACTLY:
```rust
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(rename = "IntervalMinutes")]
    pub interval_minutes: u32,
    #[serde(rename = "AutoStart", default = "default_auto_start")]
    pub auto_start: bool,
    #[serde(rename = "PlaySound", default)]
    pub play_sound: bool,
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
        }
    }
}
```

`#[serde(default)]` (without an explicit function) uses `bool::default()` which is `false`. That matches the WPF parity requirement, so no `default_play_sound` helper is needed.

The rest of `settings.rs` (`pub fn settings_path`, `pub fn load`, `pub fn save`) must remain unchanged. Use the Edit tool, not Write — keep the lower part of the file intact.

- [ ] **Step 2: Build**

Run:
```powershell
cargo build
```
Expected: succeeds. Zero new warnings.

If the compiler complains "missing field `play_sound` in initializer of `Settings`" anywhere, that means another file constructs `Settings` directly via struct literal (e.g., `Settings { interval_minutes: ..., auto_start: ... }` without the `..Default::default()` shortcut). The codebase only constructs `Settings` via `Default::default()` and `serde_json::from_str`, so this should not happen — but if it does, report verbatim and STOP.

- [ ] **Step 3: Commit**

```powershell
git add .
git commit -m "feat: add PlaySound field to Settings (defaults to false)"
```

---

## Task 3: Wire `sound::play_alert` into `state::show_reminder`

**Files:**
- Modify: `src/main.rs` (add `mod sound;`)
- Modify: `src/state.rs`

After this task, the timer-fired reminder window will play the system Exclamation sound when `settings.play_sound` is `true`. The Settings UI checkbox is still missing — the user can only flip the value by hand-editing settings.json. Task 4 fixes that.

- [ ] **Step 1: Add `mod sound;` after `mod autostart;` in `main.rs`**

Open `d:\github\break_reminder_rs\src\main.rs`. Find the `mod` block:
```rust
mod settings;
mod state;
mod tray;
mod autostart;
```
Replace with:
```rust
mod settings;
mod state;
mod tray;
mod autostart;
mod sound;
```

- [ ] **Step 2: Wire the call into `state::show_reminder`**

Open `d:\github\break_reminder_rs\src\state.rs`. Find this exact block in `pub fn show_reminder` (after the dedup early-return, after `state.borrow().stop_timer();`, after the `let window = match ReminderWindow::new() { ... };` block):
```rust
    let window = match ReminderWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[break_reminder_rs] failed to create reminder window: {e}");
            AppState::restart_timer(state);
            return;
        }
    };

    window.on_dismissed({
```
Insert two new lines between the closing `};` of the match and `window.on_dismissed`. The result should be:
```rust
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

The borrow scope is a single statement; it drops before any of the subsequent `state.borrow*()` calls. The placement (after `ReminderWindow::new()` succeeds, before `on_dismissed` is wired) means we play the sound only on **new** windows, not on re-shown windows from the dedup branch — matching WPF behaviour and the spec.

- [ ] **Step 3: Build**

Run:
```powershell
cargo build
```
Expected: succeeds. Zero new warnings. The `dead_code` warning that would have appeared once `mod sound;` was added is gone because Task 3 also wires up the call.

- [ ] **Step 4: Commit**

```powershell
git add .
git commit -m "feat: play system sound on new reminder when PlaySound is enabled"
```

---

## Task 4: Add the Settings checkbox and save-time wiring

**Files:**
- Modify: `ui/MainWindow.slint`
- Modify: `src/main.rs`

This task adds the user-facing toggle. The Slint property `play-sound` two-way binds to a checkbox; the save closure reads it and writes to `state.settings.play_sound`.

- [ ] **Step 1: Overwrite `ui/MainWindow.slint`**

Use the Write tool to overwrite `d:\github\break_reminder_rs\ui\MainWindow.slint` with EXACTLY:
```slint
import { SpinBox, Button, CheckBox, VerticalBox } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "設定 - 休息提醒小幫手";
    width: 360px;
    height: 250px;

    in-out property <int> interval-minutes: 45;
    in-out property <bool> auto-start: true;
    in-out property <bool> play-sound: false;
    callback save-clicked();

    VerticalBox {
        padding: 20px;
        spacing: 12px;

        HorizontalLayout {
            spacing: 8px;
            alignment: center;
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

        Button {
            text: "儲存並隱藏";
            clicked => { root.save-clicked(); }
        }
    }
}
```

Differences from previous version:
- `height: 220px` → `height: 250px`.
- New `in-out property <bool> play-sound: false;` (default `false`, matches `Settings::default()`).
- New `CheckBox { text: "播放提示音"; checked <=> root.play-sound; }` row between the autostart checkbox and the Save button.

- [ ] **Step 2: Initialise the new `play_sound` Slint property**

Open `d:\github\break_reminder_rs\src\main.rs`. Find this block:
```rust
    settings_window.set_interval_minutes(app_state.borrow().settings.interval_minutes as i32);
    settings_window.set_auto_start(app_state.borrow().settings.auto_start);
```
Replace with:
```rust
    settings_window.set_interval_minutes(app_state.borrow().settings.interval_minutes as i32);
    settings_window.set_auto_start(app_state.borrow().settings.auto_start);
    settings_window.set_play_sound(app_state.borrow().settings.play_sound);
```

- [ ] **Step 3: Update the save closure to read/write `play_sound`**

Open `d:\github\break_reminder_rs\src\main.rs`. Find this exact closure body:
```rust
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
```
Replace with:
```rust
        move || {
            let Some(state) = weak_state.upgrade() else { return };
            let Some(window) = weak_window.upgrade() else { return };
            let new_interval = window.get_interval_minutes().max(1) as u32;
            let new_auto_start = window.get_auto_start();
            let new_play_sound = window.get_play_sound();
            {
                let mut s = state.borrow_mut();
                s.settings.interval_minutes = new_interval;
                s.settings.auto_start = new_auto_start;
                s.settings.play_sound = new_play_sound;
            }
            state.borrow().save();
            AppState::restart_timer(&state);
            if let Ok(exe) = std::env::current_exe() {
                autostart::apply(new_auto_start, &exe);
            }
            window.hide().ok();
        }
```

Three changes:
- Read `new_play_sound` immediately after `new_auto_start`.
- Inside the scoped `borrow_mut()` block, also write `s.settings.play_sound = new_play_sound;`.
- No new side-effect call — `play_sound` only takes effect on the next reminder fire (handled by Task 3's `state::show_reminder` change).

- [ ] **Step 4: Build**

Run:
```powershell
cargo build
```
Expected: succeeds. Zero warnings.

- [ ] **Step 5: Manual end-to-end verification (HUMAN ears required)**

Run from `d:\github\break_reminder_rs`:
```powershell
cargo run
```

Verify each item:
- [ ] Tray icon appears.
- [ ] Double-click tray → settings window opens. Three controls visible: SpinBox row, "登入時自動啟動" checkbox, "播放提示音" checkbox (unchecked). Save button at the bottom.
- [ ] Tick "播放提示音", change interval to 1, click 儲存並隱藏. Window hides.
- [ ] Inspect `settings.json`:
  ```powershell
  Get-Content "$env:LOCALAPPDATA\BreakReminderRs\settings.json"
  ```
  Expected: `"PlaySound": true`.
- [ ] Tray right-click → 立刻休息. Reminder window appears AND a single "ding" Exclamation sound plays concurrently.
- [ ] Click 我知道了 to dismiss. Wait 1 minute → another reminder fires with another sound.
- [ ] Double-click tray, untick "播放提示音", save. Tray right-click → 立刻休息 → reminder appears, NO sound.
- [ ] Re-open Settings → checkbox state remembered as unchecked. JSON shows `"PlaySound": false`.
- [ ] Tray right-click → 離開 to exit.

If any check fails (especially: sound doesn't play when checkbox is on, OR sound plays when checkbox is off), do NOT commit — investigate and fix first.

- [ ] **Step 6: Commit**

```powershell
git add .
git commit -m "feat: add 播放提示音 checkbox bound to PlaySound setting"
```

- [ ] **Step 7: Tag**

```powershell
git tag -a v0.3.0-play-sound -m "v0.3.0: optional system sound on reminder"
git log --oneline | head -10
git tag -l
```

Expected: HEAD on the new commit; `v0.3.0-play-sound` listed alongside `v0.1.0-mvp` and `v0.2.0-autostart`.
