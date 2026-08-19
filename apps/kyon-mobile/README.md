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

This scaffold expects a local Android SDK and Gradle installation:

```bash
cd apps/kyon-mobile
gradle :app:assembleDebug
```

No Gradle wrapper is committed yet.
