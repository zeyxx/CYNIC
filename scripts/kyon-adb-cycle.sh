#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT/apps/kyon-mobile"
PACKAGE="so.cynic.kyon"
ACTIVITY="$PACKAGE/.ui.MainActivity"
SDK_DIR="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
SSOT_REGISTRY="$ROOT/infra/registry.json"
KYN_BUILD_TASK="${KYN_GRADLE_TASK:-:app:assembleDebug}"
KYN_BUILD_MODE="${KYN_BUILD_MODE:-auto}"
KYN_KERNEL_TASK_KIND="${KYN_KERNEL_TASK_KIND:-kyon}"
KYN_KERNEL_TASK_DOMAIN="${KYN_KERNEL_TASK_DOMAIN:-kyon-mobile}"
KYN_KERNEL_TASK_TIMEOUT="${KYN_KERNEL_TASK_TIMEOUT:-900}"
KYN_KERNEL_REST_ADDR="${KYN_KERNEL_REST_ADDR:-${CYNIC_REST_ADDR:-}}"
APK_REL_PATH="app/build/outputs/apk/debug/app-debug.apk"
APK_LOCAL_PATH="$APP_DIR/$APK_REL_PATH"

GRADLE_OPTS=(
    --no-daemon
    -Dorg.gradle.daemon=false
    -Dorg.gradle.workers.max=1
    -Dkotlin.compiler.execution.strategy=in-process
    -Dkotlin.incremental=false
)

if [[ -n "${KYN_GRADLE_OPTS:-}" ]]; then
    read -r -a extra_gradle_opts <<<"${KYN_GRADLE_OPTS}"
    GRADLE_OPTS+=("${extra_gradle_opts[@]}")
fi

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

resolve_build_proxy_host() {
    local role="build_runner"
    local host=""

    if [[ ! -x "$(command -v python3 2>/dev/null || true)" ]]; then
        echo ""
        return
    fi

    host="$(python3 - "$SSOT_REGISTRY" "$role" <<'PY'
import json
import os
import sys

registry_path, role = sys.argv[1], sys.argv[2]
with open(registry_path, "r", encoding="utf-8") as handle:
    registry = json.load(handle)

host = ""
for node in registry.get("nodes", []):
    if node.get("role") == role:
        host = node.get("network", {}).get("headscale", "") or node.get("network", {}).get("local", "")
        if host.startswith("<") and host.endswith(">"):
            env_name = host[1:-1]
            host = os.environ.get(env_name, host)
        break

print(host or "", end="")
PY
)"

    printf '%s' "$host"
}

serial="${1:-}"
if [[ -z "$serial" ]]; then
    serial="$($ADB devices | awk 'NR > 1 && $2 == "device" { print $1; exit }')"
fi

if [[ -z "$serial" ]]; then
    "$ROOT/scripts/kyon-adb-connect.sh"
    serial="$($ADB devices | awk 'NR > 1 && $2 == "device" { print $1; exit }')"
fi

if [[ -z "$serial" ]]; then
    echo "No connected Android device found." >&2
    exit 1
fi

cd "$APP_DIR"
SSH_OPTS=(
    -o "ConnectTimeout=8"
    -o "BatchMode=yes"
    -o "ServerAliveInterval=10"
    -o "ServerAliveCountMax=3"
    -o "StrictHostKeyChecking=accept-new"
)
RSYNC_REMOTE_OPTS=(
    -az
    --delete
    --exclude='.git/'
    --exclude='.gradle/'
    --exclude='build/'
)

check_proxy_reachability() {
    local host="$1"
    local out

    if ! out="$(ssh "${SSH_OPTS[@]}" "$host" 'true' 2>&1)"; then
        echo "SSH probe failed for $host" >&2
        echo "$out" >&2
        return 1
    fi

    return 0
}

build_command_string() {
    local cmd=("./gradlew" "${GRADLE_OPTS[@]}" "$KYN_BUILD_TASK")
    local arg
    local out=""

    for arg in "${cmd[@]}"; do
        out+=" "
        out+="$(printf '%q' "$arg")"
    done

    printf '%s' "${out# }"
}

build_locally() {
    local cmd=("./gradlew" "${GRADLE_OPTS[@]}" "$KYN_BUILD_TASK")
    "${cmd[@]}"
}

sync_apk_from_proxy() {
    if [[ -z "${KYN_BUILD_PROXY_HOST:-}" || -z "${KYN_BUILD_PROXY_DIR:-}" ]]; then
        return 1
    fi

    if [[ ! -x "$(command -v rsync 2>/dev/null || true)" ]]; then
        return 1
    fi

    mkdir -p "$(dirname "$APK_LOCAL_PATH")"
    rsync -e "ssh ${SSH_OPTS[*]}" \
        -az \
        "$KYN_BUILD_PROXY_HOST:$KYN_BUILD_PROXY_DIR/$APK_REL_PATH" \
        "$APK_LOCAL_PATH"
}

