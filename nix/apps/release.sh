# ls-release: cut a GitHub release end to end.
#
# Usage: nix run .#release -- <version> [--dry-run]
#   <version>  REQUIRED, e.g. v1.2.3 - validated vMAJOR.MINOR.PATCH semver,
#              and the tag must not already exist. No default, no accidents.
#   --dry-run  build artifacts + generate notes, but skip tag creation,
#              tag push, and `gh release create`.
#
# Flow:
#   1. validate version arg + tag-not-exists
#   2. build the release artifacts deterministically (pinned flake toolchain)
#   3. generate changelog notes (git-cliff, git-log fallback - never hand-written)
#   4. create + push the annotated tag
#   5. `gh release create` with the artifacts as release assets
#
# GitHub Releases ONLY. The tbr-* scripts, signing keys, Play Console and
# F-Droid submissions stay OUT by design - a release app must never mint
# signing identities or submit to a store.

DRY_RUN=0
VERSION=""
for arg in "$@"; do
    case "${arg}" in
        --dry-run) DRY_RUN=1 ;;
        -*) echo "FATAL: unknown flag '${arg}'" >&2; exit 2 ;;
        *)
            if [ -n "${VERSION}" ]; then
                echo "FATAL: only one version argument allowed" >&2
                exit 2
            fi
            VERSION="${arg}"
            ;;
    esac
done

# --- 1. version validation (before any work) ----------------------------------
if [ -z "${VERSION}" ]; then
    echo "FATAL: a version is required: nix run .#release -- v1.2.3 [--dry-run]" >&2
    exit 2
fi
if ! printf '%s' "${VERSION}" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'; then
    echo "FATAL: version '${VERSION}' is not vMAJOR.MINOR.PATCH semver" >&2
    exit 2
fi
if git rev-parse -q --verify "refs/tags/${VERSION}" >/dev/null; then
    echo "FATAL: tag ${VERSION} already exists; refusing to re-release" >&2
    exit 2
fi

ls_check_deps cargo rustc git git-cliff gh python3 tar gzip sha256sum
ls_start_report "release"

ls_section "Version"
ls_result "version argument" PASS "${VERSION} (dry-run: ${DRY_RUN})"

# --- GitHub auth (ambient only; this app never accepts tokens) -----------------
ls_section "GitHub auth"
if gh auth status >/dev/null 2>&1; then
    ls_result "gh auth" PASS "ambient auth OK"
else
    if [ "${DRY_RUN}" -eq 1 ]; then
        ls_result "gh auth" SKIP "not authenticated; a real release would fail fast here"
    else
        echo "FATAL: gh is not authenticated." >&2
        echo "Authenticate ambiently (gh auth login) and re-run." >&2
        echo "This app never accepts tokens via arguments or environment." >&2
        exit 2
    fi
fi

# --- 2. release artifacts -------------------------------------------------------
ls_section "Release artifacts"
mkdir -p dist
ls_step "cargo build --release -p light-show --locked" \
    cargo build --release -p light-show --locked
BIN="target/release/light-show"
ART="dist/light-show-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
if [ -x "${BIN}" ]; then
    stage="${LS_SCRATCH}/stage"
    mkdir -p "${stage}"
    cp "${BIN}" LICENSE README.md "${stage}/"
    if tar -czf "${ART}" -C "${stage}" light-show LICENSE README.md >>"${LS_REPORT}" 2>&1; then
        ls_result "release tarball" PASS "${ART} ($(du -h "${ART}" | cut -f1))"
        sha256sum "${ART}" >>"${LS_REPORT}"
    else
        ls_result "release tarball" FAIL "tar failed"
        LS_FAILED=1
    fi
else
    ls_result "release tarball" FAIL "no binary at ${BIN} after build"
    LS_FAILED=1
fi

