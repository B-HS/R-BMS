#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-}"
if [ -z "$TARGET" ] || [ ! -e "$TARGET" ]; then
    echo "usage: bash scripts/macos-notarize.sh <R-BMS.app | file.dmg>  (APPLE_ID, APPLE_APP_SPECIFIC_PASSWORD, APPLE_TEAM_ID in the environment)" >&2
    exit 2
fi
for name in APPLE_ID APPLE_APP_SPECIFIC_PASSWORD APPLE_TEAM_ID; do
    if [ -z "${!name:-}" ]; then
        echo "$name is not set" >&2
        exit 2
    fi
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

case "$TARGET" in
    *.app)
        SUBMISSION="$WORK/$(basename "$TARGET" .app).zip"
        ditto -c -k --keepParent "$TARGET" "$SUBMISSION"
        ;;
    *)
        SUBMISSION="$TARGET"
        ;;
esac

xcrun notarytool submit "$SUBMISSION" \
    --apple-id "$APPLE_ID" --password "$APPLE_APP_SPECIFIC_PASSWORD" --team-id "$APPLE_TEAM_ID" \
    --wait | tee "$WORK/notarize.log"
if ! grep -q 'status: Accepted' "$WORK/notarize.log"; then
    echo "notarization of $TARGET was not accepted" >&2
    exit 1
fi
xcrun stapler staple "$TARGET"
xcrun stapler validate "$TARGET"
