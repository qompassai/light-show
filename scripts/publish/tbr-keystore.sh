#!/usr/bin/env bash
# matt-keystore.sh -- create the Google Play upload keystore for light-show.
#
# Matt-only: this generates a real signing key. Agents never run this for
# real; it exists so Matt can do the one step only he can do, exactly once,
# with the exact parameters the release runbook expects.
#
# Usage:
#   scripts/matt/matt-keystore.sh --dry-run   # prove the keytool command works (throwaway keystore in /tmp, deleted after)
#   scripts/matt/matt-keystore.sh             # create the real keystore (interactive)
#
# After creation:
#   1. Back up the keystore file OFFLINE (USB stick / encrypted backup).
#      Losing it means a new Play Store listing.
#   2. Store the passwords in pass:
#        qompassai/light-show/upload-keystore-password
#        qompassai/light-show/upload-key-password
#   3. Never commit the keystore or passwords to git.

set -euo pipefail

KEYTOOL=/usr/lib/jvm/java-17-openjdk/bin/keytool
ALIAS="${LIGHTSHOW_KEY_ALIAS:-lightshow-upload}"
KEYSTORE_OUT="${LIGHTSHOW_KEYSTORE_OUT:-$HOME/.config/qompassai/light-show/upload.jks}"

die() { echo "ERROR: $*" >&2; exit 1; }

[ -x "$KEYTOOL" ] || die "keytool not found at $KEYTOOL"

if [ "${1:-}" = "--dry-run" ]; then
    echo "dry-run: validating keytool command line with a throwaway keystore"
    echo "keytool: $KEYTOOL"
    "$KEYTOOL" -genkeypair -v \
        -keystore /tmp/lightshow-keystore-dryrun.jks \
        -alias dryrun-throwaway \
        -keyalg RSA -keysize 2048 -validity 10950 \
        -storepass "dryrun-storepass-123" \
        -keypass "dryrun-keypass-123" \
        -dname "CN=Dry Run, OU=Qompass AI, O=Qompass AI, L=Unknown, ST=Unknown, C=US" \
        2>&1 | tail -3
    echo "dry-run: verifying throwaway keystore"
    "$KEYTOOL" -list -v -keystore /tmp/lightshow-keystore-dryrun.jks \
        -storepass "dryrun-storepass-123" -alias dryrun-throwaway 2>/dev/null \
        | grep -E "Alias name|Valid from" || die "throwaway keystore verification failed"
    rm -f /tmp/lightshow-keystore-dryrun.jks
    echo "dry-run: PASS (throwaway keystore created, verified, deleted; nothing real was written)"
    echo "real run would write: $KEYSTORE_OUT (alias: $ALIAS)"
    exit 0
fi

echo "This will create the REAL Play upload keystore."
echo "Target: $KEYSTORE_OUT"
echo "Alias:  $ALIAS"
echo ""
read -rsp "Keystore password (min 6 chars): " STOREPASS; echo ""
read -rsp "Key password (min 6 chars, may match keystore password): " KEYPASS; echo ""
[ "${#STOREPASS}" -ge 6 ] || die "keystore password too short"
[ "${#KEYPASS}" -ge 6 ] || die "key password too short"
read -rp "Your name for the certificate CN [Qompass AI]: " CN
CN="${CN:-Qompass AI}"

mkdir -p "$(dirname "$KEYSTORE_OUT")"
[ -e "$KEYSTORE_OUT" ] && die "$KEYSTORE_OUT already exists; refusing to overwrite"

"$KEYTOOL" -genkeypair -v \
    -keystore "$KEYSTORE_OUT" \
    -alias "$ALIAS" \
    -keyalg RSA -keysize 2048 -validity 10950 \
    -storepass "$STOREPASS" -keypass "$KEYPASS" \
    -dname "CN=${CN}, OU=Qompass AI, O=Qompass AI, C=US"

chmod 600 "$KEYSTORE_OUT"
echo ""
echo "Created: $KEYSTORE_OUT"
echo "Next: back it up OFFLINE, then store both passwords in pass"
echo "  (qompassai/light-show/upload-keystore-password,"
echo "   qompassai/light-show/upload-key-password)."
unset STOREPASS KEYPASS
