#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT/apps/kyon-mobile"
PACKAGE="so.cynic.kyon"
ACTIVITY="$PACKAGE/.ui.MainActivity"
SDK_DIR="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"

if [[ -z "${JAVA_HOME:-}" && -x /usr/lib/jvm/java-17-openjdk-amd64/bin/java ]]; then
    export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
fi

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

cd "$APP_DIR"
./gradlew --no-daemon :app:clean :app:assembleDebug

"$ADB" -s "$serial" install -r app/build/outputs/apk/debug/app-debug.apk
"$ADB" -s "$serial" shell am force-stop "$PACKAGE"
"$ADB" -s "$serial" shell am start -n "$ACTIVITY"
sleep 2

out_dir="$ROOT/tmp/kyon-adb"
mkdir -p "$out_dir"
stamp="$(date +%Y%m%d-%H%M%S)"

"$ADB" -s "$serial" exec-out screencap -p > "$out_dir/kyon-$stamp.png"
"$ADB" -s "$serial" logcat -d -t 300 > "$out_dir/kyon-$stamp.log"

echo "serial=$serial"
echo "screenshot=$out_dir/kyon-$stamp.png"
echo "logcat=$out_dir/kyon-$stamp.log"
