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
        };
      });
}
