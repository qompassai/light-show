# game/src/states/ — TODO

## noc.rs (new)
- [ ] Alarm list widget (sorted: Critical → Warning, then by raised_at)
- [ ] Severity color coding (red/amber/yellow/blue)
- [ ] Acknowledge button → dispatch companion selector
- [ ] Clear button for Resolved alarms
- [ ] History view (Cleared alarms)
- [ ] Reuse banner animation from `outage.rs`

## outage.rs
- [ ] Refactor to read from `AlarmList` instead of `ActiveOutage`
- [ ] Banner shows highest-severity unacked alarm
- [ ] Preserve "never back to Playing" invariant
