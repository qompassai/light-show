# light-show — Advanced Track Roadmap

Unlockable specialist characters + NOC console. These are the advanced
levels that prepare players for real field network technician/specialist
work. Unlocked after completing all levels for Séraphine, Ondine, and Linka.

## Characters

### Clara — Calix CMS provisioning (USA)
**Country contribution:** Calix is headquartered in San Jose, California.
"Clara" means clear/bright — what she's provisioning.
**Role:** Systems provisioning expert
**Levels teach:** Calix-platform ONT activation, service profiles, bulk
subscriber turn-up, CMS-style service templates.
**Mechanics:** Pick the right service profile, push to the right ONTs,
keep the PON power budget in window. Timed bulk-provisioning challenges.

### Aino — AMS network operations (Finland)
**Country contribution:** Nokia is Finnish (Espoo). The entire AMS platform
she supervises was born there. "Aino" is from the Kalevala.
**Role:** NOC supervisor
**Levels teach:** Alarm triage, severity escalation, dispatch decisions,
multi-fault scenarios.
**Mechanics:** The NOC console (see below). Live alarm feed, acknowledge/
dispatch/clear lifecycle. Cascading failures, prioritization under timer.

### Hikari — Outside plant & DFN buildout (Japan)
**Country contribution:** NTT pioneered FTTH at massive scale; Japanese
deployment playbooks defined modern DFN buildout. "Hikari" means light.
**Role:** Field network technician
**Levels teach:** Node/central cabinet work, mainline fiber routing, DFN
design with ratio splitters, primary/secondary/tertiary MST buildout.
**Mechanics:** Read tap plans, balance cascaded splitter trees, route
mainline fiber. Construction-grade puzzles.

### Léa — Telco admin test prep (Switzerland)
**Country contribution:** The ITU (International Telecommunication Union)
sits in Geneva — the body that sets global telecom standards.
**Role:** Exam prep coach
**Levels teach:** WA 09-Telecommunications Administrator exam prep.
NEC lookup, electrical theory, Washington RCW/WAC law.
**Mechanics:** Article Dungeon (NEC as explorable map), Lookup Sprints
(timed keyword → article), Theory Workshop (calculations), Boss Battles
(themed question sets), Readiness Meter. No harsh fail states — "not yet"
instead of "wrong." Designed for people who don't like tests.
**Content source:** `PhaedrusFlow/tds` `ta/quizzes/data/*.json` (see
`docs/LEA_QUIZ_IMPORT.md` for the import plan).

## NOC Console

AMS-style alarm management built on `osp_sim::Outage`:
- `AlarmSeverity`: Critical/Major/Minor/Warning (from outage kind + timer)
- `AlarmAck`: New → Acknowledged → Resolved → Cleared
- `AlarmList`: Bevy resource, multiple concurrent alarms
- New `GameState::NocConsole` (resolves to Results, never back to Playing)

See `game/src/states/TODO.md` for implementation tasks.

## Phases

### Phase 1: NOC foundation (Aino)
- [x] `osp_sim` alarm types (`AlarmSeverity`, `AlarmAck`, `Alarm`)
- [ ] `AlarmList` Bevy resource
- [ ] NOC console UI (`states/noc.rs`)
- [ ] Aino sprite sheet + dialogue
- [ ] 3-4 NOC tutorial levels

### Phase 2: Calix provisioning (Clara)
- [ ] Service profile data model
- [ ] Bulk provisioning mechanics
- [ ] Clara sprite sheet + dialogue  
- [ ] 4-5 provisioning levels

### Phase 3: OSP buildout (Hikari)
- [ ] Ratio splitter support in `osp_sim`
- [ ] MST/tap plan data model
- [ ] Hikari sprite sheet + dialogue
- [ ] 5-6 construction levels

### Phase 4: Telco admin prep (Léa)
- [ ] Quiz data import (`ta/quizzes/data/*.json` → game format)
- [ ] Article Dungeon (NEC navigation map)
- [ ] Lookup Sprint mechanics
- [ ] Theory Workshop (calculation puzzles)
- [ ] Boss Battles per exam domain
- [ ] Readiness Meter
- [ ] Léa sprite sheet + dialogue

### Phase 5: Integration
- [ ] Unlock system (complete 3 base girls → unlock specialists)
- [ ] Cross-character dispatch (Aino dispatches Hikari to field repairs)
- [ ] Advanced mixed scenarios

## Art pipeline

Each character needs (following the existing companion pipeline):
- Base portrait (done for Clara/Aino/Hikari/Léa — see
  `~/workspace/light-show/new-characters/`, magenta keyed)
- Mature variant
- Expression set (11 expressions × base/mature)
- Idle animation GIF
- Sprite sheet for in-game use

See `art/TODO.md` for per-character art tasks.
