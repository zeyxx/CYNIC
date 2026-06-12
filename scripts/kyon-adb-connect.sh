#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDK_DIR="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"

if [[ -z "$SDK_DIR" && -f "$ROOT/apps/kyon-mobile/local.properties" ]]; then
    SDK_DIR="$(sed -n 's/^sdk\.dir=//p' "$ROOT/apps/kyon-mobile/local.properties" | head -1)"
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
    serial="$(
        "$ADB" mdns services |
            awk '$2 == "_adb-tls-connect._tcp" { print $3; exit }'
    )"
fi

if [[ -z "$serial" ]]; then
    echo "No wireless debugging endpoint found via adb mdns services." >&2
    echo "Pair the phone once, keep Wireless debugging enabled, then retry." >&2
    exit 1
fi

"$ADB" connect "$serial"
"$ADB" devices -l
