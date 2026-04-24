# CYNIC Soma — The Organism's Body Management

> *Soma* (σῶμα) — the body, as distinct from the mind. CYNIC's physical substrate: network, processes, storage, silicon.

**Status:** Design document. Epistemic status: CONJECTURE (φ⁻² confidence).

**Problem:** CYNIC reasons about tokens, verdicts, and crystals — but is blind to its own body. It doesn't know what ports are open, what processes are running, or whether its firewall is intact. An organism that can't feel its own flesh is vulnerable to the simplest attacks.

**Principle:** The organism must sense its own body before it can defend it.

---

## 1. Current State (observed 2026-04-24)

### What CYNIC knows about itself
- `/health` → storage status, Dog circuit states, crystal state, environment probes
- `/metrics` → verdict counts, Dog stats, cache size
- System probes: CPU/RAM/disk via `EnvironmentSnapshot`

### What CYNIC doesn't know
- What ports are open on its host
- What processes are running (including non-CYNIC ones)
- Whether its firewall exists or is enforced
- Whether mitmdump/RustDesk/other tools are exposing attack surface
- Which network interfaces are active and what they expose
- Whether DNS resolution is sovereign or ISP-controlled

### The gap
CYNIC can judge a token's legitimacy at 61.8% confidence but can't detect a remote desktop tool binding on all interfaces. The organism has a cortex but no peripheral nervous system.

---

## 2. Architecture — Three Layers

```
┌─────────────────────────────────────┐
│  L3: REFLEXES (automated response)  │  ← Phase 3
│  Alert on unauthorized port          │
│  Restart crashed organ               │
│  Shed load on resource exhaustion    │
├─────────────────────────────────────┤
│  L2: AWARENESS (kernel endpoint)    │  ← Phase 2
│  GET /soma → port map, processes,   │
│  firewall state, resource usage     │
│  Exposes body state to /health      │
├─────────────────────────────────────┤
│  L1: SKELETON (nftables + systemd)  │  ← Phase 1 (NOW)
│  Declarative firewall policy         │
│  Every organ = systemd service       │
│  Network binding policy              │
└─────────────────────────────────────┘
```

---

## 3. Phase 1 — The Skeleton

### 3.1 Firewall Policy (nftables)

**Philosophy:** Default DENY inbound. Tailscale is the perimeter. LAN is untrusted (shared WiFi = adversarial). Only WireGuard (UDP 41641) and mDNS pass on LAN.

**File:** `~/.config/cynic/firewall.nft`

```nft
#!/usr/sbin/nft -f
# CYNIC Soma — Firewall Policy
# Applied by: cynic-firewall.service (oneshot, on boot)
# Last audit: 2026-04-24

flush ruleset

table inet cynic {
    chain input {
        type filter hook input priority 0; policy drop;

        # ── Always allow ──
        iif "lo" accept comment "loopback"
        ct state established,related accept comment "return traffic"
        ct state invalid drop comment "malformed"

        # ── Tailscale mesh (tailscale0) — full trust ──
        iif "tailscale0" accept comment "inter-node CYNIC traffic"

        # ── VM bridge (virbr0) — local VMs ──
        iif "virbr0" accept comment "libvirt VMs"

        # ── LAN WiFi (wlp41s0) — minimal ──
        iif "wlp41s0" udp dport 41641 accept comment "WireGuard for Tailscale"
        iif "wlp41s0" udp dport 5353 accept comment "mDNS discovery"
        # RustDesk: bind to Tailscale IP (not 0.0.0.0), but allow LAN as fallback
        # for mobile/Windows clients not on Tailnet
        iif "wlp41s0" udp dport { 21119, 33260 } accept comment "RustDesk (restrict after Tailscale-only config)"

        # ── Log drops for debugging ──
        limit rate 5/minute log prefix "CYNIC_DROP: " drop
    }

    chain forward {
        type filter hook forward priority 0; policy drop;
        iif "virbr0" accept comment "VM outbound"
        oif "virbr0" ct state established,related accept comment "VM return"
    }

    chain output {
        type filter hook output priority 0; policy accept;
    }
}
```

**RustDesk note:** Kept open on LAN for now (mobile/Windows without Tailscale). Phase 2 goal: move all RustDesk clients to Tailscale, then close LAN ports. The firewall rule has a comment marking it for future restriction.

### 3.2 Systemd Service: Firewall Loader

**File:** `~/.config/systemd/user/cynic-firewall.service`

