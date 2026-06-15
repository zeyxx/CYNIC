#!/bin/bash
# infra/monitoring/monitor_vms.sh
# Sondeur d'état dynamique — envoie des faits structurés au Kernel CYNIC.
# ATTENTION: script legacy. La preuve d'activation devrait venir du probe
# Kernel `fleet_inventory` (infra/probes/fleet_inventory.rs). À conserver pour
# compatibilité tant que la chaîne de bootstrap dépend de ce script.
set -euo pipefail

REGISTRY="infra/registry.json"
KERNEL_URL="http://localhost:3030/observe" 

HYPERVISORS=$(jq -r '.nodes[] | select(.role=="hypervisor") | .network.local' "$REGISTRY")
EXPECTED_VMS=$(jq -c '.nodes[] | select(.name=="cynic-host-proxmox") | .vms[]' "$REGISTRY")

for node in $HYPERVISORS; do
    # 1. Récupération brute des ressources
    DATA=$(ssh -o BatchMode=yes -o ConnectTimeout=5 cynic@"$node" "sudo pvesh get /cluster/resources --output-format json" 2>/dev/null)
    
    if [ -z "$DATA" ]; then continue; fi

    ACTUAL_IDS=$(echo "$DATA" | jq -r '.[] | select(.type=="lxc" or .type=="qemu") | .id')

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
        IP=$(ssh -o BatchMode=yes cynic@"$node" "sudo pvesh get /nodes/pve/${ID%%/*}/${ID#*/}/config --output-format json" 2>/dev/null | jq -r '.net0' | grep -oP 'ip=\K[0-9.]+' 2>/dev/null)
        
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
