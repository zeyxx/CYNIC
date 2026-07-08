"""Static L0-L7 organism stack contract constants."""

from __future__ import annotations


LAYERS = [
    {
        "id": "L0",
        "name": "substrate",
        "unit": "node",
        "function": "physical or virtual host, disk, clock, operating system",
        "sync_contract": ["heartbeat", "provider_evidence", "clock"],
    },
    {
        "id": "L1",
        "name": "membrane",
        "unit": "runtime_principal",
        "function": "users, permissions, environment, filesystem boundary",
        "sync_contract": ["ownership_probe", "permission_probe", "environment_projection"],
    },
    {
        "id": "L2",
        "name": "synapse",
        "unit": "endpoint",
        "function": "mesh reachability, DNS, IPAM, service discovery",
        "sync_contract": ["dns_resolution", "route_probe", "service_discovery"],
    },
    {
        "id": "L3",
        "name": "cell",
        "unit": "process_or_binary",
        "function": "daemon, executable, venv, healthcheckable runtime cell",
        "sync_contract": ["process_health", "binary_freshness", "dependency_import"],
    },
    {
        "id": "L4",
        "name": "organ",
        "unit": "bounded_subsystem",
        "function": "manifest-backed capability owner",
        "sync_contract": ["manifest", "capability", "state_boundary"],
    },
    {
        "id": "L5",
        "name": "circulation",
        "unit": "signal",
        "function": "Kernel tasks, claims, observations, read-back",
        "sync_contract": ["task", "claim", "observation", "read_back"],
    },
    {
        "id": "L6",
        "name": "soma",
        "unit": "convergence_action",
        "function": "desired versus observed reconciliation and remediation",
        "sync_contract": ["diff", "decision", "dispatch", "proof"],
    },
    {
        "id": "L7",
        "name": "cortex",
        "unit": "intent",
        "function": "human and cortex agents mutating organism DNA",
        "sync_contract": ["intent", "claim", "commit", "handoff"],
    },
]


ASYNC_PRIMITIVES = [
    "event",
    "command",
    "task",
    "lease",
    "claim",
    "heartbeat",
    "deadline",
    "retry_policy",
    "idempotency_key",
    "causal_parent",
    "result",
    "read_back",
]
