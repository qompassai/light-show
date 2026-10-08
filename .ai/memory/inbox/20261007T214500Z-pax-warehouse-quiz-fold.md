# Warehouse quiz folded into the answer-tag scheme

Affected paths: game/src/warehouse/mod.rs, game/src/states/warehouse.rs,
tests/answer_gates.rs, tools/tag_answers.py, docs/BUILD.md.

Non-obvious facts:
- The warehouse QuizQuestion (game/src/warehouse/mod.rs) now carries a
  stable `id` (scout_pro_3-001..008, the tag scope) and a keyed
  `correct_tag` instead of plaintext `correct_idx` — same scheme as
  Léa's questions: HMAC-SHA256 over `quiz:{id}:{correct_idx}` under
  LIGHT_SHOW_ANSWER_KEY (see game/src/answer_verify.rs). Grading calls
  `verify_choice`; `reveal_correct` recovers the index by candidate
  testing, so a keyless build cannot pass the warehouse quiz
  (fail-closed, like the levels).
- No refactor was needed for the explanation prose: explain_correct /
  explain_wrong render only in the post-answer reaction branches of
  states::warehouse, i.e. they are post-answer teaching text (the
  class the design blesses). The only pre-answer correctness signal
  was correct_idx itself, and it is gone: the tree now has zero
  plaintext warehouse answer fields (the one remaining "correct_idx"
  string in the module is the doc comment naming the tag payload
  format).
- Store: 8 new entries `warehouse/scout_pro_3/scout_pro_3-00N`
  ({kind: warehouse_quiz_correct_idx, level: warehouse, value}) in the
  primo sops answer store — 222 entries total. tools/tag_answers.py
  covers the warehouse in the same run as the level files: it tags
  plaintext literals drift-checked against the store and re-verifies
  tagged literals on every later run (idempotent; that verification
  is the regeneration path — update the store entry, restore the
  literal's plaintext correct_idx line, re-run).
- TWO latent defects in the shipped tools/tag_answers.py were found
  by this fold and repaired (both proven against scratch copies):
  (1) its drift-check lookups used a dotted entry-id format that
  never existed in the store (build_store.py keys entries
  `quiz/<level>/<question-id>` etc.), so any plaintext-authored level
  aborted with "store drift: None != <idx>"; (2) it HMAC'd with the
  raw bytes the key hex decodes to, while the verifier HMACs with
  the key string's own bytes — tags from the shipped tool would not
  have verified in-game. Both are fixed; the tool now tags a restored
  plaintext question and verifies the warehouse literals end-to-end.
- Gates at landing: keyed full workspace suite 795 passed / 0 failed
  (baseline 794 + the new shipped-warehouse gate test), with the
  answer_gates warehouse test printing its keyed branch (tags verify,
  wrong answer rejected, tampered tag rejected). Keyless fresh-target
  run: lib 433/0, answer_gates + game_quiz + game_levels green, the
  warehouse gate printing its fail-closed branch (nothing verifies,
  nothing recovers). cargo fmt: the 5 changed files are clean; HEAD
  already carries fmt drift in 7 untouched files and 98 clippy
  diagnostics in untouched files (zero in the changed files) — both
  pre-existing, not introduced here.
- Tests: warehouse unit tests are dual-branch in the keyed/keyless
  sense (structural checks unconditional; answer texts asserted when
  the key resolves); states::warehouse answer-flow tests recover-or-
  skip under the build key, mirroring tests/game_quiz.rs.
- F-Droid is unaffected: its recipe stays pinned to the last
  plaintext commit 4e1711d (the channel split is unchanged — the
  pinned tree predates this fold).
