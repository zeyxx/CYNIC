#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT/apps/kyon-mobile"
PACKAGE="so.cynic.kyon"
LISTENER="$PACKAGE/so.cynic.kyon.mirror.notifications.KyonNotificationListenerService"
SDK_DIR="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"

if [[ -z "$SDK_DIR" && -f "$APP_DIR/local.properties" ]]; then
    SDK_DIR="$(sed -n 's/^sdk\.dir=//p' "$APP_DIR/local.properties" | head -1)"
fi

if [[ -z "$SDK_DIR" ]]; then
    SDK_DIR="$HOME/Android/Sdk"
fi

ADB="$SDK_DIR/platform-tools/adb"
if [[ ! -x "$ADB" ]]; then
    echo "adb not found at $ADB" >&2
    exit 1
fi

if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "sqlite3 is required for the Room database probe." >&2
    exit 1
fi

serial="${1:-}"
if [[ -z "$serial" ]]; then
    serial="$("$ADB" devices | awk 'NR > 1 && $2 == "device" { print $1; exit }')"
fi

if [[ -z "$serial" ]]; then
    "$ROOT/scripts/kyon-adb-connect.sh"
    serial="$("$ADB" devices | awk 'NR > 1 && $2 == "device" { print $1; exit }')"
fi

if [[ -z "$serial" ]]; then
    echo "No connected Android device found." >&2
    exit 1
fi

out_dir="$ROOT/tmp/kyon-adb"
mkdir -p "$out_dir"
stamp="$(date +%Y%m%d-%H%M%S)"
db_copy="$out_dir/kyon-notification-probe-$stamp.db"
db_wal="$db_copy-wal"
db_shm="$db_copy-shm"
screenshot="$out_dir/kyon-notification-probe-$stamp.png"
logcat="$out_dir/kyon-notification-probe-$stamp.log"

"$ROOT/scripts/kyon-adb-cycle.sh" "$serial" >/tmp/kyon-adb-cycle.out

started_ms="$(date +%s%3N)"
"$ADB" -s "$serial" shell cmd notification allow_listener "$LISTENER"
"$ADB" -s "$serial" shell cmd notification post -t KyonProbe "kyon-probe-$stamp" metadata-only >/dev/null
sleep 2

"$ADB" -s "$serial" exec-out screencap -p > "$screenshot"
"$ADB" -s "$serial" logcat -d -t 500 > "$logcat"
"$ADB" -s "$serial" exec-out run-as "$PACKAGE" cat "databases/kyon.db" > "$db_copy"
"$ADB" -s "$serial" exec-out run-as "$PACKAGE" cat "databases/kyon.db-wal" > "$db_wal"
"$ADB" -s "$serial" exec-out run-as "$PACKAGE" cat "databases/kyon.db-shm" > "$db_shm"

match_count="$(
    sqlite3 "$db_copy" \
        "SELECT COUNT(*) FROM activity_events WHERE source = 'MOBILE_NOTIFICATION' AND eventType = 'notification_posted' AND target = 'com.android.shell' AND timestampMs >= $started_ms;"
)"

if [[ "$match_count" -lt 1 ]]; then
    echo "notification probe failed: no recent com.android.shell notification_posted row" >&2
    echo "serial=$serial" >&2
    echo "screenshot=$screenshot" >&2
    echo "logcat=$logcat" >&2
    echo "db=$db_copy" >&2
    exit 1
fi

echo "notification_probe=ok"
echo "serial=$serial"
echo "matches=$match_count"
echo "screenshot=$screenshot"
echo "logcat=$logcat"
echo "db=$db_copy"
