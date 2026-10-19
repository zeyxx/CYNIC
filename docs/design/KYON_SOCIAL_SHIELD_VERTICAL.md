# KYON Social Shield — Minimal Vertical

## Status

Design proposal. Local-first, consent-based, anti-grooming without surveillance.

## Goal

Help a young person recognize, slow down, refuse and document uncomfortable
digital interactions started by unknown or manipulative contacts, without
Kyon reading private conversations automatically or exposing data to another
person.

## Scope

- Social applications such as Snapchat, TikTok, WhatsApp, Instagram, Telegram.
- No automatic content capture from those applications.
- No cross-app semantic analysis by default.
- No remote parent, sibling or investigator dashboard.
- No hidden monitoring.

## Threats addressed

- First contact from an unknown person.
- Requests for secrecy.
- Requests for images or sexual content.
- Pressure, guilt, urgency or emotional coercion.
- Links to unsafe sites or unknown payment/wallet flows.
- Isolation from friends, family or trusted adults.

## Non-goals

- Reading private messages from social apps.
- Intercepting notifications from other apps beyond metadata already available.
- Scoring a person's morality, mental health or reliability.
- Automatically reporting to a relative.
- Replacing emergency services, schools or child-protection professionals.

## Privacy invariants

1. The app belongs to the user.
2. The app never reads social-app content without explicit user action.
3. The user can ignore, delete or uninstall at any time.
4. Raw content is never sent to a relative, server, or analyst.
5. Shared evidence exists only after explicit consent.
6. Metadata is enough for local risk prompts.
7. The app never diagnoses.

## Product surface

### 1. First-contact shield

 Trigger:
- User receives an SMS from a new sender.
- User manually opens Kyon after an uncomfortable interaction.
- User pastes or shares a suspicious message into Kyon.

 Output:
- Short guidance card.
- Non-shaming language.
- Suggested next action chosen by the user.

### 2. Boundary coach

 The user can declare their own boundaries:
- I do not send images.
- I do not click unknown links.
- I stop when someone asks me to keep a secret.
- I ask a trusted person before continuing a new contact.

 Kyon stores these locally and surfaces them at the moment of risk.

### 3. Local evidence locker

 User-controlled, encrypted storage on device:
- Note written by the user.
- A screenshot or copied text added manually.
- Platform, date, and a short tag such as "secrecy", "image request", or "pressure".
- No full conversation required.
- Optional export as a shareable package, only after user confirmation.

### 4. Panic and trusted-contact path

- Select a trusted adult in advance.
- Quick actions:
  - "I need advice now."
  - "Help me draft a message."
  - "I want to save this, but not send it yet."
- Kyon never sends anything automatically.

### 5. Education cards

Short, reusable scenarios:
- "Someone asks you not to tell anyone."
- "Someone sends many compliments very quickly."
- "Someone asks for a photo."
- "Someone tries to move you to another app."

Each card gives:
- What it is.
- Why it is risky.
- Three possible actions.
- A sentence the user can send to refuse.

## Proposed Android modules

- `core/model`: ActivityEvent, ShieldSignal, Boundary, EvidenceItem.
- `shield/social`: manual signal intake, boundary coaching, guidance.
- `vault`: encrypted local evidence locker and export.
- `ui`: onboarding, first-contact card, boundary setup, evidence review, panic path.
- `sync` (optional): only user-owned, opt-in, explicit consent.

## Data model

Minimum fields:
- signalId
- createdAtMs
- platformLabel (user-provided or selected)
- signalType: FIRST_CONTACT_UNKNOWN, SECRECY_REQUEST, IMAGE_REQUEST, PRESSURE, UNKNOWN_LINK, OTHER
- userSummary (optional, user-written)
- boundary reminder shown
- hasEvidence: boolean
- syncEligible: false by default

No message body, no contact name, no account ID, no screenshot unless explicitly added by the user.

## User flow

1. User opens Kyon.
2. Sets personal boundaries.
3. Adds a trusted contact.
4. Receives an uncomfortable message elsewhere.
5. Returns to Kyon and logs a tag or note.
6. Kyon shows guidance based on the tag.
7. User decides:
   - ignore;
   - save locally;
   - prepare an alert for trusted contact;
   - export evidence;
   - seek professional help.

## Optional local semantic assistance

If useful, semantic analysis may run only after the user voluntarily pastes or
shares content into Kyon. It must remain on-device where feasible. It produces:
- signal type;
- suggested boundary;
- optional short user review.

It never:
- sends raw content to a relative;
- stores content without user consent;
- replaces user judgment.

## Safety defaults

- No passive surveillance.
- No automatic reporting.
- No content capture.
- No notification text storage.
- No contact names.
- Allow by default for risky interactions: Kyon advises, does not block the app.
- The user owns the decision.

## Proof of respect for user authority

A successful vertical proves:
- no content was captured without explicit user action;
- no data was shared without explicit user consent;
- every alert is explainable;
- the user can delete all local data;
- the app still works offline.

## First implementation slice

1. Onboarding with boundaries and trusted-contact selection.
2. Manual "log a concern" flow.
3. Guidance cards by concern type.
4. Encrypted local evidence locker.
5. Optional user-triggered evidence export.

No parent dashboard. No automatic social-app inspection. No semantic pipeline outside the user's explicit action.