# --- Android artifacts: only where the toolchain allows -------------------------
ls_section "Android artifacts (best effort)"
if command -v cargo-apk >/dev/null 2>&1 \
    && { [ -n "${ANDROID_HOME:-}" ] || [ -n "${ANDROID_NDK_HOME:-}" ]; } \
    && rustc --print target-list 2>/dev/null | grep -q "aarch64-linux-android"; then
    ls_step "cargo apk build --release" cargo apk build --release -p light-show
    for apk in target/release/apk/*.apk; do
        [ -e "${apk}" ] || continue
        cp "${apk}" dist/
        ls_result "apk artifact" PASS "dist/$(basename "${apk}")"
    done
else
    ls_result "Android APK/AAB" SKIP \
        "needs cargo-apk + ANDROID_HOME/ANDROID_NDK_HOME + android target std in the nix toolchain (not in nixpkgs); APK builds stay primo-local per docs/FLAKE.md"
fi

# --- 3. changelog / release notes (generated, never hand-written) -----------------
ls_section "Changelog / release notes"
PREV_TAG="$(git tag --sort=-v:refname 2>/dev/null | head -n 1 || true)"
if [ -n "${PREV_TAG}" ]; then
    RANGE="${PREV_TAG}..HEAD"
else
    RANGE="HEAD"
fi
NOTES="${LS_SCRATCH}/release-notes.md"
{
    echo "# Light Show ${VERSION}"
    echo
} >"${NOTES}"
CLIFF_OK=0
if git cliff --config nix/lib/cliff.toml --tag "${VERSION}" "${RANGE}" \
    >>"${NOTES}" 2>"${LS_SCRATCH}/cliff.err"; then
    if grep -q "^- " "${NOTES}"; then
        CLIFF_OK=1
    fi
else
    cat "${LS_SCRATCH}/cliff.err" >>"${LS_REPORT}"
fi
if [ "${CLIFF_OK}" -eq 0 ]; then
    {
        echo "## Changes"
        echo
        git log "${RANGE}" --pretty=format:'- %s (%h)'
        echo
    } >>"${NOTES}"
    ls_result "changelog source" WARN "git-cliff unusable on this history; fell back to git log"
else
    ls_result "changelog source" PASS "git-cliff (${PREV_TAG:-repo root}..HEAD)"
fi
{
    echo
    echo "## Artifacts"
    for f in dist/*; do
        [ -e "${f}" ] || continue
        echo "- $(basename "${f}") (sha256: $(sha256sum "${f}" | cut -d' ' -f1))"
    done
    if [ -n "${PREV_TAG}" ]; then
        echo
        echo "**Full changelog:** https://github.com/qompassai/light-show/compare/${PREV_TAG}...${VERSION}"
    fi
} >>"${NOTES}"
{
    echo
    echo "### release notes (as uploaded)"
    echo
    cat "${NOTES}"
} >>"${LS_REPORT}"

# --- safety + debugger gates BEFORE anything is published -------------------------
ls_safety_checks
ls_debug_smoke

LS_CLEAN_EXTRAS="dist"   # dist/ is a report-time artifact dir, never "new dirt"
# --- 4+5. tag, push, create the GitHub release ------------------------------------
# Never publish when anything above failed.
if [ "${LS_FAILED}" -ne 0 ]; then
    echo "FATAL: refusing to publish with failing checks; see ${LS_REPORT}" >&2
    ls_verify_clean
    echo "report: ${LS_REPORT}"
    exit 2
fi

ls_section "Release creation"
if [ "${DRY_RUN}" -eq 1 ]; then
    ls_result "dry-run" PASS "skipped: git tag, tag push, gh release create"
    ls_result "would-be tag" SKIP "${VERSION} (not created in dry-run)"
else
    ls_step "git tag -a ${VERSION}" git tag -a "${VERSION}" -m "Light Show ${VERSION}"
    ls_step "git push origin ${VERSION}" git push origin "${VERSION}"
    # shellcheck disable=SC2086
    ls_step "gh release create ${VERSION}" gh release create "${VERSION}" \
        --title "Light Show ${VERSION}" \
        --notes-file "${NOTES}" \
        dist/*
fi

# On success the artifacts live on GitHub; drop the local copies.
if [ "${LS_FAILED}" -eq 0 ] && [ "${DRY_RUN}" -eq 0 ]; then
    rm -rf dist
    ls_result "dist/ cleaned after upload" PASS ""
else
    ls_result "dist/ retained" SKIP "dry-run or failure; inspect dist/ manually"
fi

ls_verify_clean

echo "report: ${LS_REPORT}"
if [ "${LS_FAILED}" -ne 0 ]; then
    echo "RELEASE FAILED - see ${LS_REPORT}" >&2
fi
exit "${LS_FAILED}"
