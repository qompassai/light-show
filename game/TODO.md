# game/ — TODO

> Tracked on GitHub: [master TODO #2](https://github.com/qompassai/light-show/issues/2) -> [this track](https://github.com/qompassai/light-show/issues/3). Close the sub-issue to check off an item; completed items get a date + what/where below.

Main Bevy game crate.

## NOC console (Aino track)
- [ ] Add `GameState::NocConsole` to `src/states/mod.rs`
- [ ] Create `src/states/noc.rs` (alarm list UI, ack/dispatch/clear)
- [ ] `AlarmList` Bevy resource (replaces single `ActiveOutage` view)
- [ ] Wire `playing::check_scripted_outage` → `AlarmList::raise()`
- [ ] NOC banner shows highest-severity unacked alarm

## Unlock system
- [ ] Track completion of Séraphine/Ondine/Linka level sets
- [ ] Unlock Clara/Aino/Hikari in companion select when complete
- [ ] Locked character UI (silhouette + unlock requirement text)

## Clara (provisioning)
- [ ] Service profile data model in `level.rs`
- [ ] Bulk provisioning level mechanics

## Hikari (OSP)
- [ ] MST/tap plan rendering in `board.rs`
