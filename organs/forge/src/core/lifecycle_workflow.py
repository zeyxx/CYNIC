"""Lifecycle workflow helpers for organ-forge.

This module keeps lifecycle payload, evidence, and activation logic separate
from the transport/client adapter so the adapter can stay focused on kernel I/O.
"""

from __future__ import annotations

from .lifecycle_payloads import (
    build_lifecycle_profile_payloads as _payload_build_lifecycle_profile_payloads,
    build_lifecycle_task_payload as _payload_build_lifecycle_task_payload,
    lifecycle_profile_intents as _payload_lifecycle_profile_intents,
    validate_lifecycle_task_payload as _payload_validate_lifecycle_task_payload,
)
from .lifecycle_runtime import (
    build_lifecycle_runtime_paths as _runtime_build_lifecycle_runtime_paths,
    verify_lifecycle_runtime_paths as _runtime_verify_lifecycle_runtime_paths,
)
from .lifecycle_evidence import (
    activate_lifecycle_registry_node as _evidence_activate_lifecycle_registry_node,
    apply_lifecycle_activation_evidence_patch as _evidence_apply_lifecycle_activation_evidence_patch,
    build_lifecycle_service_evidence_patch as _evidence_build_lifecycle_service_evidence_patch,
    persist_lifecycle_activation_evidence_patch as _evidence_persist_lifecycle_activation_evidence_patch,
    persist_lifecycle_registry_node_activation as _evidence_persist_lifecycle_registry_node_activation,
)
from .lifecycle_activation import (
    build_lifecycle_activation_plan as _activation_build_lifecycle_activation_plan,
)
from .lifecycle_activation_requests import (
    build_lifecycle_activation_requests as _activation_build_lifecycle_activation_requests,
    dispatch_lifecycle_activation_requests as _activation_dispatch_lifecycle_activation_requests,
    validate_lifecycle_activation_request_payload as _activation_validate_lifecycle_activation_request_payload,
)


def lifecycle_profile_intents(
    profile: str,
    registry: dict | None = None,
) -> list[str]:
    return _payload_lifecycle_profile_intents(profile, registry)


def build_lifecycle_task_payload(
    intent: str,
    domain: str,
    agent_id: str,
    registry: dict | None = None,
) -> dict:
    return _payload_build_lifecycle_task_payload(
        intent=intent,
        domain=domain,
        agent_id=agent_id,
        registry=registry,
    )


def validate_lifecycle_task_payload(
    payload: dict,
    registry: dict | None = None,
) -> dict:
    return _payload_validate_lifecycle_task_payload(payload, registry)


def build_lifecycle_profile_payloads(
    profile: str,
    domain: str,
    agent_id: str,
    registry: dict | None = None,
) -> dict:
    return _payload_build_lifecycle_profile_payloads(
        profile=profile,
        domain=domain,
        agent_id=agent_id,
        registry=registry,
    )


def build_lifecycle_activation_plan(
    registry: dict | None = None,
    inspect_fleet: bool = False,
) -> dict:
    return _activation_build_lifecycle_activation_plan(
        registry=registry,
        inspect_fleet=inspect_fleet,
    )


def build_lifecycle_activation_requests(
    registry: dict | None = None,
    inspect_fleet: bool = False,
) -> dict:
    return _activation_build_lifecycle_activation_requests(
        registry=registry,
        inspect_fleet=inspect_fleet,
    )


def validate_lifecycle_activation_request_payload(
    payload: dict,
    registry: dict | None = None,
) -> dict:
    return _activation_validate_lifecycle_activation_request_payload(payload, registry)


def dispatch_lifecycle_activation_requests(
    registry: dict | None = None,
    inspect_fleet: bool = False,
    kernel_client_factory=None,
) -> dict:
    return _activation_dispatch_lifecycle_activation_requests(
        registry=registry,
        inspect_fleet=inspect_fleet,
        kernel_client_factory=kernel_client_factory,
    )


def build_lifecycle_runtime_paths(registry: dict | None = None) -> dict:
    return _runtime_build_lifecycle_runtime_paths(registry)


def verify_lifecycle_runtime_paths(
    node_name: str,
    registry: dict | None = None,
    home: str | None = None,
) -> dict:
    return _runtime_verify_lifecycle_runtime_paths(node_name, registry, home)


def build_lifecycle_service_evidence_patch(
    node_name: str,
    registry: dict | None = None,
    service_states: dict | None = None,
) -> dict:
    return _evidence_build_lifecycle_service_evidence_patch(
        node_name,
        registry=registry,
        service_states=service_states,
    )


def apply_lifecycle_activation_evidence_patch(
    patch: dict,
    registry: dict | None = None,
) -> dict:
    return _evidence_apply_lifecycle_activation_evidence_patch(patch, registry)


def persist_lifecycle_activation_evidence_patch(
    patch: dict,
    registry_path: str | Path | None = None,
) -> dict:
    return _evidence_persist_lifecycle_activation_evidence_patch(
        patch,
        registry_path=registry_path,
    )


def activate_lifecycle_registry_node(
    node_name: str,
    registry: dict | None = None,
) -> dict:
    return _evidence_activate_lifecycle_registry_node(node_name, registry)


def persist_lifecycle_registry_node_activation(
    node_name: str,
    registry_path: str | Path | None = None,
) -> dict:
    return _evidence_persist_lifecycle_registry_node_activation(
        node_name,
        registry_path=registry_path,
    )


def lifecycle_activation_requests_payload(
    registry: dict | None = None,
    inspect_fleet: bool = False,
) -> dict:
    return build_lifecycle_activation_requests(registry=registry, inspect_fleet=inspect_fleet)


def dispatch_lifecycle_activation_requests_via_client(
    registry: dict | None = None,
    inspect_fleet: bool = False,
    kernel_client_factory=None,
) -> dict:
    return dispatch_lifecycle_activation_requests(
        registry=registry,
        inspect_fleet=inspect_fleet,
        kernel_client_factory=kernel_client_factory,
    )
