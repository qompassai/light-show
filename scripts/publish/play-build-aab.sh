#!/usr/bin/env bash
# play-build-aab.sh -- Build the light-show release AAB for Google Play.
#
# Contract:
#   Inputs:  --dry-run (default) | --live ; --skip-native (reuse existing jniLibs)
#   Work:    cargo-ndk builds the cdylib per ABI, then Gradle bundleRelease.
#   Output:  path and size of the produced AAB on stdout (data).
#   State:   writes .so files to android/app/src/main/jniLibs and the AAB
#            under android/app/build/outputs/bundle/release/ (--live only).
#   Exit:    0 success, 1 build/prereq failure, 2 usage error.
#   Safety:  dry-run performs zero builds; --live never signs or uploads.
#
# Usage:
#   scripts/publish/play-build-aab.sh --dry-run   # check prereqs, print plan
#   scripts/publish/play-build-aab.sh --live       # actually build the AAB

set -euo pipefail

# --- constants ---------------------------------------------------------------
readonly REPO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
readonly GAME_DIR="$REPO_DIR/game"
readonly ANDROID_DIR="$REPO_DIR/android"
readonly JNILIBS_DIR="$ANDROID_DIR/app/src/main/jniLibs"
readonly AAB_PATH="$ANDROID_DIR/app/build/outputs/bundle/release/app-release.aab"
readonly ABIS="arm64-v8a armeabi-v7a x86_64"
readonly NDK_VERSION="28.0.12674087"
readonly BUILD_TIMEOUT_S=1800  # 30 min bound on the native build

# --- helpers -----------------------------------------------------------------
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
usage() { printf 'Usage: %s [--dry-run|--live] [--skip-native]\n' "$0" >&2; exit 2; }

have() { command -v "$1" >/dev/null 2>&1; }

check_prereqs() {
    local missing=0
    for tool in cargo rustc; do
        if ! have "$tool"; then
            printf 'MISSING: %s not on PATH\n' "$tool" >&2
            missing=1
        fi
    done
    if ! cargo ndk --version >/dev/null 2>&1; then
        printf 'MISSING: cargo-ndk (cargo install cargo-ndk)\n' >&2
        missing=1
    fi
    for target in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
        if ! rustup target list --installed 2>/dev/null | grep -qx "$target"; then
            printf 'MISSING: rust target %s (rustup target add %s)\n' "$target" "$target" >&2
            missing=1
        fi
    done
    if [[ ! -x "$ANDROID_DIR/gradlew" ]]; then
        printf 'MISSING: %s/gradlew not executable\n' "$ANDROID_DIR" >&2
        missing=1
    fi
    # Non-interactive shells may not source /etc/profile.d/android-sdk.sh;
    # probe the well-known location before failing.
    if [[ -z "${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}" ]]; then
        for candidate in /opt/android-sdk "$HOME/Android/Sdk"; do
            if [[ -d "$candidate/platform-tools" ]]; then
                export ANDROID_HOME="$candidate" ANDROID_SDK_ROOT="$candidate"
                printf 'INFO: ANDROID_HOME probed as %s\n' "$candidate" >&2
                break
            fi
        done
    fi
    if [[ -z "${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}" ]]; then
        printf 'MISSING: ANDROID_HOME/ANDROID_SDK_ROOT not set\n' >&2
        missing=1
    fi
    return "$missing"
}

build_native() {
    printf 'Building native cdylib per ABI: %s\n' "$ABIS" >&2
    local ndk_args=()
    local abi
    for abi in $ABIS; do
        ndk_args+=(-t "$abi")
    done
    (
        cd "$GAME_DIR"
        timeout "$BUILD_TIMEOUT_S" cargo ndk \
            "${ndk_args[@]}" \
            -o "$JNILIBS_DIR" \
            build --release -p light-show
    )
}

build_aab() {
    printf 'Running Gradle bundleRelease\n' >&2
    (cd "$ANDROID_DIR" && ./gradlew bundleRelease --console=plain)
}

report_aab() {
    if [[ ! -f "$AAB_PATH" ]]; then
        die "AAB not found at $AAB_PATH after build"
    fi
    local size
    size=$(stat -c%s "$AAB_PATH")
    printf 'AAB: %s\n' "$AAB_PATH"
    printf 'SIZE_BYTES: %s\n' "$size"
}

# --- main --------------------------------------------------------------------
main() {
    local mode="dry-run" skip_native=0
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --dry-run) mode="dry-run"; shift ;;
            --live) mode="live"; shift ;;
            --skip-native) skip_native=1; shift ;;
            -h|--help) usage ;;
            *) usage ;;
        esac
    done

    printf '=== play-build-aab (%s) ===\n' "$mode" >&2
    if ! check_prereqs; then
        die "prerequisites missing (see above)"
    fi
    printf 'Prereqs OK.\n' >&2

    if [[ "$mode" == "dry-run" ]]; then
        printf 'DRY RUN — would execute:\n' >&2
        if [[ "$skip_native" -eq 0 ]]; then
            printf '  1. cargo ndk -t %s -o <jniLibs> build --release -p light-show\n' \
                "${ABIS// / -t }" >&2
        else
            printf '  1. (skip native: reusing %s)\n' "$JNILIBS_DIR" >&2
        fi
        printf '  2. ./gradlew bundleRelease\n' >&2
        printf '  3. report %s\n' "$AAB_PATH" >&2
        if [[ -f "$AAB_PATH" ]]; then
            printf '\nExisting AAB found:\n' >&2
            report_aab
        else
            printf '\nNo existing AAB at %s\n' "$AAB_PATH" >&2
        fi
        return 0
    fi

    # --live
    if [[ "$skip_native" -eq 0 ]]; then
        build_native
    else
        [[ -d "$JNILIBS_DIR" ]] || die "--skip-native but $JNILIBS_DIR missing"
        printf 'Reusing existing jniLibs.\n' >&2
    fi
    build_aab
    report_aab
    printf '\nNOTE: this AAB is UNSIGNED. Signing is Matt-only:\n' >&2
    printf '  scripts/publish/tbr-keystore.sh  (creates the upload key)\n' >&2
}

main "$@"