```ini
[Unit]
Description=CYNIC Soma firewall (nftables)
Before=cynic-kernel.service

[Service]
Type=oneshot
RemainAfterExit=yes
# Needs root — uses sudo with NOPASSWD for this specific command
ExecStart=/usr/bin/sudo /usr/sbin/nft -f %h/.config/cynic/firewall.nft
ExecStop=/usr/bin/sudo /usr/sbin/nft flush ruleset

[Install]
WantedBy=default.target
```

**Requires:** sudoers entry: `<user> ALL=(root) NOPASSWD: /usr/sbin/nft -f /home/<user>/.config/cynic/firewall.nft, /usr/sbin/nft flush ruleset, /usr/sbin/nft list ruleset`

### 3.3 Systemd Service: Hermes Proxy

**File:** `~/.config/systemd/user/hermes-proxy.service`

```ini
[Unit]
Description=CYNIC Hermes X proxy (mitmdump)
After=cynic-kernel.service

[Service]
Type=simple
WorkingDirectory=%h/Bureau/CYNIC/scripts/hermes-x
ExecStart=%h/.local/bin/mitmdump -s x_proxy.py --listen-host 127.0.0.1 -p 8888 --set stream_large_bodies=1m --set ssl_insecure=true -q
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
```

### 3.4 Network Binding Policy

Every CYNIC process must declare its bind address:

| Process | Must bind to | Reason |
|---------|-------------|--------|
| cynic-kernel | Tailscale IP (:3030) | Accessible via Funnel + mesh |
| llama-server | Tailscale IP (:8080, :8081) | Inter-node inference |
| SurrealDB | 127.0.0.1 (:8000) | Kernel-local only |
| hermes-proxy | 127.0.0.1 (:8888) | Browser-local proxy |
| RustDesk | Tailscale IP (goal) | Remote access via mesh |

**Rule:** Any process binding `0.0.0.0` on a port not in the firewall whitelist = body violation. Phase 2 detects this automatically.

### 3.5 RustDesk Restriction

**Current:** System service, public relay (rs-ny), 0.0.0.0 binding.

**Target:** Bind to Tailscale IP only. Disable public relay when all access devices are on Tailnet.

**Transition path:**
1. Install Tailscale on mobile/Windows access devices
2. Configure RustDesk `direct-server` to Tailscale IP
3. Set `relay-server = ""` (disable public relay)
4. Close firewall LAN ports 21119/33260

**Not now:** This requires installing Tailscale on all access devices first. Premature closure = locked out.

---

## 4. Phase 2 — Awareness (post-hackathon)

Add a kernel probe that reads body state:

```rust
// domain/soma.rs
pub struct SomaSnapshot {
    pub listening_ports: Vec<PortInfo>,    // from `ss -tlnp`
    pub processes: Vec<ProcessInfo>,        // CYNIC-managed processes
    pub firewall_active: bool,             // nft ruleset exists
    pub interfaces: Vec<InterfaceInfo>,    // network interfaces + IPs
    pub violations: Vec<SomaViolation>,    // unauthorized bindings
}
```

Exposed via `GET /soma` (authenticated). Integrated into `/health` as `"soma": {...}`.

**Violation detection:** Compare `listening_ports` against the binding policy table. Any mismatch = `SomaViolation`.

---

## 5. Phase 3 — Reflexes (future)

Automated responses to body violations:
- Port violation detected → alert via `/events` SSE + crystal observation
- Organ crash → systemd restart (already works)
- Resource exhaustion → shed lowest-priority Dog (gemma first, deterministic last)
- Firewall absent → kernel enters degraded mode, all endpoints require auth

**FOGC test:** If I invert SOVEREIGNTY, does any line change? Yes — the reflex policy is what SOVEREIGNTY means at the body layer. This passes.

---

## 6. Falsification

| Claim | Falsified if |
|-------|-------------|
| nftables policy blocks unauthorized inbound | Port scan from LAN shows open ports not in whitelist |
| Systemd service management covers all organs | `pgrep` shows CYNIC processes not managed by systemd |
| Phase 2 soma probe detects violations | Manually bind 0.0.0.0:9999, soma doesn't flag it |
| RustDesk via Tailscale only works | Mobile/Windows client can't connect after relay removal |

---

## Dependencies

- **sudo access for nft:** Required. Sudoers entry scoped to nft commands only.
- **Tailscale on access devices:** Required before closing RustDesk LAN ports.
- **Kernel domain/soma.rs:** Phase 2 — new domain module, new REST endpoint.

## What this does NOT solve

- KC1 (disk encryption) — separate concern, requires LUKS
- KC2 (Tailscale SPOF) — separate concern, requires Headscale
- KC5 (DNS poisoning) — separate concern, requires Unbound/Pi-hole
- AMD PSP — hardware-level, no software mitigation
