"""Map Odysseum boundary drift across real repo surfaces."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import re
from typing import Any


SURFACES = {
    "odysseum_manifest": [
        Path("docs/foundation/ODYSSEUM_MANIFEST.md"),
        Path("axioms/odysseum-manifest-v1.json"),
    ],
    "odysseum_internal_narrative": [
        Path("docs/internal/ODYSSEUM_FUTARDIO_WHITEPAPER.md"),
        Path("docs/internal/STOCKS_ON_SOLANA_GROUND_TRUTH.md"),
        Path("docs/ODYSSEUM_PITCH_WAKEUP_2026-06-30.md"),
    ],
    "odysseum_public_ui": [
        Path("packages/odysseum-ui/README.md"),
        Path("packages/odysseum-ui/src"),
    ],
    "talaria_public_surface": [
        Path("infra/talaria/runtime-surfaces.json"),
        Path("packages/public-ui/src"),
        Path("apps/talaria-terminal"),
    ],
    "solana_library_surface": [
        Path("organs/solana"),
        Path("axioms/pump_mechanisms.json"),
        Path("axioms/canonical_pumpswap_ssot.json"),
        Path("axioms/pumpfun_fee_tiers.json"),
    ],
    "percolator_bounty_surface": [
        Path("organs/percolator"),
    ],
    "rag_memory_surface": [
        Path("organs/rag"),
        Path("crates/cynic-inference/src/rag.rs"),
    ],
}

BOUNDARY_PATTERNS = {
    "market_hype_or_unverified_projection": (
        "licorne",
        "x100",
        "mandatory",
        "millions de dollars",
        "96 Millions",
        "certifiés directement",
        "prouvent",
        "Wall Street",
    ),
    "protocol_business_or_legal_mix": (
        "TALARIA SAS",
        "invoices",
        "contracts",
        "enterprise intake",
        "equity",
        "Caïmans",
    ),
    "odysseum_scope_inflation": (
        "Stocks on Solana",
        "Options on Solana",
        "M&A",
        "OPA",
        "Conglomerate",
        "financial OS",
        "Mobile-First Wallet",
        "Privacy L2",
    ),
    "solana_mechanism_assumption": (
        "sharing_config",
        "natively hooks",
        "0.95%",
        "Creator Fees",
        "Token Extensions",
        "Pump.fun",
        "PumpSwap",
    ),
    "percolator_odysseum_mix": (
        "ODYSSEUM",
        "Odysseum",
        "odysseum",
        "pumpswap_pitch",
        "odysseum_monte_carlo",
        "ARR B2B",
    ),
    "rag_memory_mix": (
        "store.json",
        "local retrieval state",
        "RAG BUNDLE",
    ),
}

EXPECTED_BOUNDARIES = {
    "odysseum_manifest": "stable doctrine and public/product boundary only",
    "odysseum_internal_narrative": "internal pitch and research; may be aggressive but must not become machine authority",
    "odysseum_public_ui": "public control surface; should expose verification/intake without overclaiming protocol mechanics",
    "talaria_public_surface": "commercial/legal/public Talaria surface, not protocol custody",
    "solana_library_surface": "mechanism sensing and observations; not Odysseum product ownership",
    "percolator_bounty_surface": "security bounty workflow; not Odysseum market research home",
    "rag_memory_surface": "retrieval over Kernel/library memory; not source of truth",
}


@dataclass(frozen=True)
class BoundaryFinding:
    severity: str
    surface: str
    path: str
    kind: str
    evidence: str
    recommended_action: str


def build_odysseum_boundary_audit(root: Path) -> dict[str, Any]:
    findings: list[BoundaryFinding] = []
    scanned: dict[str, list[str]] = {}

    for surface, roots in SURFACES.items():
        paths = _expand_paths(root, roots)
        scanned[surface] = [_relative(root, path) for path in paths]
        for path in paths:
            findings.extend(_scan_file(root, surface, path))

    node_modules = root / "packages" / "odysseum-ui" / "node_modules"
    if node_modules.exists():
        findings.append(
            BoundaryFinding(
                severity="critical",
                surface="odysseum_public_ui",
                path="packages/odysseum-ui/node_modules/",
                kind="generated_dependency_inside_source_surface",
                evidence="node_modules exists inside the Odysseum UI package",
                recommended_action="remove generated dependencies from the source tree and rely on package manifests",
            )
        )

    findings.extend(_cross_surface_findings(root))
    ordered = sorted(findings, key=lambda item: (_severity_rank(item.severity), item.surface, item.path, item.kind))

    return {
        "schema": "cynic.odysseum_boundary_audit.v1",
        "status": "advisory",
        "principle": "Odysseum is a product/protocol standard. Solana is the market and mechanism substrate, Percolator is a bounty workflow, RAG is retrieval, and Talaria SAS is the business surface.",
        "counts": {
            "surfaces": len(SURFACES),
            "files_scanned": sum(len(paths) for paths in scanned.values()),
            "findings": len(ordered),
            "critical": sum(1 for item in ordered if item.severity == "critical"),
            "high": sum(1 for item in ordered if item.severity == "high"),
            "medium": sum(1 for item in ordered if item.severity == "medium"),
            "low": sum(1 for item in ordered if item.severity == "low"),
        },
        "expected_boundaries": EXPECTED_BOUNDARIES,
        "scanned": scanned,
        "findings": [item.__dict__ for item in ordered],
    }


def _expand_paths(root: Path, paths: list[Path]) -> list[Path]:
    expanded: list[Path] = []
    for rel in paths:
        path = root / rel
        if not path.exists():
            continue
        if path.is_file():
            expanded.append(path)
            continue
        expanded.extend(
            item
            for item in path.rglob("*")
            if item.is_file()
            and "__pycache__" not in item.parts
            and "node_modules" not in item.parts
            and "dist" not in item.parts
        )
    return sorted(expanded)


def _scan_file(root: Path, surface: str, path: Path) -> list[BoundaryFinding]:
    rel = _relative(root, path)
    text = path.read_text(encoding="utf-8", errors="replace")
    findings: list[BoundaryFinding] = []

    for kind, patterns in BOUNDARY_PATTERNS.items():
        hits = [pattern for pattern in patterns if _contains_pattern(text, pattern)]
        if not hits:
            continue
        severity = _severity(surface, kind)
        if _is_explicitly_non_normative_odysseum_axiom(rel, kind, text):
            severity = "ignore"
        if severity == "ignore":
            continue
        findings.append(
            BoundaryFinding(
                severity=severity,
                surface=surface,
                path=rel,
                kind=kind,
                evidence=", ".join(sorted(hits)[:5]),
                recommended_action=_recommendation(surface, kind),
            )
        )

    return findings


def _cross_surface_findings(root: Path) -> list[BoundaryFinding]:
    findings: list[BoundaryFinding] = []
    axiom = root / "axioms" / "odysseum-manifest-v1.json"
    manifest = root / "docs" / "foundation" / "ODYSSEUM_MANIFEST.md"
    if axiom.exists() and manifest.exists():
        axiom_text = axiom.read_text(encoding="utf-8", errors="replace")
        manifest_text = manifest.read_text(encoding="utf-8", errors="replace")
        non_normative_claims = _non_normative_odysseum_claims(axiom_text)
        for claim in ("Stocks on Solana", "Options on Solana", "M&A", "Zero-Human"):
            if claim in non_normative_claims:
                continue
            if claim.lower() in axiom_text.lower() and claim.lower() not in manifest_text.lower():
                findings.append(
                    BoundaryFinding(
                        severity="high",
                        surface="odysseum_manifest",
                        path="axioms/odysseum-manifest-v1.json",
                        kind="axiom_exceeds_manifest_authority",
                        evidence=claim,
                        recommended_action="downgrade unsupported machine claims or promote them through the manifesto with evidence",
                    )
                )
    return findings


def _is_explicitly_non_normative_odysseum_axiom(rel: str, kind: str, text: str) -> bool:
    return (
        rel == "axioms/odysseum-manifest-v1.json"
        and kind == "odysseum_scope_inflation"
        and '"research_inputs_not_normative"' in text
    )


def _non_normative_odysseum_claims(text: str) -> set[str]:
    try:
        payload = json.loads(text)
    except json.JSONDecodeError:
        return set()
    claims: set[str] = set()
    for item in payload.get("research_inputs_not_normative", []):
        if not isinstance(item, dict):
            continue
        identifier = str(item.get("id", ""))
        rule = str(item.get("rule", ""))
        if identifier == "stocks_on_solana":
            claims.add("Stocks on Solana")
        if identifier == "options_on_solana":
            claims.add("Options on Solana")
        if identifier == "conglomerate_mechanisms":
            claims.add("M&A")
        if identifier == "zero_human_layer":
            claims.add("Zero-Human")
    return claims


def _severity(surface: str, kind: str) -> str:
    if kind == "percolator_odysseum_mix" and surface != "percolator_bounty_surface":
        return "ignore"
    if surface == "odysseum_public_ui" and kind in {
        "market_hype_or_unverified_projection",
        "odysseum_scope_inflation",
        "protocol_business_or_legal_mix",
    }:
        return "high"
    if surface == "odysseum_manifest" and kind in {"market_hype_or_unverified_projection", "odysseum_scope_inflation"}:
        return "high"
    if surface == "percolator_bounty_surface" and kind == "percolator_odysseum_mix":
        return "high"
    if surface == "solana_library_surface" and kind == "solana_mechanism_assumption":
        return "low"
    if surface == "odysseum_internal_narrative":
        return "medium"
    if surface == "rag_memory_surface" and kind == "rag_memory_mix":
        return "medium"
    if surface in {"talaria_public_surface", "odysseum_public_ui"} and kind == "protocol_business_or_legal_mix":
        return "medium"
    return "ignore"


def _contains_pattern(text: str, pattern: str) -> bool:
    if pattern.isupper() and pattern.isalpha() and len(pattern) <= 4:
        return re.search(rf"(?<![A-Za-z0-9_]){re.escape(pattern)}(?![A-Za-z0-9_])", text) is not None
    return pattern.lower() in text.lower()


def _recommendation(surface: str, kind: str) -> str:
    if kind == "market_hype_or_unverified_projection":
        return "keep as pitch material only until backed by Kernel observations, source datasets and reproducible calculations"
    if kind == "protocol_business_or_legal_mix":
        return "split Talaria SAS commercial authority from Odysseum protocol authority"
    if kind == "odysseum_scope_inflation":
        return "downgrade to roadmap or internal thesis unless it is in the v0 contract boundary"
    if kind == "solana_mechanism_assumption":
        return "treat as research input; verify through organ-solana observations and Rust inference before contract design"
    if kind == "percolator_odysseum_mix":
        return "move Odysseum market research out of organ-percolator; Percolator should remain bounty/security workflow"
    if kind == "rag_memory_mix":
        return "ensure RAG reads Kernel/library memory and does not become the durable source"
    return "review boundary"


def _severity_rank(severity: str) -> int:
    return {"critical": 0, "high": 1, "medium": 2, "low": 3}.get(severity, 4)


def _relative(root: Path, path: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def audit_json(root: Path) -> str:
    return json.dumps(build_odysseum_boundary_audit(root), indent=2, sort_keys=True)
