# KYON — Component Specification v0

## 1. Purpose

Kyon is a personal local-first membrane for a single user.
It helps the user observe, understand, protect and document their own digital
experience without surveillance of another person and without a remote authority.

This v0 specification replaces the previous Kyon mobile scaffold.
It defines a smaller, coherent component before implementation.

## 2. First principles

1. The user owns the device, the data and the decisions.
2. The phone works offline as the primary authority.
3. No external service is required for protection.
4. Raw private content is never captured automatically.
5. Sync is an adapter, not a dependency.
6. Kyon advises; it does not command.
7. Hypotheses are never presented as facts.
8. Every alert is explainable.
9. Every action can be refused or undone.
10. No third party receives anything without explicit consent.

## 3. Non-goals

- Parental control.
- Remote monitoring.
- Surveillance of another person's communications.
- Automatic content extraction from social applications.
- Diagnosis of mental health.
- Automatic reporting to relatives.
- Global reputation scoring.
- Centralized user profiling.

## 4. Primary vertical

First human situation:

> Alice, 13-17 years old, is increasingly anxious and secretive around her phone.
> She may or may not be experiencing coercive or grooming behavior.
> Kyon must give her a safe, private space to understand what is happening,
> preserve autonomy, and ask for help if she chooses.

This vertical does not solve the case.
It gives Alice a private tool and a door out.

## 5. System model

Kyon v0 has four local parts:

- **Mirror**: local sensing of device-level signals.
- **Shield**: local decision policy and guidance.
- **Vault**: encrypted local storage of chosen evidence.
- **Assist**: help, education and documentation.

No Federation, no ZK, no on-chain anchoring in v0.

## 6. Mirror

Mirror observes only substrate-level facts:

- network state;
- battery/network availability;
- foreground app category if explicitly authorized by the user;
- time of day;
- local event count.

Mirror does not observe message content, contact names, or social-app
conversations.

Output: `MirrorSignal`.

```rust
pub struct MirrorSignal {
    pub generated_at_ms: u64,
    pub foreground_category: Option<ForegroundCategory>,
    pub connection_state: ConnectionState,
    pub late_night: bool,
    pub battery_low: bool,
}
```

## 7. Shield

Shield is pure logic.
It takes `MirrorSignal` and user-declared boundaries.
It returns a `ShieldDecision`.

```rust
pub enum ShieldDecisionKind {
    ObserveOnly,
    GuidanceShown,
    BoundaryReminder,
    TrustedContactPrompt,
    EvidenceSaved,
}

pub struct ShieldDecision {
    pub kind: ShieldDecisionKind,
    pub reason: &'static str,
    pub user_can_dismiss: bool,
    pub requires_consent: bool,
}
```

Shield never blocks another app.
Shield never reports anyone automatically.

## 8. Vault

Vault stores only what the user explicitly saves.

```rust
pub struct VaultItem {
    pub id: VaultItemId,
    pub created_at_ms: u64,
    pub item_type: VaultItemType,
    pub user_note: Option<String>,
    pub tag: VaultTag,
    pub encrypted: bool,
    pub exportable: bool,
}
```

Vault items may include:
- user-written note;
- user-selected screenshot or copied text;
- timestamp;
- platform label entered by the user;
- selected concern tag.

Vault items must not include:
- full conversation thread;
- contact identity;
- account identifier;
- hidden metadata beyond what the user chose.

## 9. Assist

Assist provides:
- boundary reminders selected by the user;
- scenario cards;
- refusal scripts;
- help navigation;
- evidence export on explicit user action.

Example boundary set:
- I do not send images.
- I stop when someone asks me to keep a secret.
- I do not click unknown links.
- I ask a trusted person before continuing with a new contact.

## 10. State machine

```text
IDLE
  -> signal observed -> GUIDANCE_AVAILABLE
GUIDANCE_AVAILABLE
  -> user dismisses -> IDLE
  -> user opens -> GUIDANCE_SHOWN
GUIDANCE_SHOWN
  -> user saves note -> VAULT_ITEM_CREATED
  -> user asks for help -> TRUSTED_CONTACT_PROMPT
  -> user exports -> EXPORT decision requires confirmation
TRUSTED_CONTACT_PROMPT
  -> user cancels -> IDLE
  -> user confirms -> assist flow opens
```

No state transition happens without user action except `GUIDANCE_AVAILABLE`.

## 11. Android adapter

Android components are adapters only:

- `KyonApp`: owns database and local configuration.
- `KyonDatabase`: Room database for local events and vault.
- `UsageSnapshotCollector`: optional local sensing behind one port.
- `KyonNotificationListenerService`: metadata-only adapter.
- `MainActivity`: thin UI, no domain policy.

The UI must not own policy tables.

## 12. Storage

Room is acceptable for v0, but only behind local ports:

- `EventPort`;
- `VaultPort`;
- `BoundaryPort`;
- `AssistPort`.

No cloud sync by default.

## 13. Sync

Sync is optional and user-owned.

Rules:
- disabled by default;
- no implicit scheduling;
- no production kernel URL hardcoded;
- user must explicitly enable and provide or discover endpoint;
- sync sends only explicitly marked items;
- at least one local confirmation before any outbound report.

## 14. Permissions

Request capability-scoped permissions only when used:

- usage stats only if user enables Mirror;
- notification listener only if user enables it;
- SMS/call handling only if user enables Shield;
- phone-state permissions only for call shielding if enabled.

No manifest-wide broad permission set by default.

## 15. Proof of correctness

v0 is complete when:

1. App launches offline.
2. User can set boundaries.
3. User can create a manual concern record.
4. Guidance is explainable and dismissible.
5. Vault stores only user-selected items.
6. Export requires explicit confirmation.
7. No raw content is captured automatically.
8. No network call occurs by default.
9. All local data can be deleted.
10. Build passes.

## 16. Implementation order

1. Define Rust types and pure predicates.
2. Define Room schema for events and vault.
3. Implement local ports.
4. Implement Android adapters.
5. Implement UI for boundaries and concern intake.
6. Implement guidance cards.
7. Implement vault and export.
8. Add JVM tests for pure policy and repository mapping.
9. Build debug APK.
10. Manual walkthrough of the primary vertical.

## 17. Open questions

- Should local policy live in `cynic-protocol` or a Kyon-specific no_std crate?
- Should the first Android target be Kotlin-only with Rust policy compiled later?
- What is the minimum education content needed for v0?
- What exact consent text is required before export?
- How should deletion of vault items be proven?
