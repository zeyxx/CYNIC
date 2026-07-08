# Organ Browser — Specification

## 1. Biological Role & Identity
- **Name**: `organ-browser`
- **Instance**: `organ-browser-systemd`
- **Role**: Web Perception and Interaction Subsystem
- **Scope**: External environment scanning, opportunity discovery, evidence verification

In the CYNIC biological paradigm, `organ-browser` acts as the sensory organs (eyes, ears, tactile feedback) of the organism. It allows the episodic cortex or persistent agents to query the external web, scrape known platforms (like Superteam Earn, Twitter, Discord), and interact with web pages to verify claims or prepare draft submissions without compromising the organism's memory or security.

## 2. Decoupled Architecture (Hexagonal Ports & Adapters)

To ensure strict decoupling, all I/O, network requests, and external APIs are abstracted behind interfaces (Ports), implemented by specific Adapters.

### Hermes Agent Bootstrap

`organ-browser` uses `NousResearch/hermes-agent` as its upstream bootstrap
framework for browser-capable agent tooling. Hermes Agent is not the CYNIC
kernel and does not own organism memory. It provides the browser/tool substrate
that CYNIC adapts behind policy ports.

See `HERMES_BOOTSTRAP.md` for the binding contract.

### A. Ports
- `SearchPort`: Decouples query execution and search discovery.
- `ScrapePort`: Decouples raw HTML/markdown extraction.
- `BrowserPort`: Decouples direct CDP/Playwright browser automation.
- `HermesBrowserBackendPort`: Decouples Hermes-compatible browser tools such as
  navigate, snapshot, click, type, scroll, console, CDP, and dialog handling.
- `EvidenceStorePort`: Decouples saving evidence bundles (traces, redacted screenshots).
- `SecretScannerPort`: Decouples sanitization and verification that no sensitive data (cookies, private keys) escapes.
- `KernelObservationPort`: Decouples dispatching structured discoveries to the CYNIC coordination mempool.

### B. Adapters
- `SearXNGAdapter`: Connects to local/configured SearXNG instances.
- `FirecrawlAdapter`: Connects to Firecrawl API or triggers local python fallback extraction.
- `BrowserHubAdapter`: Connects to the running `organ-browser-hub.service` instance.
- `HermesAgentBrowserAdapter`: Target adapter over Hermes Agent browser tooling
  (`agent-browser`, CDP supervisor, Camofox, or cloud backends when explicitly
  enabled). The first local implementation maps Hermes-compatible
  `browser_navigate` and `browser_snapshot` calls to `organ-browser-hub`; other
  declared Hermes tools remain blocked until adapted.
- `PlaywrightAdapter`: Uses local `playwright` framework for direct CDP operations.
- `FilesystemEvidenceStore`: Writes logs and outputs strictly to `organs/browser/state/`.
- `CynicKernelObservationAdapter`: Sends observations via MCP or REST to the Tailscale CYNIC kernel.

## 3. Policy & Safety Layers

Safety is enforced sequentially across four explicit logical layers:

| Layer | Actions Allowed | Constraints | Mutation Risk |
|---|---|---|---|
| **Search** | Querying, URL discovery, collecting snippet metadata | Public sources only. Zero credential leakage. | None |
| **Extract** | Scrape known URLs, convert HTML to clean markdown, parse page metadata | Read-only. Redact all potential cookies, access tokens, and private identifiers. | Low |
| **Interact** | Navigate, click, scroll, probe forms, take screenshots | Strictly read-only browsing patterns. Blocks submit events. | Medium |
| **Mutate** | Submit forms, post content, sign transactions, connect wallets | **STRICT HUMAN GATE REQUIRED**. Any action must verify presence of approved `human_gate` config. | High |

Hermes-compatible tools inherit this policy. A tool can exist upstream and still
remain unavailable through CYNIC MCP until its risk layer is explicitly allowed.

## 4. Secret Scanner & Redaction Rules

Before any evidence is written to disk or sent to the kernel:
- All cookies, session tokens, passwords, and private API keys are parsed and replaced with standard redaction flags (`[REDACTED_API_KEY]`).
- Raw authenticated HTML dumps are strictly forbidden. Only clean, text-only markdown fragments are exported.
- Images/screenshots are stored locally in the organ's state directory (`organs/browser/state/evidence/`) and never committed to Git.

## 5. Structural Layout

```
organs/browser/
├── MANIFEST.yaml           # Metadata & topological entry
├── ORGAN_SPEC.md           # Biological spec & contracts
├── pyproject.toml          # Sovereign python project workspace
├── contracts/              # JSON schemas for Tasks & Results
│   ├── browser_task.schema.json
│   └── browser_result.schema.json
├── src/
│   ├── __init__.py
│   ├── core/               # Domain logic (entities, ports, services)
│   │   ├── __init__.py
│   │   ├── entities.py
│   │   ├── ports.py
│   │   └── services.py
│   ├── adapters/           # Specific I/O implementations
│   │   ├── __init__.py
│   │   ├── searxng_adapter.py
│   │   ├── firecrawl_adapter.py
│   │   ├── browser_hub_adapter.py
│   │   ├── filesystem_evidence_store.py
│   │   └── kernel_observation_adapter.py
│   └── entrypoints/        # Execution endpoints (CLI, MCP, Hub daemon)
│       ├── __init__.py
│       ├── browser_hub.py
│       ├── mcp_server.py
│       └── cli.py
└── state/                  # STRICT MEMORY membrane (ignored by Git)
    ├── evidence/           # Redacted logs and screenshot bundles
    └── action_ledger.jsonl # Append-only action records
```
