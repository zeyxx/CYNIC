#!/bin/bash
# infra/monitoring/monitor_vms.sh
# Sondeur d'état dynamique — envoie des faits structurés au Kernel CYNIC.
# ATTENTION: script legacy. La preuve d'activation devrait venir du probe
# Kernel `fleet_inventory` (infra/probes/fleet_inventory.rs). À conserver pour
# compatibilité tant que la chaîne de bootstrap dépend de ce script.
set -euo pipefail

REGISTRY="infra/registry.json"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${CYNIC_ROOT:-$(cd "${SCRIPT_DIR}/.." && pwd)}"
source "${REPO_ROOT}/scripts/cynic-endpoint.sh"
KERNEL_URL="$(cynic_kernel_rest_url)/observe"

API_NODE="${CYNIC_PROXMOX_NODE:-pve}"
PROXMOX_URL="${CYNIC_PROXMOX_URL:-}"
PROXMOX_TOKEN_ID="${CYNIC_PROXMOX_TOKEN_ID:-}"
PROXMOX_SECRET="${CYNIC_PROXMOX_TOKEN_SECRET:-${CYNIC_PROXMOX_SECRET:-}}"

proxmox_api_call() {
    local path="$1"
    local out=""
    local rc=0
    if [[ -z "${PROXMOX_URL}" || -z "${PROXMOX_TOKEN_ID}" || -z "${PROXMOX_SECRET}" ]]; then
        echo "proxmox_api_call: missing env vars (PROXMOX_URL/PROXMOX_TOKEN_ID/PROXMOX_TOKEN_SECRET)" >&2
        return 1
    fi

    out=$(curl -ks --connect-timeout 5 --max-time 10 \
        -H "Authorization: PVEAPIToken=${PROXMOX_TOKEN_ID}=${PROXMOX_SECRET}" \
        "${PROXMOX_URL%/}${path}" 2>&1) || rc=$?
    if (( rc != 0 )); then
        echo "proxmox_api_call: curl failed for ${path} (exit ${rc})" >&2
        if [[ -n "$out" ]]; then
            echo "  ${out}" >&2
        fi
        return 1
    fi
    if [[ -z "$out" ]]; then
        echo "proxmox_api_call: empty response for ${path}" >&2
        return 1
    fi
    if ! jq -e '.data' >/dev/null 2>&1 <<<"$out"; then
        echo "proxmox_api_call: invalid JSON payload for ${path}: ${out:0:180}" >&2
        return 1
    fi
    printf '%s\n' "$out"
    return 0
}

proxmox_cluster_resources() {
    local json
    if json="$(proxmox_api_call "/api2/json/cluster/resources?type=vm")"; then
        jq -r '.data // empty' <<<"$json"
        return 0
    fi
    return 1
}

proxmox_vm_config() {
    local vm_id="$1"
    local vm_type="$2"
    local json
    if json="$(proxmox_api_call "/api2/json/nodes/${API_NODE}/${vm_type}/${vm_id}/config")"; then
        jq -r '.data // empty' <<<"$json"
        return 0
    fi
    return 1
}

ssh_cluster_resources() {
    local node="$1"
    ssh -o BatchMode=yes -o ConnectTimeout=5 "cynic@${node}" "sudo pvesh get /cluster/resources --output-format json" 2>/dev/null
}

ssh_vm_config() {
    local node="$1"
    local vm_type="$2"
    local vm_id="$3"
    ssh -o BatchMode=yes "cynic@${node}" "sudo pvesh get /nodes/pve/${vm_type}/${vm_id}/config --output-format json" 2>/dev/null
}

HYPERVISORS=$(jq -r '.nodes[] | select(.role=="hypervisor") | .network.local' "$REGISTRY")
EXPECTED_VMS=$(jq -c '.nodes[] | select(.name=="cynic-host-proxmox") | .vms[]' "$REGISTRY")

