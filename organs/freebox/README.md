# Organ Freebox

**Role:** `router_and_firewall_manager`
**Scope:** Local Network & Edge Perimeter

## Overview
`organ-freebox` is the sensorial and reactive organ responsible for managing the local network's gateway to the external internet. It integrates directly with the Freebox OS REST API.

## Core Responsibilities
- **Dynamic Port Forwarding:** Automatically opens (and closes) ports (e.g., 443, 80) upon Kernel demand, enabling completely sovereign, self-hosted web servers without third-party tunnels.
- **Firewall & Security:** Monitors edge traffic and enforces local IP blocking on the physical router.
- **State Management:** Keeps track of the dynamic WAN IP and coordinates with DNS services if needed.

## Architecture
Following the `DATA_CONSTITUTION.md`:
1. **Perception:** Detects Kernel intents requiring external exposure.
2. **Transformation:** Converts intents into Freebox OS API calls.
3. **Structuration:** Maintains persistent App Tokens in a secure environment.
4. **Reliability:** Acts autonomously upon network topology changes.

## Execution
The organ runs as a background process or is triggered dynamically via `organism_freebox.py`.
