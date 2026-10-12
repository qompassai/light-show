# Overnight verification pass on the primo mirror (2026-10-12)

- Scope: verification only, run in a fresh worktree off the primo mirror
  (~/.GH/Qompass/LightShow) at cd67814. ~/workspace/repos/light-show was
  not touched; the api36 and splice-tray branches were not touched.
- Suite: workspace keyless `cargo test` re-verified at cd67814:
  806 passed / 0 failed across 21 test binaries/sections (doc-tests
  included), matching the recorded baseline exactly.
- SCIP: index.scip regenerated with the estate route,
  `rust-analyzer scip . --output index.scip` (rust-analyzer
  1 (9074e9b4c6 2026-09-06), ~31s, exit 0; one stderr note:
  "Duplicate symbol: rust-analyzer cargo light-show 0.1.0 crate/").
  Size 3,615,969 -> 7,387,284 bytes; document coverage 49 -> 82 .rs
  paths, a strict superset of the previous index (nothing dropped).
- Publish: added scripts/publish/keystore-dry-run.sh (print-only,
  Tiger Style bash; shellcheck zero, shfmt clean). It creates
  $XDG_DATA_HOME/keystores/light-show/ (mode 700) and prints the exact
  keytool commands for the release and Play upload keystores without
  executing them; --live and extra args are refused (exit 2).
  tbr-keystore.sh was not run (Matt-only, as designed).
