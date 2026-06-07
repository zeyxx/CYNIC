# Talaria Observatory UI

Current package path: `packages/cynic-ui`.

This is the current technical home of the Talaria Observatory. The package name is historical and provisional; it is expected to be renamed later to `packages/talaria-observatory`.

## Role

The app is the entity-facing observatory for Talaria as a whole:

- Talaria proof-of-work team timeline
- Talaria proof-of-humanity user states
- B&C chess organism signals
- CYNIC judgments and oracle traces
- MetaDAO/futarchy proposal context
- governance review state
- incidents and remediation
- reputation/alignment public views

CYNIC remains the backend engine and canonical registry. This UI presents selected CYNIC/Talaria state.

## Current status

The app currently contains an early CYNIC Observatory shell with:

- `TIMELINE`
- `TOPOLOGY`
- `ORACLE`

Talaria Observatory should emerge here first before creating a separate public app.

## Naming decision

Do not rename the package yet.

Rename later only when these references can be updated in a controlled pass:

- root `package.json` scripts
- Vercel/project links
- docs
- imports
- deployment scripts
- operator habits/runbooks

Expected rename:

```txt
packages/cynic-ui -> packages/talaria-observatory
```

## Boundaries

This UI should not own business truth. It reads/project Talaria state from CYNIC.

B&C remains its own chess organism. The observatory includes B&C signals as part of Talaria, but does not absorb B&C product ownership.
