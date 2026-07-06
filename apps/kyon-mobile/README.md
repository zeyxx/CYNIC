# Kyon Mobile

Android-first mobile cortex for CYNIC.

Kyon has two separate axes:

- Kyon Shield: call/SMS protection backed by the CYNIC `phone-number` domain.
- Kyon Mirror: local-first behavioral sensing for the "who are you?" axis.

Shared infrastructure lives in `core`: event schema, local storage, sensing primitives, and opt-in sync.

## Safety Defaults

- No message bodies are stored.
- No notification text is stored.
- No contact names are stored.
- Upload is opt-in only.
- Shield allows by default when local/kernel confidence is missing.

## Layout

```text
app/src/main/java/so/cynic/kyon/
  core/
    model/      normalized events
    sensing/    shared sensing contracts
    storage/    Room database
    sync/       CYNIC kernel client
  shield/
    calls/      CallScreeningService
    sms/        SMS receiver
    reporting/  user reports
  mirror/
    usage/      UsageStats collector
    notifications/ NotificationListenerService
    episodes/   behavior episode builder
    identity/   identity snapshots
  ui/           Compose shell
```

## Build

This scaffold expects a local Android SDK and a Gradle installation supplied by the
operator environment:

```bash
cd apps/kyon-mobile
gradle :app:assembleDebug
```

For Codex `/goal` iterations, use the repo-level validation gate:

```bash
make gate-0
```

Set `KYN_WITH_DEVICE=1` to include the connected-device notification probe after
unit tests and APK build.

## Portable deployment target

Kyon is structured as a portable framework:
- clean domain ports/contracts
- explicit bootstrap/adapters composition root
- runtime settings never hardcoded in source

Before adapting this app into a new repo, read:
- [Portable Architecture Contract](docs/portable-architecture.md)
- [Repo Portability Runbook](docs/repo-portability-runbook.md)
