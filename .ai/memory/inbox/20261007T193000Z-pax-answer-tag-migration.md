# Answer-tag migration landed (Option B, keyed HMAC tags)

Affected paths: game/src/answer_verify.rs (new), game/build.rs (new),
game/src/level.rs, game/src/states/{quiz,api_console,triage_console}.rs,
27 level JSONs under game/assets/levels/, tests/{answer_gates.rs (new),
game_quiz.rs, game_levels.rs, Cargo.toml}, tools/tag_answers.py (new),
scripts/publish/play-build-aab.sh, docs/{BUILD,FDROID}.md,
fdroid/ai.qompass.lightshow.yml.

Non-obvious facts:
- Level answers are HMAC-SHA256 tags now (domains quiz / api_sequence /
  alarm_triage), keyed by LIGHT_SHOW_ANSWER_KEY via option_env!. The
  production key lives ONLY in the primo sops answer store
  (answer_key.production) plus release-build environments; never in the
  repo. game/build.rs declares rerun-if-env-changed for the key --
  cargo does not track option_env! on its own.
- Fail-closed by design: a build without the matching key verifies
  nothing. tests/answer_gates.rs proves both branches (keyed: verify +
  wrong/tamper rejected; keyless: nothing verifies, nothing recovers).
  Gates at landing: keyed full suite 793/0; keyless lib 432/0 +
  answer_gates/game_quiz/game_levels green in a fresh target dir.
- Per-step prefix tags exist because the consoles give per-click
  feedback and a next-op hint; DESIGN.md specced only full-sequence
  tags. The last step tag equals the full tag.
- Channel split: Play/Windows = tagged tree + injected key
  (play-build-aab.sh --live loads the key from the store and dies
  without it). F-Droid recipe is PINNED to the last plaintext commit
  4e1711da1a3eb87f84f708f39a8319a5f28f90f9 -- do not bump past the tag
  migration without revisiting the split.
- tools/import_quiz.py never existed in this repo; the level.rs doc
  reference was aspirational (docs/LEA_QUIZ_IMPORT.md is the plan doc).
  tools/tag_answers.py is the tagging path now: author plaintext
  locally, tag on primo against the store (drift-checked), commit tags.
- Conscious exceptions: power windows (83) and playthrough/test
  fixtures stay plaintext (tuning/fixture data). The warehouse quiz
  (game/src/warehouse/mod.rs) has its OWN QuizQuestion with plaintext
  correct_idx in code -- not in the audit classes; its explain_wrong
  prose names the answers anyway. Flagged, not migrated.
- cfg(test) builds fall back to a fixed TEST_KEY so unit tests are
  self-consistent keyless; integration tests link the non-test lib and
  skip answer-dependent assertions when the key does not resolve the
  shipped tags.
