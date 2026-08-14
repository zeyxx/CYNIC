# CYNIC Organism Roadmap

This document outlines the roadmap for the CYNIC multi-agent organism. Aligning with our core principles of organic homeostasis, bottom-up architecture, and sovereign control, this roadmap guides our transition from planned phases to fully active deployment.

---

## Vision

CYNIC is a persistent, self-improving, and sovereign multi-agent organism. Built from the bottom up (Bottom-Up Principle), CYNIC prioritizes OS, network, and hardware sovereignty (L1–L4) as the foundation for complex cognitive tasks and high-level applications (L7). 

Our vision is to cultivate a resilient ecosystem of autonomous, specialized organs (subsystems) and cortex agents ( Claude Code, Codex, Antigravity) that coordinate peer-to-peer through a persistent kernel bus. Rather than relying on rigid, centralized orchestration or external cloud control, CYNIC maintains homoeostasis through structured contract boundaries, zero-markdown dynamic memory, and a three-level sovereign validation gate.

---

## Architecture

The CYNIC organism architecture is structured as a bottom-up virtualization hierarchy:

1. **Level 0 (Bare Metal / Physical Host):** `cynic-host-proxmox` (the sovereign hypervisor), plus physical inference/operator hosts such as `cynic-host-gpu` and `cynic-host-cortex`.
2. **Level 1 (QEMU VM):** Complete hardware isolation. Hosts the active Kernel Primary (`cynic-vm-v2`), staging environments (`cynic-vm-staging`), and legacy systems undergoing migration (`cynic-vm-core`).
3. **Level 2 (LXC System Container):** OS-level virtualization sharing the host kernel for high-performance stateful organs like `organ-headscale` (Mesh VPN), `cynic-lxc-git` (sovereign Git forge), and the candidate build runner (`cynic-builder`).
4. **Level 3 (Docker Application Container):** Process encapsulation inside VMs/LXCs for ephemeral services and frontend interfaces like `cynic-portal`.

Cortex sessions interact with the persistent kernel bus via Model Context Protocol (MCP) servers (`cynic-coord`, `cynic-browser`, `cynic-docker`), while persistent autonomous daemons run as specialized Hermes agents under systemd. All infrastructure configuration, network policies, and active nodes are governed by the single source of truth: `infra/registry.json`.

---

## Prochaines étapes

Our short-to-medium-term engineering tasks are focused on hardening the persistence layer and activating planned organs:

1. **Activation of `organ-keep`:**
   - Integrate `organ-keep` into the active system topology to manage SurrealDB backups, storage durability, state migration, and data recovery contracts.
   - Deploy `organ-keep-systemd` to run daemonized storage integrity tasks.

2. **Transition and Staging:**
   - Provision and test the planned `cynic-vm-staging` node to act as a dry-run target for kernel releases.
   - Refine the candidate build runner `cynic-builder` to fully automate Level 2 (`gate-2`) release checks.

3. **CI Gate Automation & Unified CLI:**
   - Complete the transition from legacy Makefile task wrappers to the unified `cynic` CLI command suite.
   - Shift Git validation hook controls entirely into in-session MCP validation tools and the sovereign CI check suite (`make check` / `make gate`).

4. **Metabolism & Sensor Integration:**
   - Implement real-time thermal, network, and cognitive energy monitoring sensors to drive automated host selection based on the organism's energy profile.
