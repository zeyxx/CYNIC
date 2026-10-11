# Security Policy

## Supported Versions

Only the current main development branch and the latest released tags receive security fixes.

| Version | Supported          |
| ------- | ------------------ |
| 0.8.x   | :white_check_mark: |
| < 0.8.0 | :x:                |

---

## Reporting a Vulnerability

The CYNIC collective takes security and epistemic integrity seriously. If you discover a vulnerability or security flaw, please disclose it responsibly.

### How to Report

**Do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report security vulnerabilities via one of the following channels:

1. **GitHub Private Vulnerability Reporting**: Use the "Report a vulnerability" button in the repository's Security tab on GitHub.
2. **Security Contact**: Send an encrypted or direct email to `security@cynic.local` or contact the lead maintainer directly via GitHub profile details.

### What to Include

Please provide:
- A detailed description of the vulnerability.
- Steps to reproduce the issue (including test code, minimal examples, or targeted argv commands).
- Affected crates, modules, or network planes.
- Potential impact and threat model.

### Response Timeline

- **Initial Response**: Within 48 hours acknowledging receipt of the report.
- **Triage & Assessment**: Within 5 business days with an assessment and preliminary remediation plan.
- **Fix & Public Advisory**: Coordinated public release once a fix is verified against our canonical release gates.

---

## Secret Governance Policy

- **Zero Credentials in Git**: CYNIC enforces automated pre-commit and CI scans (`cynic audit secret`) to ensure no API keys, private keys, or tokens are committed.
- **Runtime Injection**: All secrets must be injected at runtime via environment variables, principal-owned files (`mode 0640`), or OS keyrings behind `SecretProviderPort`.
