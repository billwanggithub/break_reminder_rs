# Break Reminder (Rust) — 休息提醒小幫手

A Windows tray app that reminds you to take breaks at regular intervals, plus runs scheduled reminders (e.g. lunch, end-of-day, bedtime). Rust + Slint port of the original WPF [break_reminder](https://github.com/billwanggithub/break_reminder).

## Features

- System tray icon with right-click menu and double-click to open settings
- Configurable reminder interval (default: 45 minutes)
- Always-on-top reminder window
- Optional Windows system "Exclamation" sound on each reminder
- Optional auto-start at Windows login (HKCU Run key)
- Scheduled reminders: any number of named reminders that fire at specific times on selected days, each with a custom message
- Settings persisted to `%LocalAppData%\BreakReminderRs\settings.json` (compatible with the WPF version's JSON layout)
- Single ~9 MB executable, no .NET runtime needed

## Requirements

- Windows 10 / 11
- Rust toolchain (`stable-msvc`) and Visual Studio Build Tools with the "Desktop development with C++" workload (needed by Rust's MSVC linker and by Slint)

## Build & Run

```powershell
cargo build              # debug
cargo run                # debug, with console window
cargo build --release    # release: target/release/break_reminder_rs.exe (no console)
```

## Usage

1. Launch the app — no window opens; look for the icon in the system tray.
2. Double-click the tray icon (or right-click → 設定) to open Settings.
3. Adjust:
   - **休息間隔** — minutes between automatic break reminders (1–240).
   - **登入時自動啟動** — register the exe in `HKCU\...\Run`.
   - **播放提示音** — play the system Exclamation sound on each reminder.
   - **排程提醒** — list of named reminders, each with time / weekdays / message.
4. Click **儲存並隱藏**.

Right-click the tray icon for:
- **設定 (Settings)** — open the settings window
- **立刻休息 (Break Now)** — fire a reminder immediately (also resets the interval timer)
- **離開 (Exit)** — quit

## Architecture

Single-threaded, event-driven Slint application. The Slint event loop owns everything; an `Rc<RefCell<AppState>>` is shared across timer / tray / window callbacks. Tray events come in via the `tray-icon` crate on a background thread and are routed back to the UI thread via `slint::invoke_from_event_loop` plus a `thread_local` AppState handle.

Source layout:

```
src/
├── main.rs        — wiring: load settings, build UI, init tray, start timers
├── state.rs       — AppState, both timers, show_reminder, show_settings
├── settings.rs    — JSON load/save (serde, dirs)
├── tray.rs        — tray icon + menu (tray-icon)
├── autostart.rs   — HKCU\...\Run reconcile (winreg)
├── sound.rs       — Win32 MessageBeep wrapper
└── schedule.rs    — DayOfWeekMask, ScheduledReminder, check_due_reminders
ui/
├── app.slint      — module-graph root re-exporting the windows
├── MainWindow.slint    — settings window
└── ReminderWindow.slint — reminder window (always-on-top)
assets/
└── app.ico        — embedded into the exe via winres
```

## Design Documents

Full design spec and implementation plan for each release lives under [`docs/`](docs/):

- v0.1.0 — [MVP design](docs/specs/2026-05-02-rust-break-reminder-mvp-design.md)
- v0.2.0 — [autostart](docs/specs/2026-05-02-autostart-design.md), [plan](docs/plans/2026-05-02-autostart.md)
- v0.3.0 — [play sound](docs/specs/2026-05-02-play-sound-design.md), [plan](docs/plans/2026-05-02-play-sound.md)
- v0.4.0 — [scheduled reminders](docs/specs/2026-05-02-scheduled-reminders-design.md), [plan](docs/plans/2026-05-02-scheduled-reminders.md)

## Settings JSON

```json
{
  "IntervalMinutes": 45,
  "AutoStart": true,
  "PlaySound": false,
  "ScheduledReminders": [
    { "Enabled": true, "Time": "11:55", "Days": "Weekdays", "Message": "吃飯囉！" },
    { "Enabled": true, "Time": "19:00", "Days": "Weekdays", "Message": "下班時間到！" },
    { "Enabled": true, "Time": "00:00", "Days": "All", "Message": "該睡覺了！" }
  ]
}
```

`Days` accepts the WPF-style `"All"` / `"Weekdays"` aliases, comma-separated day names, or `|`-separated day names.

## Known Limitations

- If a reminder is already on screen and a scheduled reminder fires at the same time, the second message is dropped (no pending-message queue). Same-minute fires are rare in practice.
- UI text is hardcoded in Traditional Chinese.
- Windows-only (uses `winreg`, `MessageBeep`, and the `windows-subsystem = "windows"` attribute).

## License

Same as upstream — MIT (see WPF [break_reminder](https://github.com/billwanggithub/break_reminder)).
