# CYNIC IaC — Infrastructure as Code

## Overview
Proxmox-based infrastructure for CYNIC multi-agent runtime.
Sensitive values (IPs, domains, secrets) are NOT stored here.
Use `infra/iaac-private/` (gitignored) for real configs.

## Network Planes (abstract)
| Plane | CIDR placeholder | Purpose |
|-------|-----------------|---------|
| Service | `${SERVICE_CIDR}` | Internal services |
| Admin | `${ADMIN_CIDR}` | Bare-metal management |
| Mesh (WireGuard) | `${WG_CIDR}` | Node interconnection |
| Mesh (Tailscale) | `${TS_CIDR}` | External access |

## Template Files
- `templates/proxmox-vm.tf` — Proxmox VM provisioning (Terraform)
- `templates/cloud-init.yaml` — VM bootstrap template
- `templates/headscale-config.yaml` — Headscale config template
- `templates/nginx-proxy.conf` — Reverse proxy template

## Security Policy
- Real IPs, domains, and secrets go in `infra/iaac-private/` (gitignored)
- Only templates and documentation are committed
- Never commit `.env` files or secret references
