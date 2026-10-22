# C0-2 / C0-3 / C0-10 / C0-13 execution log — 2026-10-22 (hypervisor)

Branch: `agent/hermes/c03-hypervisor`. Approval: operator explicit per firewall command; blanket go for the rest.
Note: `origin/main` advanced during the day (`d88f1312` -> `4250fff2`, PR #239). This branch bases `4250fff2`; earlier day-branches base `d88f1312` and need rebase before PRs.

## C0-2 sshd hardening: DONE (pve root)

- `PasswordAuthentication no`, `PermitRootLogin prohibit-password` (canonical `without-password`). Rollback copies: `/root/c03-hypervisor-20261022/sshd_config`.
- Proof of no lockout: fresh `ssh root@192.168.100.1` AFTER reload prints connection OK and the hardened `sshd -T` values.

## C0-3 firewall: DONE host-level (pve root)

- `/etc/pve/firewall/cluster.fw`: `enable: 1`, `policy_in: ACCEPT`, `policy_out: ACCEPT` (fail-open base, deliberate).
- `/etc/pve/nodes/pve/host.fw`: `enable: 1`; ACCEPT SSH(22)+8006 from `eno1`/admin LAN only; ACCEPT 80/443 on `wlp1s0`; DROP 22/8006/111/3128/tcp + 53/udp on `wlp1s0`.
- Lessons: `policy_in` is INVALID in host.fw (parse error, config stays pending) — it belongs in cluster.fw. `pve-firewall compile` validates before applying (used). Datacenter `enable: 1` is required or host rules stay inert ("firewall disabled" on first compile).
- Proof: `pve-firewall status` = enabled/running, 116 PVEFW iptables entries, wlp1s0 DROPs listed, fresh admin ssh OK.
- NOT done: per-CT firewalls, egress rules, `ip_forward` review, port scan from outside (no external probe available; operator laptop can run it).

## C0-10 cloud-api retired: DONE (pve root)

- `systemctl stop + disable cynic-cloud` (was active as root). Binary left in place at `/usr/local/bin/cynic-cloud-api` (archive step deferred — one `mv` to `/root`, not yet done).
- Removed the CYNIC Gateway block (`/api/deploy` + `/health`) from the api-talaria nginx site (`nginx -t` OK before reload; rollback copy in `/root/c03-hypervisor-20261022/`).
- Proof: `https://api.talaria.build/api/deploy` = 404 publicly (was 502 with dead backend); nginx access logs show no `/api/deploy` hits (per audit).

## C0-13 headscale vhost: DONE (pve root)

- `proxy_pass` 10.10.10.100:8080 -> 10.10.10.40:8080 (`nginx -t` OK before reload; rollback copy kept).
- Proof: `https://cynic-hs.duckdns.org/health` = 200 publicly.

## Exit criteria status

- [x] `sshd -T` shows `passwordauthentication no`.
- [x] `pve-firewall status` enabled/running.
- [x] `cynic-cloud.service` inactive + `/api/deploy` 404 publicly.
- [ ] External port scan of 192.168.0.37 (operator to run).
- [ ] Binary archived (one `mv` left).
