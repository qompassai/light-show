# Integration tests: structural + README/asset images (tests crate)

Agent: claude-opus-5-5 (test-building worker). Date: 2026-10-05.

## Discoveries

- `cargo fmt -p light-show-tests` rewrites EVERY file in tests/, not just
  one; with parallel workers, use `cargo fmt --check -p light-show-tests`
  (read-only) and fix your own files by hand. It also errors (but keeps
  going) when a `[[test]]` path in tests/Cargo.toml does not exist yet.
- `cargo clippy -p light-show-tests ... -- -D warnings` also lints the
  `light-show` game lib (workspace path dep), which already has 3 clippy
  errors: too_many_arguments at game/src/states/outage.rs:310 and
  game/src/states/playing.rs:277, type_complexity at
  game/src/states/results.rs:135. Use `--no-deps` to gate only the tests crate.
- `LICENSE` is a 25-line GPL-3.0 notice that points to gnu.org, not the
  full license text. Workspace license = GPL-3.0-or-later.
- The README gameplay GIFs are 654 and 477 frames (360x640). Fully
  decoding them takes about 22 s in a debug build.

## Evidence / validation

    cargo test --test assets_images   # 18 passed
    cargo test --test structural      # 19 passed
    cargo clippy -q --no-deps -p light-show-tests --test assets_images \
        --test structural -- -D warnings   # clean

Affected paths: tests/assets_images.rs, tests/structural.rs.
