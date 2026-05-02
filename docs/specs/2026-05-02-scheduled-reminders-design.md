# Scheduled Reminders — Design

**Date:** 2026-05-02
**Status:** Approved (brainstorming phase)
**Builds on:** v0.3.0-play-sound

## Goal

Restore the WPF version's "scheduled reminders" feature: a list of named reminders that fire at specific times on selected days of the week. Each reminder has its own message, shown in the reminder window in place of the default text. Settings JSON layout is byte-compatible with the WPF version, so a user with an existing `BreakReminder\settings.json` can copy `ScheduledReminders` over to `BreakReminderRs\settings.json` and have it work.

## In Scope

- New `ScheduledReminder` model: `enabled: bool`, `time: NaiveTime`, `days: DayOfWeekMask`, `message: String`, `last_fired_date: Option<NaiveDate>`.
- `DayOfWeekMask` as a `bitflags` 7-bit set; serializes as the WPF strings (`"Weekdays"`, `"All"`, `"Mon, Tue, Fri"`, etc.).
- New `scheduled_reminders: Vec<ScheduledReminder>` field on `Settings` (JSON key `ScheduledReminders`).
- Seed defaults on first run (no JSON or no `ScheduledReminders` field): three reminders matching WPF defaults.
- A second `slint::Timer` running every 30 seconds, calling `schedule::check_due_reminders` to fire any due reminders.
- Reminder window accepts an `Option<&str>` message; default text shows when `None`.
- Settings UI gains an editable list of scheduled reminders with enable / time / day-mask / message / delete columns, plus an "新增提醒" button.

## Out of Scope (vs. WPF parity)

- **Pending message queue**: WPF queues messages while a reminder window is open. Rust MVP drops the second message but still marks `last_fired_date = today`, so the user can miss a same-minute reminder. Acceptable: same-minute scheduled fires are rare, and the simpler design avoids a separate queue + window-closed callback. Documented as future work.
- **Timezone handling**: both versions use system local time; no UTC.

## Architecture

A new `schedule` module owns the reminder model, JSON serde, and the `check_due_reminders` function. `state.rs` adds a second `slint::Timer` that ticks every 30 seconds; the tick handler calls into `schedule`. The reminder window's text becomes a property; `show_reminder` accepts an optional override.

```
                ┌──── Slint Event Loop ───────────────┐
                │                                     │
   45-min Timer │      Rc<RefCell<AppState>>          │
   (interval) ──┤  ├ settings (with reminders Vec)    │
                │  ├ timer (existing)                 │
   30-sec Timer │  ├ schedule_timer (NEW)             │
   (schedule) ──┤  └ reminder_window                  │
                └─────────┬───────────────────────────┘
                          │
            schedule_timer tick
                          │
                          ▼
            schedule::check_due_reminders(state)
                          │
                          ├ for each due reminder:
                          │   - mark last_fired_date = today
                          │   - state.save()
                          │   - show_reminder(&state, Some(message))
                          │
                          └ idempotent within a calendar day
```

## Components

| Component | Responsibility |
|---|---|
| `src/schedule.rs` (new) | `DayOfWeekMask` (bitflags), `ScheduledReminder` struct, custom serde for WPF-compatible JSON, `check_due_reminders(state)`, `seed_defaults()` |
| `src/settings.rs` (modified) | Add `scheduled_reminders: Vec<ScheduledReminder>` field; `Default` impl seeds defaults |
| `src/state.rs` (modified) | Add `schedule_timer: slint::Timer`; new `restart_schedule_timer` method; modify `show_reminder` signature to take `Option<&str>` |
| `ui/ReminderWindow.slint` (modified) | Replace hardcoded body text with `in property <string> message: "現在立刻離開座位..."` |
| `ui/MainWindow.slint` (modified) | Add scheduled-reminders section: header row + `for` loop binding to a `[ScheduledReminderRow]` model + "新增提醒" button. Window height grows ~250→540, with overflow handled by an inner ScrollView |
| `src/main.rs` (modified) | At startup: `schedule::check_due_reminders` + `restart_schedule_timer`. Bind a Slint `VecModel` to `app_state.borrow().settings.scheduled_reminders`. Save closure writes the model back to settings, saves JSON, restarts schedule timer |

`MainWindow.slint` will grow significantly; if it crosses ~120 lines I'll consider splitting the scheduled-reminders section into its own `.slint` component imported via `app.slint`. Otherwise inlined is fine for the MVP.

## Data Flow

### Startup

