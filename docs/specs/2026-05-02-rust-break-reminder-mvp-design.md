# Rust Break Reminder MVP — Design

**Date:** 2026-05-02
**Status:** Approved (brainstorming phase)
**Target repo:** `d:\github\break_reminder_rs\` (new, separate from this WPF repo)

## Goal

Port the core of this WPF break-reminder app to Rust + Slint as a learning vehicle for Rust GUI development on Windows, while validating that VS Code's Slint preview gives an XAML-Designer-like workflow.

The MVP intentionally drops scheduled reminders, the reminder queue, sound playback, and Windows auto-startup. Those exist in the WPF version and may be added later, but each is a non-trivial subsystem that would distract from getting the core toolchain (Slint + tray-icon + Slint Timer + serde) working end-to-end.

## In Scope (MVP)

- System tray icon with right-click menu: Settings / Break Now / Exit
- Double-click tray to open settings
- Fixed-interval timer (default 45 minutes)
- Reminder window (always-on-top, centered, "I got it" button)
- Settings window (single integer input for interval minutes + Save button)
- Settings persisted to `%LocalAppData%\BreakReminderRs\settings.json`

## Out of Scope (MVP)

- Scheduled per-day reminders (the `ScheduledReminder` list in WPF version)
- Sound playback
- Windows auto-startup via registry
- Reminder queue / pending message coalescing
- Localization (UI hardcoded in Traditional Chinese, matching WPF)
- Tests (matches WPF version; manual checklist instead)

## Architecture

Single-threaded, event-driven Slint application. The Slint event loop is the hub; all events (timer ticks, tray clicks, button clicks) are dispatched on the UI thread.

```
┌────────────────────────────────────────────┐
│           Slint Event Loop (UI thread)     │
│                                            │
│   ┌──────────┐   ┌──────────┐  ┌────────┐  │
│   │  Timer   │   │   Tray   │  │ Window │  │
│   │ (45 min) │   │   icon   │  │  s     │  │
│   └────┬─────┘   └────┬─────┘  └───┬────┘  │
│        │              │            │       │
│        └──────────────┴────────────┘       │
│                       │                    │
│                       ▼                    │
│                 ┌──────────┐               │
│                 │ AppState │               │
│                 │ (Rc<...>)│               │
│                 └──────────┘               │
└────────────────────────────────────────────┘
                        │
                        ▼
              %LocalAppData%\BreakReminderRs\
                  settings.json
```

`AppState` is owned via `Rc<RefCell<AppState>>` and shared across closures. Single-threaded means no `Arc`/`Mutex`/channels — every callback runs on the same thread.

## Components

| Component | Responsibility | Maps to WPF |
|---|---|---|
| `AppState` (in `state.rs`) | Holds `interval_minutes`, settings path, weak handles to settings/reminder windows, `slint::Timer` | Fields on `App.xaml.cs` |
| `settings.rs` | Load/save `settings.json` via serde_json | `LoadSettings`/`SaveSettings` |
| `tray.rs` | Build tray icon and menu; wire double-click + menu events into Slint event loop | `_notifyIcon` setup |
| `MainWindow.slint` | Settings window: SpinBox + Save button | `MainWindow.xaml` |
| `ReminderWindow.slint` | Reminder window: headline + message + "I got it" button | `ReminderWindow.xaml` |
| `main.rs` | Wire everything: load settings → build state → init tray → start timer → run event loop | `OnStartup` |

Each file has one clear responsibility and can be read independently.

### Project layout

```
break_reminder_rs/
├── Cargo.toml
├── build.rs              # invokes slint-build to compile .slint files
├── assets/
│   └── app.ico           # copied from this WPF repo
├── ui/
│   ├── MainWindow.slint
│   └── ReminderWindow.slint
└── src/
    ├── main.rs
    ├── state.rs
    ├── settings.rs
    └── tray.rs
