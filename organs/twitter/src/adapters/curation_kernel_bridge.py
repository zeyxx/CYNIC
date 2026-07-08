"""Kernel observation bridge for curated Hermes X signals."""

import os

from organs.twitter.src.adapters.kernel_observe_transport import post_observe_payload


BRIDGE_TOP_N = 3


def post_to_kernel(signals_by_domain: dict[str, list[dict]], top_n: int = BRIDGE_TOP_N) -> int:
    """POST top-N curated signals per domain to kernel /observe."""
    addr = os.environ.get("CYNIC_REST_ADDR", "")
    key = os.environ.get("CYNIC_API_KEY", "")
    if not addr or not key:
        print("CYNIC_REST_ADDR or CYNIC_API_KEY not set - bridge skipped")
        return 0
    posted = 0
    errors = 0

    for domain, signals in sorted(signals_by_domain.items()):
        top = sorted(signals, key=lambda signal: signal.get("strength", 0), reverse=True)[:top_n]

        for signal in top:
            result = post_observe_payload(
                {
                    "tool": "hermes-curation",
                    "target": signal.get("signal_id", "unknown"),
                    "domain": domain,
                    "context": signal.get("pattern", "")[:200],
                    "agent_id": "hermes-organ",
                    "tags": ["hermes-curated", f"domain-{domain}"],
                },
                timeout=5,
            )
            if result.posted:
                posted += 1
            else:
                errors += 1
                if errors <= 2:
                    print(f"Bridge POST failed for {signal.get('signal_id', '?')}: {result.error or result.status}")

    if posted > 0:
        print(f"Bridge: {posted} signals -> kernel /observe ({errors} errors)")
    elif errors > 0:
        print(f"Bridge: 0 posted, {errors} errors (kernel may be down)")
    return posted
