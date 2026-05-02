# Mute (Do Not Disturb) — Design

**Date:** 2026-05-02
**Status:** Approved (brainstorming phase)
**Builds on:** v0.4.0-scheduled-reminders

## Goal

Add a global "mute" toggle that pauses all *automatic* reminders (interval timer + scheduled checks) without exiting the app. Manual breaks (the tray "立刻休息" item) keep working. Users can toggle mute either from the Settings checkbox or directly from the tray menu.

## In Scope

- New `muted: bool` field on `Settings` (JSON key `Muted`, default `false`).
- New "請勿打擾" Settings checkbox bound to `muted`.
- New "請勿打擾" tray menu item (above Settings) with a checkmark when active. Toggling it flips `muted` and saves.
- When `muted = true`:
  - Interval timer does not start.
  - Schedule timer does not start.
  - Startup-time `check_due_reminders` is skipped.
- When `muted = false` (or transitioning from true → false):
  - Both timers start as normal.
  - `check_due_reminders` runs once immediately, so any reminder due *right now* fires (e.g. mute released at 11:55 with the lunch reminder set for 11:55 → fires).
- Tray tooltip changes to "休息提醒小幫手 (請勿打擾中)" while muted.
- "立刻休息" tray item is not affected by mute — manual breaks always work.

## Out of Scope

- Timed mute (e.g. "mute for 1 hour"). Just a flat on/off.
- Visual icon change. Tooltip text-only signal.
- Catch-up of *missed* scheduled reminders after unmute. Past-due-during-mute fires are dropped (matches the existing 2-minute late-fire window — the reminder is gone once it slips past that window). Only reminders due *at or near* the unmute moment fire.
- A separate per-reminder mute. Each `ScheduledReminder` already has its own `Enabled` flag; this feature is the global override.

## Architecture

State of truth: `Settings.muted`. Both timers and `check_due_reminders` consult it.

```
                 ┌── settings.muted ──┐
                 │                    │
         ┌───────┴────────┐  ┌────────┴────────┐  ┌─────────────────────┐
         │ restart_timer  │  │restart_schedule │  │ check_due_reminders │
         │ early-return   │  │  early-return   │  │   early-return       │
         └────────────────┘  └─────────────────┘  └─────────────────────┘

  Tray menu "請勿打擾" — toggle muted + save + restart both timers
  Settings checkbox     — same effect on save
  show_reminder         — UNCHANGED (manual "立刻休息" must work while muted)
```

## Components

| Component | Responsibility |
|---|---|
| `src/settings.rs` (modified) | Add `muted: bool` field with `#[serde(default)]` (false). |
| `src/state.rs` (modified) | `restart_timer` and `restart_schedule_timer` early-return if `settings.muted` (also stop any running timer). New helper `apply_mute_state(state)` that calls both. |
| `src/schedule.rs` (modified) | `check_due_reminders` early-return if `settings.muted`. |
| `src/tray.rs` (modified) | New "請勿打擾" CheckMenuItem above "設定". On toggle: flip `state.settings.muted`, save, call `apply_mute_state`. Tooltip update via `tray.set_tooltip` when muted state changes. |
| `ui/MainWindow.slint` (modified) | New `<bool> muted` property + checkbox row above the existing autostart checkbox. |
| `src/main.rs` (modified) | Init `muted` Slint property; save closure writes `muted` and calls `apply_mute_state`. Wrap startup `check_due_reminders` with the muted check (already done by the function itself, but keep the explicit comment). |

## Data Flow

### Startup
1. `settings::load` reads `muted` (defaults to `false`).
2. `restart_timer` and `restart_schedule_timer` are called; if `muted`, they no-op.
3. `check_due_reminders` is called; if `muted`, it returns immediately.
4. Tray builder reads `state.settings.muted` to set initial menu checkmark and tooltip.

### Tray menu toggle
1. User clicks "請勿打擾". Hands the click event back to the UI thread via the existing thread_local pattern.
2. UI-thread closure: `state.settings.muted = !state.settings.muted`, save, call `apply_mute_state(&state)`, update tray menu checkmark + tooltip.
3. `apply_mute_state` stops then conditionally restarts both timers based on the new value, AND calls `check_due_reminders` if just-unmuted.

### Settings checkbox save
1. Save closure reads `window.get_muted()`, writes `state.settings.muted`, persists.
2. Calls `restart_timer`, `restart_schedule_timer`, `check_due_reminders` — these now respect the new value automatically.
3. Updates tray menu checkmark + tooltip (tray needs a way to reflect external changes — see below).

### Tray sync from Settings save

