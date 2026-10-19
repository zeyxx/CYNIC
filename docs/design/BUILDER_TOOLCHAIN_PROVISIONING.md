# Builder Toolchain Provisioning (Go, Node)

Status: `normative procedure` (2026-10-18)
Owner: CYNIC builder node (cynic-builder, 10.10.10.119)

## Purpose

The builder (10.10.10.119) is the SSOT for builds. Historically it carried
only Rust/cargo, so external Go/Node projects could not be built or tested
there, forcing observation onto cortex and breaking the "build on builder"
invariant. This document records the durable provisioning of Go and Node so
that any future agent discovers the toolchains from the SSOT instead of
probing.

## SSOT

`infra/registry.json` → node `cynic-builder` → `toolchains` field lists every
provisioned tool with version, install path, bin path, symlink, and purpose.
Agents MUST read this field before assuming a toolchain is available. A tool
not listed there is not provisioned.

## Provisioned toolchains (2026-10-18)

| Tool | Version | Install path | Symlink |
|------|---------|--------------|---------|
| go | go1.27.2 | /opt/go | /usr/local/bin/go |
| node | v22.23.3 | /opt/node | /usr/local/bin/node |
| npm | 10.9.9 | /opt/node | /usr/local/bin/npm |

## Procedure (reproducible)

1. Verify network access: `curl -sI https://go.dev` and `https://nodejs.org`.
2. Download the official tarball:
   - Go: `wget https://go.dev/dl/go<VERSION>.linux-amd64.tar.gz`
   - Node: `wget https://nodejs.org/dist/v<VERSION>/node-v<VERSION>-linux-x64.tar.xz`
3. Extract under /opt (never /tmp, never a hardcoded ./target):
   - `rm -rf /opt/go && tar -C /opt -xzf go.tar.gz`
   - `rm -rf /opt/node && tar -C /opt -xJf node.tar.xz && mv /opt/node-v*-linux-x64 /opt/node`
4. Wire PATH durably:
   - Symlinks in /usr/local/bin: `ln -sf /opt/<tool>/bin/<bin> /usr/local/bin/<bin>`
   - `/etc/profile.d/cynic-toolchains.sh` exporting PATH (for interactive shells).
5. Verify: `go version`, `node --version`, `npm --version` (non-interactive SSH).
6. Formalize in CYNIC:
   - Add the tool to `infra/registry.json` → `cynic-builder.toolchains.tools`.
   - Update `infra/external-projects.json` observation_environment if affected.
   - Commit + push + PR.

## Anti-rules

- Never install toolchains via apt on the builder (versions drift, untracked).
- Never hardcode toolchain paths in CLI args or code — resolve from the registry.
- Never assume a toolchain is present without checking `cynic-builder.toolchains`.
- Never install under /tmp (ephemeral) or inside the CYNIC tree (pollution).

## Related

- `infra/registry.json` (cynic-builder.toolchains)
- `docs/design/EXTERNAL_PROJECT_CONSORTIUM.md` (§9 observation results)
- `infra/external-projects.json` (observation_environment)