# game/ — TODO

Main Bevy game crate.

## NOC console (Amara track)
- [ ] Add `GameState::NocConsole` to `src/states/mod.rs`
- [ ] Create `src/states/noc.rs` (alarm list UI, ack/dispatch/clear)
- [ ] `AlarmList` Bevy resource (replaces single `ActiveOutage` view)
- [ ] Wire `playing::check_scripted_outage` → `AlarmList::raise()`
- [ ] NOC banner shows highest-severity unacked alarm

## Unlock system
- [ ] Track completion of Séraphine/Ondine/Linka level sets
- [ ] Unlock Calista/Amara/Terra in companion select when complete
- [ ] Locked character UI (silhouette + unlock requirement text)

## Calista (provisioning)
- [ ] Service profile data model in `level.rs`
- [ ] Bulk provisioning level mechanics

## Terra (OSP)
- [ ] MST/tap plan rendering in `board.rs`
