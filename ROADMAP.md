# light-show — Advanced Track Roadmap

Unlockable specialist characters + NOC console. These are the advanced
levels that prepare players for real field network technician/specialist
work. Unlocked after completing all levels for Séraphine, Ondine, and Linka.

## Characters

### Calista — Calix CMS provisioning
**Role:** Systems provisioning expert
**Levels teach:** Calix-platform ONT activation, service profiles, bulk
subscriber turn-up, CMS-style service templates.
**Mechanics:** Pick the right service profile, push to the right ONTs,
keep the PON power budget in window. Timed bulk-provisioning challenges.

### Amara — AMS network operations  
**Role:** NOC supervisor
**Levels teach:** Alarm triage, severity escalation, dispatch decisions,
multi-fault scenarios.
**Mechanics:** The NOC console (see below). Live alarm feed, acknowledge/
dispatch/clear lifecycle. Cascading failures, prioritization under timer.

### Terra — Outside plant & DFN buildout
**Role:** Field network technician
**Levels teach:** Node/central cabinet work, mainline fiber routing, DFN
design with ratio splitters, primary/secondary/tertiary MST buildout.
**Mechanics:** Read tap plans, balance cascaded splitter trees, route
mainline fiber. Construction-grade puzzles.

## NOC Console

AMS-style alarm management built on `osp_sim::Outage`:
- `AlarmSeverity`: Critical/Major/Minor/Warning (from outage kind + timer)
- `AlarmAck`: New → Acknowledged → Resolved → Cleared
- `AlarmList`: Bevy resource, multiple concurrent alarms
- New `GameState::NocConsole` (resolves to Results, never back to Playing)

See `game/src/states/TODO.md` for implementation tasks.

## Phases

### Phase 1: NOC foundation (Amara)
- [ ] `osp_sim` alarm types (`AlarmSeverity`, `AlarmAck`, `Alarm`)
- [ ] `AlarmList` Bevy resource
- [ ] NOC console UI (`states/noc.rs`)
- [ ] Amara sprite sheet + dialogue
- [ ] 3-4 NOC tutorial levels

### Phase 2: Calix provisioning (Calista)
- [ ] Service profile data model
- [ ] Bulk provisioning mechanics
- [ ] Calista sprite sheet + dialogue  
- [ ] 4-5 provisioning levels

### Phase 3: OSP buildout (Terra)
- [ ] Ratio splitter support in `osp_sim`
- [ ] MST/tap plan data model
- [ ] Terra sprite sheet + dialogue
- [ ] 5-6 construction levels

### Phase 4: Integration
- [ ] Unlock system (complete 3 base girls → unlock specialists)
- [ ] Cross-character dispatch (Amara dispatches Terra to field repairs)
- [ ] Advanced mixed scenarios

## Art pipeline

Each character needs (following the existing companion pipeline):
- Base portrait (done: see `art/aseprite/` for existing girls)
- Mature variant
- Expression set (11 expressions × base/mature)
- Idle animation GIF
- Sprite sheet for in-game use

See `art/TODO.md` for per-character art tasks.
