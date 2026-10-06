#!/usr/bin/env bash
# play-upload-dryrun.sh -- Exercise the Play Console upload path WITHOUT uploading.
#
# Contract:
#   Inputs:  --dry-run (default and ONLY mode for now). --live is REFUSED:
#            the real upload needs Matt's Play Console service-account key
#            and his signed AAB; agents never touch either.
#   Work:    validates the AAB exists, is a well-formed zip, is signed
#            (reports signer cert), and shows exactly what WOULD be uploaded
#            (package, versionCode, versionName, size, track).
#   Output:  validation report on stdout; the would-be upload plan.
#   State:   read-only. ZERO network calls. Exits before any upload API.
#   Exit:    0 dry-run validation passed, 1 AAB invalid/missing,
#            2 usage error (includes --live).
#
# Usage:
#   scripts/publish/play-upload-dryrun.sh --dry-run

set -euo pipefail

# --- constants ---------------------------------------------------------------
readonly REPO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
readonly AAB_PATH="$REPO_DIR/android/app/build/outputs/bundle/release/app-release.aab"
readonly SIGNED_AAB="$REPO_DIR/android/app/build/outputs/bundle/release/app-release-signed.aab"
readonly APPID="ai.qompass.lightshow"
readonly TRACK="internal"  # first upload should always go to the internal track

# --- helpers -----------------------------------------------------------------
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
usage() { printf 'Usage: %s --dry-run\n  (--live is refused: upload is Matt-only)\n' "$0" >&2; exit 2; }
have() { command -v "$1" >/dev/null 2>&1; }

# --- main --------------------------------------------------------------------
main() {
    [[ "${1:-}" == "--dry-run" ]] || usage
    printf '=== play-upload dry run (NO network, NO upload) ===\n'

    # 1. pick the AAB: signed if Matt has produced it, else the unsigned build
    local aab=""
    if [[ -f "$SIGNED_AAB" ]]; then
        aab="$SIGNED_AAB"
        printf 'AAB: %s (SIGNED)\n' "$aab"
    elif [[ -f "$AAB_PATH" ]]; then
        aab="$AAB_PATH"
        printf 'AAB: %s (UNSIGNED — Matt must sign before real upload)\n' "$aab"
    else
        die "no AAB found; run scripts/publish/play-build-aab.sh --live first"
    fi

    # 2. well-formed zip?
    have unzip || die "unzip not on PATH"
    unzip -tq "$aab" >/dev/null || die "AAB is not a valid zip: $aab"
    printf 'PASS: AAB is a well-formed zip\n'

    # 3. size + contents sanity (capture listing first: grep -q closes the
    # pipe early, and with pipefail the SIGPIPE on unzip would fail the check)
    local size listing
    size=$(stat -c%s "$aab")
    printf 'SIZE_BYTES: %s\n' "$size"
    listing=$(unzip -l "$aab")
    for entry in "base/manifest/AndroidManifest.xml" "BUNDLE-METADATA"; do
        grep -q "$entry" <<< "$listing" \
            || die "AAB missing expected entry: $entry"
    done
    printf 'PASS: AAB contains AndroidManifest.xml + BUNDLE-METADATA\n'

    # 4. signature state (informational — upload REQUIRES a signed AAB)
    if have apksigner; then
        if apksigner verify --print-certs "$aab" >/dev/null 2>&1; then
            printf 'PASS: AAB signature verifies\n'
            apksigner verify --print-certs "$aab" 2>/dev/null \
                | grep -m1 "Signer #1" || true
        else
            printf 'WARN: AAB is NOT signed (expected pre-Matt-signing)\n'
        fi
    else
        printf 'WARN: apksigner not on PATH; skipping signature check\n'
    fi

    # 5. version coherence with build.gradle.kts
    local gradle_vc gradle_vn
    gradle_vc=$(grep -oP 'versionCode\s*=\s*\K\d+' "$REPO_DIR/android/app/build.gradle.kts" | head -1)
    gradle_vn=$(grep -oP 'versionName\s*=\s*"\K[^"]+' "$REPO_DIR/android/app/build.gradle.kts" | head -1)
    printf 'PACKAGE: %s\nVERSION_CODE: %s\nVERSION_NAME: %s\n' "$APPID" "$gradle_vc" "$gradle_vn"

    # 6. the would-be upload plan (NOT executed)
    printf '%s\n' '' '--- WOULD-BE UPLOAD (not executed) ---'
    printf '  file:         %s\n' "$aab"
    printf '  package:      %s\n' "$APPID"
    printf '  track:        %s\n' "$TRACK"
    printf '  release name: %s (%s)\n' "$gradle_vn" "$gradle_vc"
    printf '  auth:         Play Console service-account JSON (Matt-only)\n'
    printf '  precondition: AAB must be SIGNED with the upload key\n'
    printf '%s\n' '--------------------------------------'
    printf 'DRY RUN COMPLETE. No network calls were made.\n'
    printf "Matt's single command when ready:\n"
    printf '  1. scripts/publish/tbr-keystore.sh        # one-time: create upload key\n'
    printf '  2. sign the AAB (see docs/BUILD.md §Option B step 4)\n'
    printf '  3. upload %s to the %s track in Play Console\n' "$(basename "$aab")" "$TRACK"
}

main "$@"