1. `settings::load` deserialises JSON. Missing `ScheduledReminders` field → empty `Vec`. Empty `Vec` → seed three defaults via `Settings::default()` then save back.
2. `main()` calls `schedule::check_due_reminders(&app_state)` once. Catches "app launched after 11:55, fire the lunch reminder if not yet fired today."
3. `restart_schedule_timer(&app_state)` starts the 30-second tick.
4. Existing 45-minute interval timer keeps running.

### Schedule check (every 30 seconds, and once at startup)

For each reminder:
1. Skip if `!enabled`.
2. Skip if `last_fired_date == today`.
3. Skip if `!days.contains(today.weekday())`.
4. Skip if `current_time < reminder.time`.
5. Skip if `current_time - reminder.time > Duration::minutes(2)`. (Prevents firing way-late reminders, e.g., user booted at noon and the 09:00 reminder shouldn't pop.)
6. Otherwise: set `last_fired_date = today`, call `state.save()`, call `show_reminder(&state, Some(&reminder.message))`.

### Edit reminders (Settings UI)

1. Open Settings. UI's scheduled-reminders model is populated from `app_state.borrow().settings.scheduled_reminders`.
2. User toggles, edits time, edits days, edits message, deletes a row, or clicks "新增提醒".
3. Click "儲存並隱藏":
   - Read the Slint model back, replace `state.settings.scheduled_reminders` (preserving `last_fired_date` only when matching a previously-existing reminder by index — see "Identity" below).
   - `state.save()`.
   - `AppState::restart_schedule_timer(&state)` (just restarts the 30s ticker; doesn't reset history).
4. Hide window.

### Identity for `last_fired_date` preservation

Reminders have no stable ID. When the user edits time/message/days, the reminder is "the same one" semantically; when they delete row N, the array shrinks. Strategy:
- Snapshot `last_fired_date` from the existing `Vec` in **index order** before reading the UI model.
- After reading the UI model, copy `last_fired_date` back at matching indices, **but only when the time/message hasn't changed materially**. If the user edited fields, treat it as a "new" reminder for that day → `last_fired_date = None`.
- Rationale: the WPF version doesn't preserve `LastFiredDate` across edits either (the editor regenerates the list and saves), so this matches WPF behaviour.

Pragmatic simplification: **don't preserve `last_fired_date` across saves at all**. After save, re-evaluate `check_due_reminders` immediately so a same-minute fire still triggers if appropriate. This matches WPF and is easier to reason about. Choose this option.

## JSON Format (WPF-compatible)

```json
{
  "IntervalMinutes": 45,
  "AutoStart": true,
  "PlaySound": false,
  "ScheduledReminders": [
    {
      "Enabled": true,
      "Time": "11:55",
      "Days": "Weekdays",
      "Message": "吃飯囉！",
      "LastFiredDate": "2026-05-02"
    },
    { "Enabled": true, "Time": "19:00", "Days": "Weekdays", "Message": "下班時間到！" },
    { "Enabled": true, "Time": "00:00", "Days": "All", "Message": "該睡覺了！" }
  ]
}
```

- `Time`: `"HH:mm"` (24-hour). `chrono::NaiveTime::format("%H:%M")` produces this.
- `Days`: WPF C# enum-flag string. We write a custom (de)serializer because the `bitflags` crate's default `Display` produces `"Mon | Tue"`, not the comma format the WPF version uses.
  - **On write**: if mask matches Weekdays exactly → `"Weekdays"`; if all 7 days → `"All"`; if zero → `"None"`; otherwise comma+space-separated days in calendar order: `"Sun, Mon, Tue, Wed, Thu, Fri, Sat"` (only the present days).
  - **On read**: split by comma OR by `|` (tolerant of both formats). Trim each token. Recognise `"Weekdays"`, `"All"`, `"None"`, and the seven day names (`"Sun"`, `"Mon"`, `"Tue"`, `"Wed"`, `"Thu"`, `"Fri"`, `"Sat"`). Unknown tokens → log + skip.
- `LastFiredDate`: `"yyyy-MM-dd"`. Field omitted when `None` via `#[serde(skip_serializing_if)]`.

## Slint UI

`ReminderWindow.slint`:
- New property: `in property <string> message: "現在立刻離開座位，喝杯水，讓眼睛過濾一下吧！";`
- The smaller Text now binds to `root.message`.

`MainWindow.slint` adds (below the existing controls, above the Save button):

```slint
// existing structs + a new one for the scheduled reminder rows
struct ScheduledReminderRow {
    enabled: bool,
    time-text: string,   // bound to a TextInput; main.rs parses HH:mm
    mon: bool, tue: bool, wed: bool, thu: bool, fri: bool, sat: bool, sun: bool,
    message: string,
}

in-out property <[ScheduledReminderRow]> reminders;
callback add-reminder();
callback delete-reminder(int);  // index

// header row
HorizontalLayout {
    spacing: 6px;
    Text { text: "啟用"; width: 36px; horizontal-alignment: center; font-size: 11px; color: gray; }
    Text { text: "時間"; width: 60px; horizontal-alignment: center; font-size: 11px; color: gray; }
    Text { text: "一 二 三 四 五 六 日"; width: 196px; horizontal-alignment: center; font-size: 11px; color: gray; }
    Text { text: "訊息"; horizontal-alignment: left; font-size: 11px; color: gray; }
}

// scrollable list
ScrollView {
    height: 200px;
    VerticalLayout {
        for r[i] in root.reminders : HorizontalLayout {
            spacing: 6px;
            CheckBox { checked <=> r.enabled; }
            LineEdit { text <=> r.time-text; width: 60px; }
            // 7 day toggles, each ~26px wide
            CheckBox { text: "一"; checked <=> r.mon; }
            // ... etc
            LineEdit { text <=> r.message; horizontal-stretch: 1; }
            Button { text: "✕"; width: 32px; clicked => { root.delete-reminder(i); } }
        }
    }
}

Button {
    text: "+ 新增提醒";
    clicked => { root.add-reminder(); }
}
```

Window height grows from 250px to ~560px to fit the section (header + 200px scroll + add button + existing controls).

`main.rs` wires:
- A `slint::VecModel<ScheduledReminderRow>` populated from settings on startup.
- `on_add_reminder` callback pushes a new default row to the model.
- `on_delete_reminder` callback removes the row at the given index.
- Save closure reads the model back, parses each `time-text` (`HH:mm`), assembles `Vec<ScheduledReminder>`, replaces `state.settings.scheduled_reminders`, saves.

## Dependencies

Add to `[dependencies]`:
```toml
bitflags = "2"
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
```

`chrono` is needed for `NaiveTime`, `NaiveDate`, `Local::now()`, and `Duration` arithmetic. `default-features = false` keeps the dep light (no `wasmbind` or `unstable-locales`).

## Error Handling

- Time parse failure on a row in the UI ("not HH:mm"): keep the row but skip during schedule check (don't crash). Log to stderr.
- JSON parse failure on a single reminder: skip that reminder, keep others, log to stderr.
- Schedule check internal failures (date math, etc.): log + continue with next reminder.

## Settings JSON Migration

- v0.3.0 JSON without `ScheduledReminders` field → on load: `scheduled_reminders = vec![]`. After `Default::default()` is consulted via `#[serde(default = "...")]`, seed the three defaults. First save writes them back.
- v0.3.0 JSON with empty `ScheduledReminders` array → treated as "user explicitly cleared the list", do NOT seed. Empty list persists.
- WPF JSON copied in: `ScheduledReminders` field's content is read directly. The WPF `Days` strings (incl. `"Weekdays"`/`"All"`) are recognised.

## Testing

No automated tests, matching the project policy. Manual checklist:

- [ ] Fresh install (no settings.json): three default reminders appear in Settings UI.
- [ ] Edit one reminder's time to current+1 minute, save → reminder fires at that minute with the correct message.
- [ ] After firing, restart app within the same minute → does NOT fire again (last_fired_date prevents).
- [ ] Wait until tomorrow (or change system date) → fires again.
- [ ] Delete a reminder, save, reopen Settings → row is gone.
- [ ] Add a new reminder via the button, save, reopen → row is present with whatever was entered.
- [ ] Set day-mask to weekends only, fire on a weekday → does not fire.
- [ ] Save: JSON file contains expected structure (matches WPF format).
- [ ] Copy ScheduledReminders block from a WPF settings.json → loads correctly.
- [ ] Bad time string ("99:99") in UI → save succeeds (kept as-is), schedule check skips that row, no crash.

## Resolved Decisions

1. **Bitmask crate**: `bitflags 2`.
2. **Time/date crate**: `chrono` (default-features off, `clock` + `serde`).
3. **JSON format**: WPF-compatible byte-for-byte, including `"Weekdays"`/`"All"` aliases.
4. **Default reminders**: 11:55 weekdays "吃飯囉！", 19:00 weekdays "下班時間到！", 00:00 daily "該睡覺了！".
5. **Schedule check cadence**: 30 seconds (matches WPF).
6. **Late-fire window**: 2 minutes (matches WPF).
7. **`last_fired_date` across saves**: NOT preserved. Save → re-run `check_due_reminders` → idempotent.
8. **Pending messages queue**: NOT implemented. Reminder during reminder is dropped.
9. **UI layout**: inline scrollable list in MainWindow.slint; if file passes ~120 lines after this change, split into a sub-component.
10. **"立刻休息" message**: stays as `None` (default text), matching WPF.
