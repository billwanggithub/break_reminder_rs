# Autostart Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire `HKCU\...\Run\BreakReminderRs` registry write/delete to a new `auto_start` settings field, exposed via a Settings checkbox, defaulting to `true` for parity with the WPF version.

**Architecture:** A new `src/autostart.rs` module is the only place that touches the registry; it exposes `apply(enabled, exe_path)`. `settings.rs` gains an `auto_start: bool` field with `#[serde(default)]`-style fallback to `true`. `main.rs` calls `apply` once at startup and again from the save closure. `MainWindow.slint` gains a `<bool>` property and a checkbox.

**Tech Stack:** winreg 0.52 for registry I/O. Existing serde / Slint / std::env::current_exe.

**Spec:** [`docs/specs/2026-05-02-autostart-design.md`](../specs/2026-05-02-autostart-design.md)

**Note on testing:** No automated tests, matching v0.1.0-mvp. Each task ends with `cargo build` and (where the change is observable) a manual registry-check command for the human user to run.

---

## Task 1: Add `winreg` dependency and create `autostart.rs`

**Files:**
- Modify: `Cargo.toml` — add `winreg = "0.52"` to `[dependencies]`
- Create: `src/autostart.rs`

This task introduces the registry-touching module in isolation, with no callers yet. Future tasks wire it in.

- [ ] **Step 1: Add `winreg` to Cargo.toml**

Open `d:\github\break_reminder_rs\Cargo.toml` and append at the end of the `[dependencies]` block (after `image = ...`):
```toml
winreg = "0.52"
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
```

- [ ] **Step 2: Create `src/autostart.rs`**

Create `d:\github\break_reminder_rs\src\autostart.rs` with EXACTLY:
```rust
use std::path::Path;

use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "BreakReminderRs";

/// Reconcile the HKCU Run-key entry with the desired state.
///
/// `enabled = true`  → write `"<exe path>"` (full path, quoted).
/// `enabled = false` → delete the value if it exists.
///
/// All errors are logged to stderr and swallowed — autostart is
/// non-essential. The app must keep running even if registry writes fail.
pub fn apply(enabled: bool, exe_path: &Path) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = match hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_SET_VALUE) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("[break_reminder_rs] failed to open Run key: {e}");
            return;
        }
    };

    if enabled {
        let Some(path_str) = exe_path.to_str() else {
            eprintln!("[break_reminder_rs] exe path is not valid UTF-8");
            return;
        };
        let quoted = format!("\"{path_str}\"");
        if let Err(e) = run_key.set_value(VALUE_NAME, &quoted) {
            eprintln!("[break_reminder_rs] failed to set Run value: {e}");
        }
    } else {
        match run_key.delete_value(VALUE_NAME) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {} // already absent
            Err(e) => eprintln!("[break_reminder_rs] failed to delete Run value: {e}"),
        }
    }
}
```

- [ ] **Step 3: Build to verify the dep resolves and module compiles**

Run from `d:\github\break_reminder_rs`:
```powershell
cargo build
```
Expected: succeeds. The first build pulls `winreg` and compiles it (~30 seconds).

The module is currently unused; expect a `dead_code` warning on `apply`. That's fine — Task 3 wires it up. No other warnings should appear.

- [ ] **Step 4: Commit**

```powershell
git add .
git commit -m "feat: add autostart module with HKCU Run-key write/delete"
```

---

## Task 2: Add `auto_start` field to `Settings`

**Files:**
- Modify: `src/settings.rs`

This makes the JSON round-trip aware of the new field, with backwards compatibility for existing files.

- [ ] **Step 1: Update `Settings` struct and Default impl**

Open `d:\github\break_reminder_rs\src\settings.rs`. Replace the `Settings` struct and its `Default` impl (the first ~14 lines) with:
```rust
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(rename = "IntervalMinutes")]
    pub interval_minutes: u32,
    #[serde(rename = "AutoStart", default = "default_auto_start")]
    pub auto_start: bool,
}

fn default_auto_start() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_minutes: 45,
            auto_start: true,
        }
    }
}
```

The rest of `settings.rs` (`settings_path`, `load`, `save`) stays unchanged.

- [ ] **Step 2: Build to verify compilation**

Run:
```powershell
cargo build
```
Expected: succeeds. No new warnings.

- [ ] **Step 3: Commit**

```powershell
git add .
git commit -m "feat: add AutoStart field to Settings (defaults to true)"
```

---

## Task 3: Wire startup-time registry sync in `main.rs`

**Files:**
- Modify: `src/main.rs`

This task only adds the startup call. The save-time call comes in Task 4 alongside the UI changes (they share state).

- [ ] **Step 1: Add `mod autostart;` and call `apply` once at startup**

Open `d:\github\break_reminder_rs\src\main.rs`. After `mod tray;` (existing), add `mod autostart;`. Then in `main()`, between `let loaded = settings::load(&path);` and `let settings_window = MainWindow::new()?;`, insert the startup sync.

