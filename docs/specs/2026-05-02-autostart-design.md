# Autostart on Windows Login — Design

**Date:** 2026-05-02
**Status:** Approved (brainstorming phase)
**Builds on:** v0.1.0-mvp

## Goal

Make `break_reminder_rs.exe` start automatically when the Windows user logs in, with a Settings checkbox to opt out. Behaviour matches the WPF version (default-on), but adds the missing UI control.

## In Scope

- Write `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\BreakReminderRs = "<full quoted exe path>"` on startup when `auto_start = true`.
- Delete that registry value when `auto_start = false`.
- Settings window gets a "登入時自動啟動" checkbox bound to `auto_start`.
- New JSON field `AutoStart` in `settings.json`. Missing field → defaults to `true` (matches WPF default-on behaviour and existing user expectations).

## Out of Scope

- Per-user vs. machine-wide install (HKLM). HKCU is the only target.
- Startup folder shortcuts (`shell:startup`). Registry Run key is the chosen mechanism.
- Detecting/repairing user edits made directly via regedit. Settings.json is the authoritative state; the registry is a downstream effect.
- Migration of the WPF version's `BreakReminderApp` registry entry. The Rust app uses a distinct key name (`BreakReminderRs`) so the two apps coexist cleanly.

## Architecture

A small `autostart` module exposes one function: `apply(enabled, exe_path)`. The module is the **only** place that touches the registry. Callers pass desired state and the current exe path; `apply` writes or deletes accordingly.

State of truth flows in one direction:

```
settings.json (source of truth)
        │
        ▼
   load() in main.rs
        │
        ▼
  autostart::apply(enabled, exe_path)
        │
        ▼
HKCU\...\Run\BreakReminderRs (downstream effect)
```

The Settings UI never reads the registry — it reads `settings.auto_start`. The "save" button writes settings, then calls `apply` to reconcile the registry with the new state.

## Components

| Component | Responsibility |
|---|---|
| `src/autostart.rs` (new) | `pub fn apply(enabled: bool, exe_path: &Path)` — write or delete the Run key value |
| `src/settings.rs` (modified) | Add `auto_start: bool` field with `#[serde(default = "default_auto_start")]` returning `true` |
| `ui/MainWindow.slint` (modified) | Add `auto-start` property + checkbox below the interval row |
| `src/main.rs` (modified) | Call `autostart::apply` once at startup; in `on_save_clicked`, update settings and call `apply` again |

## Data Flow

### Startup
1. `main()` loads settings (existing path).
2. `std::env::current_exe()` gives the running `.exe` path.
3. `autostart::apply(settings.auto_start, &exe)` is called once. Failure is logged to stderr and ignored — autostart is non-essential.
4. Settings window is created; its `auto-start` property is set from `settings.auto_start` (alongside the existing `interval_minutes` set).

### User toggles checkbox and saves
1. User opens Settings, toggles checkbox (Slint property `auto-start` updates via two-way binding).
2. User clicks "儲存並隱藏".
3. `on_save_clicked` reads `interval_minutes` AND `auto_start` from the window.
4. Both fields go into `state.settings`. `state.save()` writes JSON.
5. `autostart::apply(state.settings.auto_start, &current_exe)` reconciles the registry.
6. Window hides.

### EXE moved by user
- Next startup, `current_exe()` returns the new path. `apply(true, &new_path)` writes the new path to the Run key, replacing the old one. Self-healing.

## Error Handling

Pragmatic, non-blocking:

- **Cannot open `HKCU\...\Run`** → `eprintln!` and return. App continues without autostart.
- **Cannot write value** → same.
- **Cannot delete value** when disabling → same. (May leave a stale entry; the user can clean it via regedit. Will not happen under normal conditions.)
- **`current_exe()` fails** → skip autostart entirely with a stderr log; app continues.

The WPF version's `try { } catch (Exception) { }` is the model — silent failure is acceptable for a non-essential feature.

## Settings JSON

### Before (v0.1.0-mvp)
```json
{ "IntervalMinutes": 45 }
```

### After
```json
{ "IntervalMinutes": 45, "AutoStart": true }
```

Backwards compatibility: `#[serde(default = "default_auto_start")]` returns `true` when the field is absent. Existing v0.1.0-mvp users get the WPF-equivalent default-on behaviour with no migration code.

## Slint UI

Below the existing horizontal row (interval + label), add:

```slint
CheckBox {
    text: "登入時自動啟動";
    checked <=> root.auto-start;
}
```

`auto-start` is declared as `in-out property <bool>` on the `MainWindow` component, alongside `interval-minutes`.

The window height grows from 180px to 220px to fit the new row without cramping.

## Dependencies

Add to `[dependencies]`:
```toml
winreg = "0.52"
```

`winreg` is the standard, well-maintained Rust crate for Windows registry access. No platform feature flag needed because the entire app is Windows-only.

## Testing

No automated tests (matches the rest of this app). Manual verification:

- [ ] Fresh install (no `BreakReminderRs` in registry yet): launch app → `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v BreakReminderRs` shows the exe path in quotes.
- [ ] Restart Windows / log out + log in → app launches automatically; tray icon appears.
- [ ] Open Settings, uncheck "登入時自動啟動", save → `reg query` returns `ERROR: ... cannot find` (value deleted).
- [ ] Re-check, save → `reg query` shows the path again.
- [ ] Move `break_reminder_rs.exe` to a new folder, launch → `reg query` shows the new path (self-heal).
- [ ] Old `settings.json` without `AutoStart` field still loads; first save writes the new field.
- [ ] WPF version's `BreakReminderApp` registry entry is untouched (different key name).

## Resolved Decisions

1. **Registry key name**: `BreakReminderRs` (distinct from WPF's `BreakReminderApp`).
2. **Default**: `true` for new installs and missing JSON field.
3. **Source of truth**: `settings.json`. Registry is downstream-only.
4. **Path format**: full path wrapped in `"..."` quotes (matches WPF `App.xaml.cs:309`).
5. **Error policy**: silent stderr log, never blocks app.
