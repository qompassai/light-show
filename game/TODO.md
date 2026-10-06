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
- [x] Service profile data model in `level.rs` (2026-10-05; closes #16)
  - `ServiceProfile` struct + `SERVICE_PROFILES` catalog (GPON-100/500, XGS-1000) in `game/src/level.rs`
  - `SubscriberDef` (name, node, profile, distance_km, reg_id) with EXOS Registration ID validation
  - `service_profile()` lookup, `verify_provisioning()` pure verification function
- [x] Bulk provisioning level mechanics (2026-10-05; closes #17)
  - `LevelDef.subscribers` field; `is_provisioning_win()` checks placed splitter, port count (`branch_count`), and per-subscriber Rx windows
  - Splitter ratios 1:4/1:8/1:16/1:32 via existing `osp_sim::SplitterRatio`
  - Two playable levels: `game/assets/levels/clara1_provisioning.json` (9 subs, port-planning puzzle), `game/assets/levels/clara2_outage.json` (6 subs, WaterIntrusion, tired OLT)
  - `Companion::Clara.track_start_index()` → `Some(8)` in `game/src/waifu/mod.rs`
  - 12 new tests (provisioning validation/adversarial, reg_id rules, winning-pill counts); 391 workspace tests green
- [x] Clara 10-level provisioning track (2026-10-05)
  - 8 new levels: clara3_dead_box_swap (XGS intro), clara4_profile_audit (overload trap),
    clara5_drive_the_nbi (NBI API), clara6_bulk_turn_up (hybrid), clara7_smx_lifecycle (SMx API),
    clara8_building_turn_up (MDU 12 subs), clara9_tight_budget (hot OLT 1:32),
    clara10_night_cutover (expert: 14 subs + outage + API + hot OLT)
  - New API-sequence mechanic in `game/src/states/api_console.rs`: `ApiOp` enum,
    `ApiSequenceDef`, `verify_api_sequence()`, console UI with alarm counter;
    win-gated in both `playing::check_win_condition` and `outage::check_outage_resolution`
  - `Companion::track_indices()` → Clara at 40-49; `track_len()` = 10
  - Playthrough test covers all 10 levels via real input; 5 api_console unit tests
  - Difficulty progression: tutorial (ports) → XGS → outage → API → hybrid → MDU → hot OLT → expert capstone

## Hikari (OSP)
- [ ] MST/tap plan rendering in `board.rs`
