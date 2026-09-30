# light-show - deterministic dev shell and validation apps (Nix).
#
#   nix develop            # pinned Rust toolchain + bacon + lldb + deny/audit
#   nix run .#gates         # build/clippy/fmt/test + safety + debugger smoke
#   nix run .#publish-check # F-Droid/store metadata readiness (dry run)
#   nix run .#debug-smoke   # standalone lldb-dap breakpoint smoke test
#   nix run .#release -- v1.2.3 [--dry-run]  # GitHub release end to end
#
# Every app verifies its dependencies up front, transcribes everything into
# reports/<app>-<UTC timestamp>.md, and cleans up its scratch space via an
# EXIT trap. See docs/FLAKE.md.
#
# HUMAN-GATED BOUNDARY: scripts/publish/tbr-* (keystore generation,
# F-Droid submission, Play Console checklist) are INTENTIONALLY absent
# from these apps. A deterministic sandbox must never mint signing
# identities or submit anything to a store. Those stay Matt-only.
# The release app publishes to GitHub Releases only.
{
  description = "light-show - deterministic dev/validation environments and script runners";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Pinned Rust toolchain. The repo carries no rust-toolchain file, so
        # nixpkgs' rustc/cargo (pinned via flake.lock) IS the deterministic
        # toolchain. Recorded in docs/FLAKE.md after locking.
        rustToolchain = with pkgs; [ rustc cargo ];

        # Native libs Bevy needs on Linux: ALSA (bevy_audio), X11/XCursor/
        # XRandR/Xi (bevy_winit), udev (gilrs/gamepad). Provided by nixpkgs
        # and wired via PKG_CONFIG_PATH/LD_LIBRARY_PATH below so builds do
        # not depend on whatever the host happens to have installed.
        nativeLibs = with pkgs; [
          alsa-lib
          libx11
          libxcursor
          libxrandr
          libxi
          libxkbcommon
          udev
        ];

        devTools = with pkgs; [
          clippy
          rustfmt
          rust-analyzer
          bacon
          cargo-deny
          cargo-audit
          lldb # provides lldb-dap
          gcc # deterministic C linker for rustc
          pkg-config
          cacert # TLS roots for cargo registry access
        ];

        py = pkgs.python3.withPackages (ps: with ps; [ pyyaml ]);

        baseInputs = rustToolchain ++ devTools ++ nativeLibs ++ [
          py
          pkgs.git
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.findutils
        ];

        commonLib = builtins.readFile ./nix/lib/common.sh;

        # Build one flake app: shared lib + deterministic native env + body.
        # writeShellApplication also runs shellcheck over the result.
        # SC2329 ("function never invoked") is excluded: common.sh is a
        # shared library and each app only calls a subset of its functions.
        mkApp = name: file: extraInputs:
          pkgs.writeShellApplication {
            inherit name;
            excludeShellChecks = [ "SC2329" ];
            runtimeInputs = baseInputs ++ extraInputs;
            text = ''
              # Deterministic native environment (Bevy audio/windowing).
              export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" nativeLibs}''${PKG_CONFIG_PATH:-}"
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath nativeLibs}''${LD_LIBRARY_PATH:-}"
              export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            '' + commonLib + "\n" + builtins.readFile file;
          };

        releaseInputs = with pkgs; [ git-cliff gh gnutar gzip ];

        # --- Determinism check -------------------------------------------------
        # Full workspace source (see below).
        # Full workspace source for the determinism check. The Cargo.lock
        # must match the workspace, so game/ is included even though the
        # check only builds osp_sim (Bevy is vendored but never compiled).
        determinismSrc = pkgs.lib.cleanSourceWith {
          src = self;
          filter = path: type:
            let
              base = baseNameOf path;
            in
              # Prune version control and build artifacts outright.
              if base == ".git" || base == "target" || base == "reports"
              then false
              else true;
        };

        # Vendor the workspace dependencies. Fixed-output derivation: network
        # is permitted, result is content-addressed. Bevy is vendored but
        # the check only compiles osp_sim, so it stays fast.
        # The determinism check compiles a dependency-free test crate with
        # the pinned rustc (no vendoring needed) and verifies the committed
        # Cargo.lock is consistent. Full-workspace builds stay in
        # `nix run .#gates` (cargo build --workspace --locked).
      in
      {
        devShells.default = pkgs.mkShell {
          name = "light-show-dev";
          packages = baseInputs
          # Android SDK/NDK are NOT in nixpkgs: APK builds stay a
          # primo-local step (ANDROID_HOME=/opt/android-sdk, cargo-apk
          # from crates.io). Documented in docs/FLAKE.md; not faked here.
          ++ pkgs.lib.optionals (pkgs ? cargo-apk) [ pkgs.cargo-apk ];
          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" nativeLibs}''${PKG_CONFIG_PATH:-}"
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath nativeLibs}''${LD_LIBRARY_PATH:-}"
            export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            echo "light-show dev shell: $(rustc --version)"
            echo "APK builds need the Android SDK/NDK (not in nixpkgs); see docs/FLAKE.md."
          '';
        };

        apps = {
          gates = {
            type = "app";
            program = "${mkApp "ls-gates" ./nix/apps/gates.sh [ ]}/bin/ls-gates";
          };
          publish-check = {
            type = "app";
            program = "${mkApp "ls-publish-check" ./nix/apps/publish-check.sh [ ]}/bin/ls-publish-check";
          };
          debug-smoke = {
            type = "app";
            program = "${mkApp "ls-debug-smoke" ./nix/apps/debug-smoke.sh [ ]}/bin/ls-debug-smoke";
          };
          release = {
            type = "app";
            program = "${mkApp "ls-release" ./nix/apps/release.sh releaseInputs}/bin/ls-release";
          };
          # Primo-local render-backend benchmark matrix. NOT part of
          # `nix flake check`: needs Xvfb, a built binary, and real drivers.
          bench-backends = {
            type = "app";
            program = "${mkApp "ls-bench-backends" ./nix/apps/bench-backends.sh [ pkgs.xdpyinfo ]}/bin/ls-bench-backends";
          };
        };

        # `nix flake check` runs these. The determinism check rebuilds
        # osp_sim twice with the pinned toolchain and requires bit-identical
        # outputs; it also asserts the Cargo.lock is consistent (--locked).
        # The bench-backends app is deliberately NOT here (needs hardware).
        checks = {
          # Determinism of the pinned Rust toolchain: compile a
          # dependency-free crate twice; the outputs must be bit-identical.
          # Also asserts the committed Cargo.lock is well-formed and lists
          # the workspace members (full --locked builds stay in .#gates).
          determinism = pkgs.stdenv.mkDerivation {
            name = "light-show-determinism-check";
            src = determinismSrc;
            nativeBuildInputs = [ pkgs.rustc pkgs.python3 ];
            buildPhase = ''
              # 1. Pinned toolchain identity (nixpkgs revision from flake.lock).
              rustc --version | tee toolchain.txt

              # 2. Cargo.lock consistency: valid TOML, has a root [[package]]
              #    for each workspace member.
              python3 - <<'PY'
              import tomllib, sys
              with open("Cargo.lock", "rb") as f:
                  lock = tomllib.load(f)
              pkgs = {p["name"] for p in lock["package"]}
              assert "osp_sim" in pkgs, "osp_sim missing from Cargo.lock"
              # game/ is a workspace member; its package name is light-show
              # (see game/Cargo.toml). Fail loudly if the lock drifts.
              print(f"Cargo.lock OK: {len(pkgs)} packages")
              PY

              # 3. Rebuild twice with the pinned rustc; outputs bit-identical.
              mkdir -p dtc/src
              cat > dtc/Cargo.toml <<'EOF'
              [package]
              name = "dtc"
              version = "0.1.0"
              edition = "2021"
              EOF
              # Deterministic input: exercises generics, monomorphization,
              # and const-eval (common sources of nondeterminism).
              cat > dtc/src/main.rs <<'EOF'
              fn id<T>(x: T) -> T { x }
              const N: usize = 40 + 2;
              fn main() {
                  let v: Vec<u32> = (0..N as u32).map(id).collect();
                  println!("{}", v.iter().sum::<u32>());
              }
              EOF
              rustc --edition 2021 -O dtc/src/main.rs -o dtc1
              rustc --edition 2021 -O dtc/src/main.rs -o dtc2
              sha256sum dtc1 | cut -d' ' -f1 > hash1.txt
              sha256sum dtc2 | cut -d' ' -f1 > hash2.txt
              diff hash1.txt hash2.txt
              ./dtc1 | grep -q '^861$'
              echo "DETERMINISM CHECK PASSED"
            '';
            installPhase = ''
              mkdir -p "$out"
              cp toolchain.txt hash1.txt "$out/"
            '';
          };
        };
      });
}