```

## Data Flow

### Startup

1. `main()` resolves `%LocalAppData%\BreakReminderRs\settings.json`.
2. Load settings; on missing/corrupt file, use default `{ interval_minutes: 45 }`.
3. Build `AppState` wrapped in `Rc<RefCell<...>>`.
4. Build tray icon with menu (Settings / Break Now / Exit) and double-click handler.
5. Start a `slint::Timer` with interval = `interval_minutes` minutes; tick handler calls `show_reminder()`.
6. No window is shown initially. Enter `slint::run_event_loop()`.

### User changes interval

1. User opens settings (double-click tray or "Settings" menu).
2. Settings window shows; SpinBox is two-way bound (Slint property) to `interval_minutes`.
3. User edits value, clicks "Save and hide":
   - Write `settings.json`.
   - Restart `slint::Timer` with new interval.
   - Hide settings window.

### Timer fires

1. Timer callback calls `show_reminder()`.
2. If reminder window already exists, just call `window.show()` (no duplicate).
3. Otherwise build a new `ReminderWindow` and show it.
4. User clicks "I got it" → `window.hide()`. Timer continues into the next interval automatically.

### "Break Now" menu / double-click tray

- "Break Now": same as a timer tick — show reminder window. Per WPF parity, this also resets the timer (stop on show, restart when reminder closes), so the next automatic break is a full interval away.
- Double-click tray: open settings window (no timer effect).

## Error Handling

Pragmatic, not defensive:

- **Settings file missing or corrupt** → use defaults silently; next save overwrites. No dialog.
- **Settings file write fails** → `eprintln!` to stderr (visible in debug build); continue running. Matches WPF's `catch {}`.
- **Tray icon creation fails** → `panic!` and exit. App has no purpose without the tray.
- **Invalid interval input** → Slint SpinBox enforces `min=1, max=240` at the UI layer. No backend check needed.
- **Window construction fails** → propagate `Result` up to `main()`; let the OS show the crash. This is unrecoverable.

## Testing

Following the WPF version's precedent, the MVP ships without an automated test suite.

Reasons:
- UI-interaction tests (Slint windows, tray events) need a real event loop and a Windows session — high cost, low value.
- Pure-logic surface is tiny (only `settings.json` serde), and serde itself is well-tested.
- MVP priority is making the toolchain path work end-to-end.

### Manual verification checklist

- [ ] `cargo run` shows tray icon
- [ ] Double-click tray opens settings window
- [ ] Edit interval, click save → window hides
- [ ] Restart app → new interval persisted
- [ ] After interval elapses, reminder window appears on top, centered
- [ ] Click "I got it" → window closes, timer continues
- [ ] "Break Now" menu item shows reminder immediately and resets the timer
- [ ] "Exit" menu item terminates the process
- [ ] Open `.slint` file in VS Code; Slint preview renders the window

## Dependencies

```toml
[dependencies]
slint = "1.8"
tray-icon = "0.19"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"

[build-dependencies]
slint-build = "1.8"
```

Notable choices:
- **slint** for UI and event loop (declarative `.slint` files give the XAML-Designer-like VS Code preview).
- **tray-icon** (Tauri team) — most-maintained tray crate on Windows; integrates with winit/Slint event loop.
- **dirs** to resolve `%LocalAppData%` cross-platform-correctly.
- No `tokio` — Slint Timer is sufficient for a single periodic reminder.

## Build & Distribute

- `cargo run` for development (with console window).
- `cargo build --release` for shipping; expected ~8–12 MB single `.exe`, no .NET runtime needed.
- Hide console in release: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` at top of `main.rs`.
- Embed icon via `winres` in `build.rs` (alongside the Slint compile step). Reuse `app.ico` from the WPF repo.

## Resolved Decisions

1. **Tray icon source**: copy `app.ico` from the WPF repo into `assets/app.ico`, embed via `winres`.
2. **"Break Now" timer behavior**: matches WPF — stop timer on show, restart when reminder window closes. Manual breaks reset the schedule.
3. **Spec location**: this document lives in the WPF repo (current working tree) since the Rust repo doesn't exist yet. When the Rust repo is created, a copy goes there too.