build_via_proxy() {
    if [[ -z "${KYN_BUILD_PROXY_HOST:-}" || -z "${KYN_BUILD_PROXY_DIR:-}" ]]; then
        echo "proxy mode requires KYN_BUILD_PROXY_HOST and KYN_BUILD_PROXY_DIR" >&2
        return 1
    fi

    if [[ ! -x "$(command -v rsync 2>/dev/null || true)" ]]; then
        echo "rsync not found locally; cannot use remote proxy." >&2
        return 1
    fi

    if ! check_proxy_reachability "$KYN_BUILD_PROXY_HOST"; then
        echo "Build fallback: proxy $KYN_BUILD_PROXY_HOST is not reachable (firewall/ACL/Tailscale issue)." >&2
        return 1
    fi

    rsync -e "ssh ${SSH_OPTS[*]}" \
        "${RSYNC_REMOTE_OPTS[@]}" \
        "$APP_DIR/" \
        "$KYN_BUILD_PROXY_HOST:$KYN_BUILD_PROXY_DIR/apps/kyon-mobile/"

    local remote_cmd
    remote_cmd="$(build_command_string)"
    ssh "${SSH_OPTS[@]}" "$KYN_BUILD_PROXY_HOST" <<REMOTE_EOF
set -euo pipefail
cd '$KYN_BUILD_PROXY_DIR/apps/kyon-mobile'
$remote_cmd
REMOTE_EOF

    rsync -e "ssh ${SSH_OPTS[*]}" -az \
        "$KYN_BUILD_PROXY_HOST:$KYN_BUILD_PROXY_DIR/apps/kyon-mobile/$APK_REL_PATH" \
        "$APK_LOCAL_PATH"
}

build_via_kernel() {
    if [[ -z "${CYNIC_API_KEY:-}" ]]; then
        echo "CYNIC_API_KEY is required for kernel mode." >&2
        return 1
    fi

    if [[ ! -x "$(command -v python3 2>/dev/null || true)" ]]; then
        echo "python3 is required for kernel mode." >&2
        return 1
    fi

    local cmd
    local project_root="${KYN_KERNEL_PROJECT_ROOT:-}"
    if [[ -z "$project_root" && -n "${KYN_BUILD_PROXY_DIR:-}" ]]; then
        project_root="$KYN_BUILD_PROXY_DIR/apps/kyon-mobile"
    fi
    if [[ -z "$project_root" ]]; then
        project_root="$APP_DIR"
    fi

    # Build worker executes in its own host context; use a runner-visible
    # workspace path when available.
    cmd="$(build_command_string)"

    python3 "$ROOT/scripts/kyon-kernel-build-task.py" \
        --kernel-url "$KYN_KERNEL_REST_ADDR" \
        --api-key "$CYNIC_API_KEY" \
        --project-root "$project_root" \
        --command "$cmd" \
        --kind "$KYN_KERNEL_TASK_KIND" \
        --domain "$KYN_KERNEL_TASK_DOMAIN" \
        --artifact-path "$APK_REL_PATH" \
        --timeout "$KYN_KERNEL_TASK_TIMEOUT"

    if sync_apk_from_proxy; then
        return 0
    fi

    if [[ ! -f "$APK_LOCAL_PATH" ]]; then
        echo "Kernel task succeeded but APK not present locally and proxy sync is unavailable." >&2
        return 1
    fi

    return 0
}

if [[ -z "${KYN_BUILD_PROXY_HOST:-}" ]]; then
    KYN_BUILD_PROXY_HOST="$(resolve_build_proxy_host)"
fi

if [[ -n "${KYN_BUILD_PROXY_HOST:-}" ]]; then
    # Source of truth from infra/registry.json (role: build_runner).
    if [[ -z "${KYN_BUILD_PROXY_DIR:-}" ]]; then
        KYN_BUILD_PROXY_DIR="$HOME/.cynic/repos/CYNIC"
    fi
    if [[ "$KYN_BUILD_PROXY_HOST" == "<"*">" ]]; then
        echo "Cannot resolve SSOT proxy host placeholder '$KYN_BUILD_PROXY_HOST'." >&2
        echo "Export the resolved value (ex: export KYN_BUILD_PROXY_HOST=user@100.64.x.x)." >&2
        exit 1
    fi
fi

if [[ "$KYN_BUILD_MODE" == "auto" ]]; then
    if [[ -n "${CYNIC_API_KEY:-}" ]]; then
        KYN_BUILD_MODE="kernel"
    elif [[ -n "${KYN_BUILD_PROXY_HOST:-}" ]]; then
        KYN_BUILD_MODE="proxy"
    else
        KYN_BUILD_MODE="local"
    fi
fi

if [[ "$KYN_BUILD_MODE" == "kernel" && -z "${KYN_KERNEL_REST_ADDR:-}" ]]; then
    echo "Cannot run kernel mode without CYNIC_REST_ADDR (or KYN_KERNEL_REST_ADDR)." >&2
    exit 1
fi

case "$KYN_BUILD_MODE" in
    kernel)
        echo "Build mode=kernel via $KYN_KERNEL_REST_ADDR" >&2
        if ! build_via_kernel; then
            echo "Kernel mode failed, fallback local build." >&2
            build_locally
        fi
        ;;
    proxy)
        echo "Build mode=proxy via ${KYN_BUILD_PROXY_HOST:-}" >&2
        if ! build_via_proxy; then
            echo "Proxy mode failed, fallback local build." >&2
            build_locally
        fi
        ;;
    local)
        echo "Build mode=local" >&2
        build_locally
        ;;
    *)
        echo "Unknown KYN_BUILD_MODE '$KYN_BUILD_MODE', falling back to local." >&2
        build_locally
        ;;
esac

"$ADB" -s "$serial" install -r "$APK_LOCAL_PATH"
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
