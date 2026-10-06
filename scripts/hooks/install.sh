#!/bin/bash
# Installs the light-show git hooks into .git/hooks/.
set -e
REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOK_SRC="$REPO_ROOT/scripts/hooks/post-commit"
HOOK_DST="$REPO_ROOT/.git/hooks/post-commit"
cp "$HOOK_SRC" "$HOOK_DST"
chmod +x "$HOOK_DST"
echo "Installed post-commit hook -> $HOOK_DST"
