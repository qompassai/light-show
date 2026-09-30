# Deterministic builds & validation with Nix

This repo ships a Nix flake that pins the entire dev/validation toolchain
and turns the deterministic-safe scripts into `nix run` apps. Same inputs,
same outputs, on any machine with Nix.

```sh
nix develop            # pinned Rust toolchain + bacon + lldb + deny/audit
nix run .#gates         # build / clippy / fmt / test + safety + debugger smoke
nix run .#publish-check # F-Droid/store metadata readiness (dry run)
nix run .#debug-smoke   # standalone lldb-dap breakpoint smoke test
nix run .#release -- v1.2.3 [--dry-run]  # cut a GitHub release end to end
```

## Pinned toolchain

`flake.lock` pins `nixpkgs/nixos-26.05` (rev
`7fc6f2c20af09cdcaf48b92ec3121860139ec668`, 2026-09-28). The repo carries
no `rust-toolchain` file, so the pinned nixpkgs compiler **is** the
deterministic toolchain:

- **rustc / cargo 1.95.0** (primo's system toolchain is 1.96.0; the flake
  deliberately pins 1.95.0 via nixpkgs)
- **lldb 21.1.8** (provides `lldb-dap`)
- **cargo-deny**, **cargo-audit**, **bacon**, **git-cliff**, **gh**

Bevy's native Linux deps (ALSA, X11/XCursor/XRandR/Xi, xkbcommon, udev)
come from nixpkgs too, wired via `PKG_CONFIG_PATH`/`LD_LIBRARY_PATH` in
every app and in `nix develop` — builds never depend on whatever the host
happens to have installed.

## Apps

Every app follows the same contract (`nix/lib/common.sh`):

1. **Declares all deps up front** (`ls_check_deps`) — a missing tool fails
   fast and names the exact binary. Nothing is installed at runtime.
2. **Writes a markdown report** to `reports/<app>-<UTC timestamp>.md`
   (gitignored, never committed).
3. **Runs safety checks**: `cargo deny check`, `cargo audit` when available.
4. **Runs the debugger smoke test**: drives `lldb-dap` over DAP against
   the `osp_sim` unit-test binary, sets a breakpoint on
   `osp_sim::ReceiveWindow::margin` (source-line fallback), launches the
   `tests::window_contains_and_margin` test, and confirms the breakpoint
   is actually hit with the function on the stack. Verdict is PASS / FAIL /
   SKIP — never faked. (Quirks handled: lldb-dap sends `initialized`
   after `launch`, needs `configurationDone` before the debuggee runs,
   and reports `verified=false` until the module loads.)
5. **Cleans up after itself**: scratch dirs via an `EXIT` trap; the
   worktree is verified to have no *new* dirty paths (pre-existing
   uncommitted work never trips the check).

### `nix run .#gates`

Full validation gate: `cargo build --locked`, `cargo clippy -- -D warnings`,
`cargo fmt --check`, `cargo test --locked`, then safety checks and the
debugger smoke test. First run compiles Bevy 0.14 from scratch (15–30 min);
subsequent runs are incremental.

### `nix run .#publish-check`

Runs `scripts/publish/bin/fdroid-publish-check` (stdlib-only Python)
against the repo itself: license, fastlane metadata, git tags, non-free
markers, version literals. Dry run only — it never submits anything.

### `nix run .#debug-smoke`

Just the lldb-dap breakpoint smoke test, standalone. Useful when iterating
on the debugger wiring without running the full gate.

### `nix run .#release -- <version> [--dry-run]`

Cuts a GitHub release end to end:

1. **Validates** the version: required argument, `vMAJOR.MINOR.PATCH`
   semver, and the tag must not already exist. No default, no accidents.
2. **Builds** the release artifacts with the pinned toolchain:
   `cargo build --release -p light-show --locked`, packaged as
   `dist/light-show-<version>-x86_64-unknown-linux-gnu.tar.gz` (binary +
   LICENSE + README, sha256 recorded in the report). Android APK/AAB is
   attempted only if `cargo-apk`, `ANDROID_HOME`/`ANDROID_NDK_HOME`, and
   android target stds are all present — they are not in nixpkgs, so on a
   pure flake setup this step reports SKIP (APK builds stay primo-local).
3. **Generates the changelog** from git history (`<prev-tag>..HEAD`) with
   `git-cliff` (`nix/lib/cliff.toml`, conventional commits), falling back
   to a `git log` summary when cliff yields nothing usable. Never
   hand-written.
4. **Creates and pushes the annotated tag**, then `gh release create
   <version> --title ... --notes-file ... dist/*`, uploading the artifacts.
   Uses ambient `gh` auth only — the app fails fast if `gh` is not
   authenticated and never accepts tokens via args or env.
5. `--dry-run` does everything except tag creation, tag push, and
   `gh release create`. After a successful upload the local `dist/` copies
   are removed (they live on GitHub); on dry-run/failure they are kept
   for inspection.

## What stays OUT (human-gated)

`scripts/publish/tbr-*` (keystore generation, F-Droid submission, Play
Console checklist) are intentionally absent from every app. A deterministic
sandbox must never mint signing identities or submit anything to a store.
The release app publishes to **GitHub Releases only**. Those steps are
Matt's, documented in `scripts/publish/`.

## Android gap (documented, not faked)

The Android SDK/NDK are not in nixpkgs, and the pinned rustc ships no
Android target stds. APK/AAB builds therefore stay a primo-local step
(`ANDROID_HOME=/opt/android-sdk`, `cargo-apk`); see `docs/BUILD.md`.
The flake does not pretend otherwise — the release app reports SKIP with
the reason instead of failing or faking it.