Result — the top of `main()` should read:
```rust
fn main() -> Result<(), slint::PlatformError> {
    let path = settings::settings_path();
    let loaded = settings::load(&path);

    if let Ok(exe) = std::env::current_exe() {
        autostart::apply(loaded.auto_start, &exe);
    } else {
        eprintln!("[break_reminder_rs] could not determine current exe path; skipping autostart sync");
    }

    let settings_window = MainWindow::new()?;
    // ... rest of main() unchanged
```

And the module list near the top of the file should now read:
```rust
mod settings;
mod state;
mod tray;
mod autostart;
```

- [ ] **Step 2: Build**

Run:
```powershell
cargo build
```
Expected: succeeds. The `dead_code` warning from Task 1 is now gone (`apply` is called).

- [ ] **Step 3: Manual registry check**

Run from `d:\github\break_reminder_rs`:
```powershell
cargo run
```
The settings window will not open (tray-only after MVP). Wait until you see the tray icon, then in another PowerShell:
```powershell
reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v BreakReminderRs
```
Expected: a single line showing `BreakReminderRs    REG_SZ    "<full path>\break_reminder_rs.exe"` (with quotes around the path).

Right-click tray → 離開 to stop the app.

- [ ] **Step 4: Commit**

```powershell
git add .
git commit -m "feat: sync HKCU Run key on startup based on auto_start setting"
```

---

## Task 4: Add the Settings checkbox and save-time sync

**Files:**
- Modify: `ui/MainWindow.slint`
- Modify: `src/main.rs`

This task adds the user-facing toggle. The new property is two-way bound to `auto-start` on the window; the save closure reads it, updates settings, saves JSON, then calls `autostart::apply` to reconcile the registry.

- [ ] **Step 1: Update `ui/MainWindow.slint`**

Overwrite `d:\github\break_reminder_rs\ui\MainWindow.slint` with EXACTLY:
```slint
import { SpinBox, Button, CheckBox, VerticalBox } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "設定 - 休息提醒小幫手";
    width: 360px;
    height: 220px;

    in-out property <int> interval-minutes: 45;
    in-out property <bool> auto-start: true;
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

        Button {
            text: "儲存並隱藏";
            clicked => { root.save-clicked(); }
        }
    }
}
```

Changes from previous version:
- Added `CheckBox` to the import list.
- `height: 180px` → `height: 220px` (room for the new row).
- Added `in-out property <bool> auto-start: true;`.
- Added a `CheckBox` row between the `HorizontalLayout` and the `Button`.

- [ ] **Step 2: Update `src/main.rs` to read/write `auto_start`**

In `main()`, find the spot where the spinbox is initialised:
```rust
settings_window.set_interval_minutes(app_state.borrow().settings.interval_minutes as i32);
```

Add the matching line for `auto_start` immediately after it:
```rust
settings_window.set_auto_start(app_state.borrow().settings.auto_start);
```

Then in the `on_save_clicked` closure, find the existing block:
```rust
let new_value = window.get_interval_minutes().max(1) as u32;
state.borrow_mut().settings.interval_minutes = new_value;
state.borrow().save();
AppState::restart_timer(&state);
window.hide().ok();
```

Replace it with:
```rust
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
```

The full `on_save_clicked` closure should now look like:
```rust
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
```

The block `{ let mut s = state.borrow_mut(); ... }` is required so the mutable borrow drops before `state.borrow().save()` re-borrows.

- [ ] **Step 3: Build**

Run:
```powershell
cargo build
```
Expected: succeeds. No warnings.

- [ ] **Step 4: Manual end-to-end verification**

Run:
```powershell
cargo run
```
Verify each item:

- [ ] Tray icon appears.
- [ ] Double-click tray → settings window opens. The "登入時自動啟動" checkbox is checked.
- [ ] Uncheck the checkbox, click 儲存並隱藏. Window hides, app stays running.
- [ ] In another shell:
  ```powershell
  reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v BreakReminderRs
  ```
  Expected: `ERROR: ... unable to find the specified registry key or value` (the value was deleted).
- [ ] Reopen settings, re-check the box, save. Re-run the `reg query` — value reappears with the quoted exe path.
- [ ] Inspect `settings.json`:
  ```powershell
  Get-Content "$env:LOCALAPPDATA\BreakReminderRs\settings.json"
  ```
  Expected: contains `"AutoStart": true` (or `false`, matching what was saved).
- [ ] Tray menu → 離開 to exit.

If any check fails, do NOT commit — investigate and fix first.

- [ ] **Step 5: Commit**

```powershell
git add .
git commit -m "feat: add 登入時自動啟動 checkbox and save-time registry sync"
```

- [ ] **Step 6: Tag**

```powershell
git tag -a v0.2.0-autostart -m "v0.2.0: autostart on Windows login"
git log --oneline | head -8
git tag -l
```
Expected: HEAD on the new commit; `v0.2.0-autostart` listed alongside `v0.1.0-mvp`.
