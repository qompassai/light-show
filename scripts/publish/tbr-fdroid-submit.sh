#!/usr/bin/env bash
# matt-fdroid-submit.sh -- F-Droid submission for light-show (Matt-only steps).
#
# F-Droid needs nothing secret from you: they build and sign the app
# themselves. The only human steps are the GitLab fork and the merge
# request, which need YOUR GitLab account, so agents cannot do them.
#
# Usage:
#   scripts/matt/matt-fdroid-submit.sh --dry-run   # run the upstream gates, print the exact MR steps
#   scripts/matt/matt-fdroid-submit.sh             # same gates, then print the steps to execute

set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
CHECK="$REPO_DIR/scripts/publish/bin/fdroid-publish-check"
APPID=ai.qompass.lightshow
VERCODE=1

die() { echo "ERROR: $*" >&2; exit 1; }

[ -x "$CHECK" ] || die "check binary missing at $CHECK"

echo "=== Gate check (real tree: $REPO_DIR) ==="
"$CHECK" --repo "$REPO_DIR" --appid "$APPID" --vercode "$VERCODE" || true
echo ""
echo "=== F-Droid submission steps (your GitLab account) ==="
echo "1. Fork https://gitlab.com/fdroid/fdroiddata on GitLab."
echo "2. Clone your fork; create branch '$APPID'."
echo "3. Copy the recipe from this repo's docs/FDROID.md (Reference metadata"
echo "   recipe) into your fork at metadata/$APPID.yml, replacing"
echo "   'commit:' with the full 40-char SHA of the release commit."
echo "4. In your fork: fdroid readmeta && fdroid rewritemeta $APPID"
echo "   (must produce no diff) && fdroid lint $APPID (must be clean)."
echo "5. Commit as 'New App: $APPID', push, open the MR against"
echo "   fdroid/fdroiddata:master, fill the MR template."
echo "6. Answer reviewer questions. After merge, allow 24-48h for the app"
echo "   to appear (signing needs human keystore access)."
echo ""
if [ "${1:-}" = "--dry-run" ]; then
    echo "dry-run: gates printed above; nothing was changed, no MR opened."
else
    echo "Non-dry-run: the gates above are the pre-submit check. Steps 1-6 are manual."
fi
