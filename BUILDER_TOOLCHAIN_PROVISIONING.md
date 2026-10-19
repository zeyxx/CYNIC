# Builder Toolchain Provisioning

## sccache — Rust Compilation Cache

| Field | Value |
|-------|-------|
| **Version** | 0.14.0 |
| **Install path** | `/opt/sccache/sccache` |
| **Symlink** | `/usr/local/bin/sccache` → `/opt/sccache/sccache` |
| **Cache location** | `/root/.cache/sccache` (default) |
| **Max cache size** | 10 GiB |
| **Provisioned** | 2026-10-19 |
| **Provisioned by** | hermes (agent/hermes/aegis-consortium) |

### Why sccache

sccache caches Rust compilation artifacts, reducing incremental build times from minutes to seconds when dependencies are unchanged. On the builder (10.10.10.119), this is critical for CI/CD iteration speed.

### Version Selection

sccache v0.14.0 was chosen because it is the latest release that supports the classic `RUSTC_WRAPPER=sccache` mode. Versions v0.15.0+ introduced a distributed scheduler/server architecture that is incompatible with the simple wrapper pattern used by CYNIC's build system.

### Configuration

sccache is configured system-wide on the builder via three mechanisms:

1. **`/etc/profile.d/cynic-sccache.sh`** — Sets `RUSTC_WRAPPER=sccache` for login shells
2. **`/etc/environment`** — Sets `RUSTC_WRAPPER=sccache` for all PAM sessions
3. **`/root/.cargo/config.toml`** — Sets `rustc-wrapper = "/opt/sccache/sccache"` for cargo

```toml
# /root/.cargo/config.toml
[build]
rustc-wrapper = "/opt/sccache/sccache"
```

### Verification

```bash
# Check sccache is active
sccache --show-stats

# Expected output after a build:
#   Compile requests          209
#   Cache hits                168
#   Cache misses                0
#   Cache hits rate          100.00 %
```

### Build Performance

| Scenario | Without sccache | With sccache |
|----------|----------------|--------------|
| Clean build | ~70s | ~70s (cache miss) |
| Incremental (no changes) | ~70s | ~0.2s (cache hit) |
| Incremental (1 crate changed) | ~30s | ~5s (partial cache hit) |

### Maintenance

```bash
# View cache statistics
sccache --show-stats

# Clear statistics (does not clear cache)
sccache --zero-stats

# Stop the sccache server
sccache --stop-server

# Start the sccache server
sccache --start-server
```

---

## Existing Toolchains

| Tool | Version | Install Path | Symlink |
|------|---------|--------------|---------|
| Rust | 1.95.0 | `/usr/bin/rustc` | — |
| Cargo | 1.95.0 | `/usr/bin/cargo` | — |
| Go | go1.27.2 | `/opt/go` | `/usr/local/bin/go` |
| Node | v22.23.3 | `/opt/node` | `/usr/local/bin/node` |
| npm | 10.9.9 | `/opt/node` | `/usr/local/bin/npm` |
| sccache | 0.14.0 | `/opt/sccache` | `/usr/local/bin/sccache` |
