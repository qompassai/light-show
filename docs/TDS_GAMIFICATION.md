# TDS Resources Gamification — Design

Maps Matt's TDS field resources (`PhaedrusFlow/tds`) into light-show
characters and mechanics. Source docs are vendored as reference at
`docs/reference/tds/{ams,cms,bxe,ta}/` until fully built out.

## Character mapping

| Resource | What it is | Character | Why |
|---|---|---|---|
| `ams/` | Network management: monitoring, bulk NE ops, NBI/SOAP, logs, security, OS hardening | **Aino** (NOC) | AMS is the management plane; Aino runs the NOC console. |
| `cms/` | Calix CMS: AXOS/EXOS provisioning, NBI API, SMx, release notes | **Clara** (provisioning) | CMS is the provisioning plane; Clara provisions PON. |
| `bxe/` | BxE field portal: customer diagnostics by address, install/service certification, portal workarounds | **Hikari** (OSP/field) | Field-tech daily work; Hikari owns the outside plant. |
| `ta/` | Training Academy: quizzes, flashcards, laws, mock exams | **Léa** (study) | Already mapped in `docs/LEA_QUIZ_IMPORT.md`. |

## Aino ← ams/ (NOC operations)

The `ams/` docs (command index, monitoring, bulk operations, NBI/SOAP,
logs) become Aino's NOC console content:
- **Alarm triage levels**: scripted cascades drawn from `11-logs-monitoring-support.md`
  and `13-use-cases.md` — the faults read like real NOC tickets.
- **Bulk-ops mechanic**: `09-ne-bulk-operations.md` inspires a multi-alarm
  ack/dispatch flow — select several alarms, apply one action, watch the
  severity board update.
- **NBI/SOAP puzzles**: `10-nbi-soap-reference.md` becomes a "query the
  northbound interface" mini-game — construct the right query to pull the
  fault data before the timer expires.

## Clara ← cms/ (provisioning)

The `cms/` docs (AXOS/EXOS provisioning, NBI API, SMx) become Clara's
provisioning track content:
- **EXOS provisioning levels**: `exos-provisioning.md` provides the real
  workflow — the player's in-game steps mirror the actual provision
  sequence (create subscriber, assign profile, activate ONT).
- **NBI API puzzles**: `cms-nbi-api.md` + `smx-api.md` become "drive the
  API" challenges — the right call sequence provisions the subscriber;
  wrong calls raise alarms Aino has to triage (cross-character linkage).
- **MDU scenarios**: `smartmdu.md` gives multi-dwelling levels — one
  building, many subscribers, one budget.

## Hikari ← bxe/ (field diagnostics)

The `bxe/` guides (address-to-diagnostics, certification, portal
workarounds) become Hikari's field track:
- **Address-to-diagnostics levels**: the 10-minute diagnostic workflow
  becomes a timed level — take an address, run the checks in order, find
  the fault before the customer callback timer fires.
- **Certification runs**: `certification.md` becomes the "certify the
  install" finale per level — all tests green before the level counts.
- **Portal-workaround hazards**: `slow-portal-workarounds.md` inspires
  outage events — the tool you need is slow, so you diagnose from
  partial data.

## Léa ← ta/ (study)

Already designed in `docs/LEA_QUIZ_IMPORT.md`: NEC quizzes, WA law,
flashcards, and mock exams become Léa's quiz-boss levels.

## Build order

1. Reference vendored (done — `docs/reference/tds/`).
2. Clara + cms/ (provisioning track is already the priority).
3. Aino + ams/ (NOC console consumes the alarm docs).
4. Hikari + bxe/ (field diagnostics after OSP mechanics).
5. Léa + ta/ (quiz importer per LEA_QUIZ_IMPORT.md).
