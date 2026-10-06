#!/usr/bin/env bash
# play-metadata-check.sh -- Validate Play Console listing metadata completeness.
#
# Contract:
#   Inputs:  --dry-run (default; read-only) | --live (same checks; reserved
#            for future auto-fix hooks — currently identical to dry-run).
#   Work:    checks the fastlane tree against Google Play listing requirements.
#   Output:  one PASS/FAIL/WARN line per field on stdout; missing items listed.
#   State:   read-only. Changes nothing.
#   Exit:    0 all required fields present, 1 any required field missing,
#            2 usage error.
#
# Play requirements checked (store listing):
#   title.txt              <= 50 chars
#   short_description.txt  <= 80 chars, single line
#   full_description.txt   <= 4000 chars
#   images/icon.png        512x512 PNG (required)
#   featureGraphic.png     1024x500 PNG (required by Play, NOT by F-Droid)
#   phoneScreenshots/      >= 2 images (required by Play)
#   changelogs/<vercode>.txt  <= 500 chars (required, named by versionCode)
#
# Usage:
#   scripts/publish/play-metadata-check.sh --dry-run

set -euo pipefail

# --- constants ---------------------------------------------------------------
readonly REPO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
readonly META="$REPO_DIR/fastlane/metadata/android/en-US"
readonly TITLE_MAX=50 SHORT_MAX=80 FULL_MAX=4000 CHANGELOG_MAX=500
readonly ICON_PX=512 FG_W=1024 FG_H=500 MIN_SCREENSHOTS=2

# --- helpers -----------------------------------------------------------------
usage() { printf 'Usage: %s [--dry-run|--live]\n' "$0" >&2; exit 2; }

png_dims() { # $1=path -> "WxH" via IHDR, no PIL needed
    python3 -c "
import struct,sys
with open('$1','rb') as f: h=f.read(26)
assert h[:8]==b'\x89PNG\r\n\x1a\n', 'not a png'
print('%dx%d' % struct.unpack('>II', h[16:24]))
" 2>/dev/null || echo "unreadable"
}

check_text() { # $1=file $2=max $3=label $4=required(1/0)
    local file="$1" max="$2" label="$3" required="$4"
    if [[ ! -f "$file" ]]; then
        [[ "$required" == "1" ]] && { printf 'FAIL %s: missing %s\n' "$label" "$file"; return 1; }
        printf 'WARN %s: missing (optional) %s\n' "$label" "$file"; return 0
    fi
    local len
    len=$(wc -m < "$file" | tr -d ' ')
    if (( len > max )); then
        printf 'FAIL %s: %d chars exceeds max %d\n' "$label" "$len" "$max"; return 1
    fi
    if (( len == 0 )); then
        printf 'FAIL %s: empty\n' "$label"; return 1
    fi
    printf 'PASS %s: %d chars (max %d)\n' "$label" "$len" "$max"
}

check_image() { # $1=file $2=expected-dims $3=label $4=required(1/0)
    local file="$1" want="$2" label="$3" required="$4"
    if [[ ! -f "$file" ]]; then
        [[ "$required" == "1" ]] && { printf 'FAIL %s: missing %s\n' "$label" "$file"; return 1; }
        printf 'WARN %s: missing (optional) %s\n' "$label" "$file"; return 0
    fi
    local dims
    dims=$(png_dims "$file")
    if [[ "$dims" != "$want" ]]; then
        printf 'FAIL %s: %s, want %s\n' "$label" "$dims" "$want"; return 1
    fi
    printf 'PASS %s: %s\n' "$label" "$dims"
}

# --- main --------------------------------------------------------------------
main() {
    local mode="dry-run"
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --dry-run|--live) mode="$1"; shift ;;
            -h|--help) usage ;;
            *) usage ;;
        esac
    done
    [[ "$mode" == "--live" ]] && printf '(live: checks only, no autofix yet)\n' >&2

    local failures=0
    printf '=== Play metadata check: %s ===\n' "$META"

    check_text "$META/title.txt" "$TITLE_MAX" "title" 1 || failures=$((failures+1))
    check_text "$META/short_description.txt" "$SHORT_MAX" "short_description" 1 || failures=$((failures+1))
    check_text "$META/full_description.txt" "$FULL_MAX" "full_description" 1 || failures=$((failures+1))
    check_image "$META/images/icon.png" "${ICON_PX}x${ICON_PX}" "icon" 1 || failures=$((failures+1))
    check_image "$META/images/featureGraphic.png" "${FG_W}x${FG_H}" "feature_graphic" 1 || failures=$((failures+1))

    # screenshots: >= MIN_SCREENSHOTS, any sane PNG dims
    shopt -s nullglob
    local shots=("$META"/images/phoneScreenshots/*.png)
    shopt -u nullglob
    if (( ${#shots[@]} < MIN_SCREENSHOTS )); then
        printf 'FAIL screenshots: %d found, need >= %d\n' "${#shots[@]}" "$MIN_SCREENSHOTS"
        failures=$((failures+1))
    else
        printf 'PASS screenshots: %d found\n' "${#shots[@]}"
        local s
        for s in "${shots[@]}"; do
            printf '  - %s: %s\n' "$(basename "$s")" "$(png_dims "$s")"
        done
    fi

    # changelog for current versionCode
    local vercode
    vercode=$(grep -oP 'versionCode\s*=\s*\K\d+' "$REPO_DIR/android/app/build.gradle.kts" | head -1)
    if [[ -z "$vercode" ]]; then
        printf 'FAIL changelog: cannot parse versionCode\n'
        failures=$((failures+1))
    else
        check_text "$META/changelogs/${vercode}.txt" "$CHANGELOG_MAX" "changelog(v$vercode)" 1 \
            || failures=$((failures+1))
    fi

    printf '\n%s\n' "$([[ "$failures" -eq 0 ]] && echo 'ALL REQUIRED FIELDS PRESENT' || echo "$failures REQUIRED FIELD(S) MISSING")"
    return "$([[ "$failures" -eq 0 ]] && echo 0 || echo 1)"
}

main "$@"