Two state-of-truth issues:
- The Settings checkbox can change `muted`, but the tray menu's checkmark only updates when something explicitly calls `set_checked(...)`.
- Same for the tooltip.

Approach: extend the existing `tray::install_state` thread_local to also hold the `Arc<CheckMenuItem>` and the `TrayIcon` handle (for tooltip). Expose `pub fn refresh_mute_visuals(muted: bool)` on `tray` that reads the thread_local and updates both. Settings save calls it; the tray's own toggle handler also calls it (for symmetry, though the menu-click natively toggles its own checkmark).

`CheckMenuItem` and `TrayIcon` are both `Send + Sync` per `tray-icon` 0.19, so they live alongside the existing AppState in the thread_local.

## Error Handling

- All operations are local: read/write `bool` field, start/stop slint::Timer, mutate tray MenuItem. Failures don't have meaningful recovery paths.
- `tray-icon` MenuItem operations (set_checked) return `Result<(), Error>`; we discard with `.ok()` like other tray ops.
- `tray.set_tooltip(...)` similarly: discard.

## Settings JSON

### Before (v0.4.0)
```json
{
  "IntervalMinutes": 45,
  "AutoStart": true,
  "PlaySound": false,
  "ScheduledReminders": [...]
}
```

### After
```json
{
  "IntervalMinutes": 45,
  "AutoStart": true,
  "PlaySound": false,
  "Muted": false,
  "ScheduledReminders": [...]
}
```

`#[serde(rename = "Muted", default)]` — bool default is `false`. No migration needed.

## Slint UI

`MainWindow.slint` adds (above the existing 登入時自動啟動 row):
```slint
in-out property <bool> muted: false;

CheckBox {
    text: "請勿打擾（暫停所有提醒）";
    checked <=> root.muted;
}
```

Window height stays the same (560px) — there's already enough headroom; no resize needed.

## Tray Menu

Order:
1. ✓ 請勿打擾  (checkmark visible only when muted)
2. ─ separator ─
3. 設定 (Settings)
4. 立刻休息 (Break Now)
5. ─ separator ─
6. 離開 (Exit)

`MenuItem::new` becomes `CheckMenuItem::new` for the mute item. The `tray-icon` crate's API:
```rust
let mute_item = CheckMenuItem::new("請勿打擾", true, state.borrow().settings.muted, None);
```

When toggled (via menu event), invert `state.settings.muted`, save, apply mute state, then call `mute_item.set_checked(state.borrow().settings.muted)` to keep the visual in sync if it didn't auto-update (it should, since the user click toggles it natively, but we set explicitly for symmetry with the Settings-checkbox path).

To make the Settings-save path also update the menu: stash a `Weak`/`Arc<CheckMenuItem>` somewhere reachable from the save closure. Simpler approach: extend the existing `tray::install_state` thread_local to ALSO hold the `Arc<CheckMenuItem>` and a tooltip handle; expose `tray::set_mute_visuals(muted: bool)` that reads the thread_local and updates both.

`CheckMenuItem` is `Send + Sync`-safe in `tray-icon`, so it can live in the `thread_local` alongside the existing AppState.

## Testing

No automated tests. Manual checklist:
- [ ] Fresh install: `Muted: false` in JSON; both timers running.
- [ ] Open Settings, tick "請勿打擾", save → `Muted: true`, both timers stopped, tooltip shows "(請勿打擾中)".
- [ ] Wait past the next interval — no reminder fires.
- [ ] Tray right-click → "請勿打擾" has a checkmark.
- [ ] Click "立刻休息" while muted → reminder still appears (manual override works).
- [ ] Click tray "請勿打擾" → checkmark disappears, tooltip reverts, both timers running again.
- [ ] If a scheduled reminder is due "now" at the moment of unmute → it fires immediately.
- [ ] Open Settings → "請勿打擾" checkbox reflects current state (false).
- [ ] Restart app → mute state persists.
- [ ] Old JSON without `Muted` field → loads as `false`, no errors.

## Resolved Decisions

1. **Naming**: `muted` / `Muted` / "請勿打擾".
2. **Default**: `false`.
3. **Affects**: interval timer + schedule timer + check_due_reminders. NOT manual "立刻休息".
4. **UI surfaces**: Settings checkbox AND tray CheckMenuItem (above 設定). Both surfaces always reflect the same state.
5. **Tooltip**: appended " (請勿打擾中)" when muted.
6. **Visual icon**: unchanged. Tooltip-only signal.
7. **Catch-up after unmute**: only reminders due *at or near* the unmute moment fire (via the standard 2-minute late-fire window). Past-due-during-mute fires are dropped.
8. **Persistence**: yes; `Muted` survives restarts.
