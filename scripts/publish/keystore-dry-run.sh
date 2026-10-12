#!/usr/bin/env bash
# keystore-dry-run.sh -- print (never execute) the keytool commands that mint
# light-show's signing keystores, and prepare their directory.
#
# Contract:
#   Inputs:  --dry-run, or no arguments (the script has no other mode).
#            Any other argument (including --execute or --live) is a usage
#            error, exit 2. Minting real keys is Matt-only: the Play upload
#            key is created by scripts/publish/tbr-keystore.sh, and the
#            release key uses the command printed here, run by hand.
#   Work:    creates $XDG_DATA_HOME/keystores/light-show/ (mode 700) when
#            absent, then prints the exact keytool command lines for both
#            keystores. No password ever appears in the printed commands:
#            with -storepass/-keypass omitted, keytool prompts for them, so
#            secrets stay out of argv, the environment, and shell history
#            (same discipline as tbr-keystore.sh).
#   Output:  keystore directory and the two command lines on stdout;
#            diagnostics on stderr.
#   Exit:    0 = dry-run plan printed; 1 = environment error (no keytool
#            found, or neither XDG_DATA_HOME nor HOME set); 2 = usage error.
#
# Usage:
#   scripts/publish/keystore-dry-run.sh [--dry-run]
#
# Side effects: creates the keystores directory only. No keystore file is
# created, read, or modified. No network access. Bounded work: two fixed
# command lines are printed; there are no loops over external data.

set -euo pipefail

if ((BASH_VERSINFO[0] < 4)); then
    printf 'ERROR: %s requires bash >= 4 (found %s)\n' "$0" "$BASH_VERSION" >&2
    exit 1
fi

# --- constants ---------------------------------------------------------------
readonly KEY_ALG="RSA"
readonly KEY_SIZE="2048"
readonly VALIDITY_DAYS="10950" # 30 years; Play requires validity past 2033-10-22
readonly DNAME="CN=Qompass AI, OU=Qompass AI, O=Qompass AI, C=US"
readonly APP_SLUG="light-show"
readonly RELEASE_ALIAS="lightshow-release"
readonly RELEASE_FILE="release.jks"
readonly UPLOAD_ALIAS="lightshow-upload"
readonly UPLOAD_FILE="upload.jks"
readonly JDK17_KEYTOOL="/usr/lib/jvm/java-17-openjdk/bin/keytool"

# --- helpers -----------------------------------------------------------------
die() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

usage() {
    printf 'Usage: %s [--dry-run]\n' "$0" >&2
    printf '%s\n' 'Print-only: this script never runs keytool. Real keys are Matt-only.' >&2
    exit 2
}

# Resolve the keytool binary the printed commands should name: the pinned
# JDK 17 path (same as tbr-keystore.sh) wins, then PATH.
resolve_keytool() {
    if [[ -x "$JDK17_KEYTOOL" ]]; then
        printf '%s\n' "$JDK17_KEYTOOL"
        return 0
    fi
    local found=""
    found="$(command -v keytool || true)"
    [[ -n "$found" ]] || die "keytool not found (looked for $JDK17_KEYTOOL and PATH)"
    printf '%s\n' "$found"
}

# The XDG data root: XDG_DATA_HOME when set, else the spec default.
data_home() {
    if [[ -n "${XDG_DATA_HOME:-}" ]]; then
        printf '%s\n' "$XDG_DATA_HOME"
        return 0
    fi
    [[ -n "${HOME:-}" ]] || die "neither XDG_DATA_HOME nor HOME is set"
    printf '%s\n' "$HOME/.local/share"
}

# Print one keystore plan: a comment header, the keytool command exactly as
# Matt would run it, and the permission tightening that follows creation.
print_keystore_plan() {
    local keytool="$1"
    local label="$2"
    local keystore_path="$3"
    local alias="$4"
    printf '# %s\n' "$label"
    printf '%s -genkeypair -v -keystore %s -alias %s -keyalg %s -keysize %s -validity %s -dname "%s"\n' \
        "$keytool" "$keystore_path" "$alias" "$KEY_ALG" "$KEY_SIZE" "$VALIDITY_DAYS" "$DNAME"
    printf 'chmod 600 %s\n' "$keystore_path"
}

main() {
    if (($# > 1)); then
        usage
    fi
    if (($# == 1)) && [[ "$1" != "--dry-run" ]]; then
        usage
    fi

    local keytool=""
    keytool="$(resolve_keytool)"
    local root=""
    root="$(data_home)"
    local keystore_dir="$root/keystores/$APP_SLUG"

    mkdir -p -- "$keystore_dir"
    chmod 700 "$keystore_dir"

    printf 'dry-run: no keytool command will be executed by this script\n'
    printf 'keystore directory: %s\n' "$keystore_dir"
    printf 'keytool: %s\n' "$keytool"
    printf '\n'
    print_keystore_plan "$keytool" \
        "Release keystore (F-Droid / direct-distribution signing)" \
        "$keystore_dir/$RELEASE_FILE" "$RELEASE_ALIAS"
    printf '\n'
    print_keystore_plan "$keytool" \
        "Play upload keystore (same key tbr-keystore.sh mints; it honors LIGHTSHOW_KEYSTORE_OUT)" \
        "$keystore_dir/$UPLOAD_FILE" "$UPLOAD_ALIAS"
    printf '\n'
    printf '%s\n' 'After minting by hand: back the keystores up OFFLINE, then store the passwords in pass'
    printf '%s\n' '  (qompassai/light-show/upload-keystore-password, qompassai/light-show/upload-key-password,'
    printf '%s\n' '   qompassai/light-show/release-keystore-password, qompassai/light-show/release-key-password).'
    printf '%s\n' 'Never commit a keystore or a password to git.'
}

main "$@"
