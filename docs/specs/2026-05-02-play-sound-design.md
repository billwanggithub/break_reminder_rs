# Play Sound on Reminder — Design

**Date:** 2026-05-02
**Status:** Approved (brainstorming phase)
**Builds on:** v0.2.0-autostart

## Goal

When a reminder fires, optionally play the Windows system "Exclamation" sound, controlled by a Settings checkbox. Behaviour matches the WPF version (`SystemSounds.Exclamation.Play()`), default off.

## In Scope

- A new `sound` module exposing `play_alert()`, which invokes `MessageBeep(MB_ICONEXCLAMATION)`.
- A new `play_sound: bool` field in `Settings` (JSON key `PlaySound`, default `false`).
- A new "播放提示音" checkbox in the Settings window, paired vertically with the existing "登入時自動啟動" checkbox.
- `state::show_reminder` calls `sound::play_alert()` once per **new** reminder window (not on the dedup-show path), gated on `settings.play_sound`.

## Out of Scope

- Custom sound files (user-supplied path, embedded `.wav`).
- Volume control. The system controls this.
- Looping or repeated playback. One beep per reminder.
- Any audio crate (rodio, kira, etc.) — `MessageBeep` is sufficient.
- Sound on "立刻休息" tray menu vs timer fire — both go through the same `show_reminder` path, so both will play if enabled. Matches WPF.

## Architecture

A tiny Win32 wrapper module. `sound::play_alert()` declares `MessageBeep` via `extern "system"` and calls it inside an `unsafe` block. No state, no allocations, no error path — `MessageBeep` returns immediately and plays asynchronously on a system thread, never blocking the UI.

```
state::show_reminder
        │
        ├─ ReminderWindow::new()
        │
        ├─ if settings.play_sound { sound::play_alert(); }   ◀── here
        │
        ├─ window.on_dismissed(...)
        │
        └─ window.show()
```

Settings is the single source of truth (consistent with autostart). The Slint property `play-sound` two-way binds to a checkbox; the save closure writes through to `state.settings.play_sound`.

## Components

| Component | Responsibility |
|---|---|
| `src/sound.rs` (new) | `pub fn play_alert()` — `unsafe` `MessageBeep(MB_ICONEXCLAMATION)` call |
| `src/settings.rs` (modified) | Add `play_sound: bool` field with `#[serde(default)]` and `Default::default()` value `false` |
| `ui/MainWindow.slint` (modified) | Add `play-sound` `<bool>` property + checkbox below the autostart checkbox; bump window height to 250px |
| `src/state.rs` (modified) | In `show_reminder`, after `ReminderWindow::new()` succeeds and before `window.show()`, conditionally call `sound::play_alert()` |
| `src/main.rs` (modified) | `mod sound;`; init the new `play-sound` Slint property; read it back in the save closure and write to `state.settings.play_sound` |

## Data Flow

### Startup
1. `settings::load()` reads `play_sound` (or defaults to `false` for old JSON files).
2. `main()` sets `settings_window.set_play_sound(...)` after the existing `set_interval_minutes` and `set_auto_start` calls.

### User toggles checkbox and saves
1. User toggles checkbox in Settings; Slint two-way binding updates the `play-sound` property.
2. Save click handler reads `window.get_play_sound()`, writes to `state.settings.play_sound` (in the same scoped `borrow_mut()` block as the other two fields), persists JSON via `state.save()`.
3. No registry sync, no other side-effects.

### Reminder fires
1. Timer or "立刻休息" menu calls `show_reminder(&state)`.
2. If a reminder window already exists, `show()` it again (dedup path) — no sound.
3. Otherwise, build a new `ReminderWindow`. After successful construction, check `state.borrow().settings.play_sound`; if `true`, call `sound::play_alert()`. Then wire `on_dismissed` and `show()` the window.

`play_alert` returns immediately; the window appears with the sound playing concurrently from a system thread.

## Error Handling

`MessageBeep` returns a `BOOL` (`i32`). `0` means failure, `non-zero` means success. The only realistic failure is that the user has set the system "Exclamation" sound to `(None)` in Sound Settings, in which case there's silence — that's user intent, not an app error.

We **discard the return value** entirely. No log, no fallback. Consistent with the spec's "the app must keep running even if a side-effect fails" stance.

## Settings JSON

### Before (v0.2.0-autostart)
```json
{ "IntervalMinutes": 45, "AutoStart": true }
```

### After
```json
{ "IntervalMinutes": 45, "AutoStart": true, "PlaySound": false }
```

`#[serde(rename = "PlaySound", default)]` makes the field tolerate missing values; bool's `Default` is `false`, which matches WPF's default. No migration code, no helper function (unlike `auto_start`'s explicit `default_auto_start`).

## Slint UI

After this task, `MainWindow.slint`'s body looks like:

```slint
VerticalBox {
    padding: 20px;
    spacing: 12px;

    HorizontalLayout { /* interval row, unchanged */ }

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
```

Window height grows from 220px to 250px to absorb the extra row.

## Dependencies

**No new dependencies.** `MessageBeep` is reached via `extern "system"` declaration. The function signature has been stable since Windows 95; we don't need a binding crate for one no-argument call.

## Testing

No automated tests (project policy). Manual checklist:

- [ ] Default install (no JSON or fresh JSON without `PlaySound`): checkbox unchecked, no sound on reminder.
- [ ] Tick checkbox, save, set interval to 1, wait 1 minute → hear system Exclamation sound concurrent with reminder window appearance.
- [ ] Untick checkbox, save, trigger reminder via "立刻休息" → no sound, window still appears.
- [ ] Multiple reminders in a row (interval=1, dismiss + wait) → each one plays its own sound.
- [ ] `Get-Content "$env:LOCALAPPDATA\BreakReminderRs\settings.json"` shows `"PlaySound": true` after enabling, `"PlaySound": false` after disabling.
- [ ] In Windows Sound Settings, set "Exclamation" to "(None)"; trigger reminder with checkbox on → silent reminder, no app crash.
- [ ] Pre-existing settings.json without `PlaySound` field still loads; first save adds the field with the user's choice.

## Resolved Decisions

1. **API**: `MessageBeep(MB_ICONEXCLAMATION)` via `extern "system"`. No `windows` crate, no `rodio`.
2. **Default**: `false` (matches WPF parity).
3. **Where in `show_reminder`**: after `ReminderWindow::new()` succeeds, before `on_dismissed`/`show()` wiring. Inside the new-window branch only — re-shown windows do NOT re-play.
4. **JSON key**: `PlaySound` (matches WPF; `#[serde(rename)]` maps to Rust `play_sound`).
5. **UI placement**: own row below autostart checkbox; window height 220→250.
6. **Errors**: discard `MessageBeep` return value entirely.
