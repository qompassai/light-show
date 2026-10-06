# NOC Console — Design Document

Amara's track. The player leaves the single-board puzzle view and runs the
network like a NOC tech: a live alarm list, acknowledge / dispatch / clear
workflow, and a severity banner that follows them back into the field.

## Concept

Up to now every outage has been a scripted solo event: one fault, one board,
one timer. The NOC console teaches the real job — **triage**. Alarms arrive
from all four disciplines at once, each aging on its own clock, each with a
customer-impact severity. The player can't fix everything first; they pick
the order, and the order matters.

## Game states

- `GameState::NocConsole` (new, in `game/src/states/mod.rs`): the console
  screen itself — alarm list, detail pane, ack/dispatch/clear controls.
- Entry: from the companion picker (Amara's card) and from a hotkey/banner
  tap during `Playing`.
- Exit: Back returns to the previous state (picker or the paused level).

## Data model

`AlarmList` Bevy resource (replaces the single `ActiveOutage` view):

- `alarms: Vec<Alarm>` — `Alarm` comes from `osp_sim` (`id`, `outage`,
  `severity`; severity in `Critical | Major | Minor | Warning`).
- `AlarmList::raise(outage)` — assigns the next monotonic id, pushes.
- `ack(id)` — marks acknowledged; acked alarms sort below unacked.
- `clear(id)` — removes; only allowed when `outage.resolved`.
- `highest_unacked() -> Option<&Alarm>` — drives the banner.

Wiring: `playing::check_scripted_outage` calls `AlarmList::raise()` instead
of raising the solo outage view. Existing solo-outage levels keep working —
one alarm in the list behaves like the old flow.

## UI layout

- **Left: alarm list.** Rows sorted by (unacked first, severity desc, age
  desc). Each row: severity chip (color-coded), discipline icon, one-line
  fault description, age timer. Selecting a row fills the detail pane.
- **Right: detail pane.** Full outage description, affected segment, the
  level's target window, buttons: **Acknowledge**, **Dispatch** (jumps into
  the level's board with this alarm focused), **Clear** (enabled only when
  resolved).
- **Top: NOC banner.** Persistent across `Playing` and `NocConsole`: shows
  the highest-severity unacked alarm ("CRITICAL: fiber cut — Segment 4").
  Tapping it opens the console. Empty state: "ALL CLEAR" in dim green.

## Severity and aging

Severity comes from `osp_sim` and refreshes as the outage ages
(`Alarm::refresh_severity`): a Warning left alone becomes Minor, then
Major; a Major near its complaint timer escalates to Critical. This is the
core pressure — ignoring the list is a choice with a visible cost.

## Win condition (Amara track)

Each NOC level ships a scripted alarm cascade (see `scripts/TODO.md`: NOC
scenario generator). Win = every alarm acked *and* cleared before its
complaint timer fires. Score ranks on mean time-to-ack and worst severity
reached.

## Out of scope

Real trap/streaming integration, multi-player, or persistent NOC state
between sessions. The console is a per-level simulation.
