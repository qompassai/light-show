# game/src/ — TODO

## states/
- [ ] `noc.rs`: NOC console (see game/TODO.md)
- [ ] `outage.rs`: refactor `ActiveOutage` → view over `AlarmList`

## waifu/
- [ ] Calista sprite integration + mood mappings
- [ ] Amara sprite integration + NOC-specific moods (Focused, Urgent)
- [ ] Terra sprite integration + field moods (Determined, Tired)
- [ ] Dispatch system: Amara can dispatch Terra (cross-character)

## level.rs
- [ ] `ServiceProfile` for Calista provisioning levels
- [ ] `TapPlan` for Terra MST buildout levels
- [ ] Unlock requirement field on `LevelDef`