for node in $HYPERVISORS; do
    # 1. Récupération brute des ressources (API Proxmox si possible, fallback SSH)
    DATA=""
    if DATA="$(proxmox_cluster_resources)"; then
        DATA=$(jq -c '.[]' <<<"$DATA")
    else
        DATA="$(ssh_cluster_resources "$node")"
    fi
    if [[ -n "$DATA" ]]; then
        DATA=$(jq -c 'select(.type=="lxc" or .type=="qemu")' <<<"$(jq -c . <<<"$DATA")")
    fi
    
    if [ -z "$DATA" ]; then continue; fi

    ACTUAL_IDS=$(echo "$DATA" | jq -r '.id')

    echo "$EXPECTED_VMS" | while read -r expected_vm; do
        EXPECTED_ID=$(echo "$expected_vm" | jq -r '.id')
        EXPECTED_NAME=$(echo "$expected_vm" | jq -r '.name')
        EXPECTED_IP=$(echo "$expected_vm" | jq -r '.ip')

        if ! echo "$ACTUAL_IDS" | grep -qx "$EXPECTED_ID"; then
            PAYLOAD=$(jq -nc \
                --arg tool "proxmox-monitor" \
                --arg target "$EXPECTED_ID" \
                --arg name "$EXPECTED_NAME" \
                --arg state "missing" \
                --arg liveness "missing" \
                --arg ip "$EXPECTED_IP" \
                '{
                    tool: $tool,
                    target: $target,
                    domain: "infrastructure",
                    status: "observed",
                    context: {
                        name: $name,
                        state: $state,
                        liveness: $liveness,
                        ip: $ip
                    }
                }')
            curl -s -X POST "$KERNEL_URL" -H "Content-Type: application/json" -d "$PAYLOAD" > /dev/null
        fi
    done

    echo "$DATA" | jq -c '.[] | select(.type=="lxc" or .type=="qemu") | {id: .id, name: .name, status: .status}' | while read -r vm; do
        ID=$(echo "$vm" | jq -r '.id')
        NAME=$(echo "$vm" | jq -r '.name')
        STATUS=$(echo "$vm" | jq -r '.status')
        
        # 2. Découverte dynamique de l'IP via Proxmox
        # Utilisation de 'config' pour récupérer l'IP (approche générique sans dépendre de l'agent invité si possible)
        # Pour qemu: pvesh get /nodes/pve/qemu/$VMID/config
        # Pour lxc: pvesh get /nodes/pve/lxc/$VMID/config
        
        # Simplification : essayer de récupérer l'IP via Proxmox
        # Cette partie peut varier selon la configuration réseau de vos VMs (net0, etc.)
        VM_TYPE="${ID%%/*}"
        VM_ID="${ID#*/}"
        if [[ -n "${PROXMOX_URL}" && -n "${PROXMOX_TOKEN_ID}" && -n "${PROXMOX_SECRET}" ]]; then
            IP=$(proxmox_vm_config "$VM_ID" "$VM_TYPE" | jq -r '.net0' | grep -oP 'ip=\K[0-9.]+' 2>/dev/null)
        else
            IP=$(ssh_vm_config "$node" "$VM_TYPE" "$VM_ID" | jq -r '.net0' | grep -oP 'ip=\K[0-9.]+' 2>/dev/null)
        fi
        
        # 3. Test de Liveness (port 3030)
        LIVENESS="unknown"
        if [ "$STATUS" == "running" ] && [ -n "$IP" ]; then
            if curl -s --connect-timeout 2 "http://$IP:3030/health" > /dev/null; then
                LIVENESS="up"
            else
                LIVENESS="down"
            fi
        fi

        # 4. Envoi au Kernel
        PAYLOAD=$(jq -nc \
            --arg tool "proxmox-monitor" \
            --arg target "$ID" \
            --arg name "$NAME" \
            --arg state "$STATUS" \
            --arg liveness "$LIVENESS" \
            --arg ip "$IP" \
            '{
                tool: $tool,
                target: $target,
                domain: "infrastructure",
                status: "observed",
                context: {
                    name: $name,
                    state: $state,
                    liveness: $liveness,
                    ip: $ip
                }
            }')
        curl -s -X POST "$KERNEL_URL" -H "Content-Type: application/json" -d "$PAYLOAD" > /dev/null
    done
done
