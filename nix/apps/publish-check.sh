# ls-publish-check: deterministic F-Droid / store-metadata readiness checks.
#
# Runs the vendored fdroid-publish-check (stdlib-only Python) against this
# checkout: license file, fastlane metadata tree, release tags, non-free
# dependency markers, and versionCode/versionName literals.
#
# HUMAN-GATED BOUNDARY (documented, never executed here):
#   scripts/publish/tbr-keystore.sh      - generates the upload keystore
#   scripts/publish/tbr-fdroid-submit.sh - submits the recipe to F-Droid
#   scripts/publish/tbr-play-console.md  - Play Console release checklist
# A deterministic sandbox must never mint signing identities or submit
# anything to a store. Those stay Matt-only.

ls_check_deps python3 git cargo-deny
ls_start_report "publish-check"

ls_section "F-Droid publish readiness (vendored fdroid-publish-check)"
ls_step "fdroid-publish-check --appid ai.qompass.lightshow --vercode 1" \
    python3 scripts/publish/bin/fdroid-publish-check \
    --repo . --appid ai.qompass.lightshow --vercode 1

ls_section "Human-gated boundary (intentionally not run)"
{
    echo "These scripts are excluded from all flake apps by design:"
    echo "- scripts/publish/tbr-keystore.sh (generates the upload keystore)"
    echo "- scripts/publish/tbr-fdroid-submit.sh (submits to F-Droid)"
    echo "- scripts/publish/tbr-play-console.md (Play Console checklist)"
    echo "They must be run by Matt himself."
} >>"${LS_REPORT}"
ls_result "human-gated scripts excluded" PASS "tbr-* untouched by design"

ls_safety_checks
ls_debug_smoke
ls_verify_clean

echo "report: ${LS_REPORT}"
if [ "${LS_FAILED}" -ne 0 ]; then
    echo "PUBLISH-CHECK FAILED - see ${LS_REPORT}" >&2
fi
exit "${LS_FAILED}"
