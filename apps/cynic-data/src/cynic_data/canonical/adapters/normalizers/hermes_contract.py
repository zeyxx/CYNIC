"""Hermes conservation contract — anchored on the producer's own schema.

Source of truth: ``hermes_state_common.SCHEMA_SQL`` at ``SCHEMA_VERSION = 31``
(in the Hermes-Agent tree). Every column of ``sessions`` and ``messages`` has
exactly one declared fate. When Hermes bumps the schema version or adds a
column, ``test_hermes_conservation`` fails until this file is updated — drift
becomes a test failure, never a silent loss.

Two facts drove this contract, both measured on the live store:

* **Reasoning has four homes, chosen by the MODEL, not the harness.**
  ``reasoning``/``reasoning_content`` carry plain CoT for most models;
  ``codex_reasoning_items`` carries *encrypted* CoT for codex-family models
  (gpt-5.3-codex-spark: 3 914 messages with reasoning=0);
  ``reasoning_details`` carries structured text spans. Reading only
  ``reasoning_content`` loses 8 462 messages outright.
* **Compaction rewrites history in place.** ``active=0`` marks a message that a
  compaction replaced (173 696 rows on this store). Reading them without the
  flag conflates live history with superseded history.
"""

from __future__ import annotations

from ...domain.conservation import ConceptResolution, ConservationContract

PRODUCER = "hermes-agent"
PRODUCER_SCHEMA_VERSION = 31
PRODUCER_SCHEMA_SOURCE = "hermes-agent/hermes_state_common.py:SCHEMA_SQL"

# ---------------------------------------------------------------------------
# messages (30 columns)
# ---------------------------------------------------------------------------

MESSAGES_FATE: dict[str, str] = {
    "id": "preserved: turn.native_fields.native_id (ordering + native id)",
    "session_id": "preserved: session_uid",
    "role": "preserved: turn.role",
    "content": "preserved: turn.content",
    "tool_call_id": "preserved: tool.call_id (result -> call link)",
    "tool_calls": "preserved: tool.command_or_args + TOOL_CALL edge",
    "tool_name": "preserved: tool.native_fields.tool_name",
    "effect_disposition": "preserved: turn.native_fields.effect_disposition",
    "timestamp": "preserved: turn.timestamp_iso",
    "token_count": "declared: unpopulated in the producer (0 non-null rows)",
    "finish_reason": "preserved: turn.native_fields.finish_reason",
    "reasoning": "preserved: reasoning.content (via concept 'reasoning')",
    "reasoning_content": "preserved: reasoning.content (via concept 'reasoning')",
    "reasoning_details": "preserved: reasoning.native_fields.reasoning_details",
    "codex_reasoning_items": "preserved: reasoning.native_fields.codex_reasoning_items (encrypted CoT; presence and bytes kept, never decoded)",
    "codex_message_items": "preserved: turn.native_fields.codex_message_items",
    "platform_message_id": "preserved: turn.native_fields.platform_message_id",
    "observed": "preserved: turn.native_fields.observed",
    "_compressed_summary": "preserved: turn.native_fields._compressed_summary + COMPACTION edge",
    "active": "preserved: turn.native_fields.active (0 = superseded by compaction)",
    "compacted": "preserved: state.compacted + COMPACTION edge",
    "api_content": "preserved: turn.native_fields.api_content (carries model-switch markers)",
    "display_kind": "preserved: turn.native_fields.display_kind",
    "display_metadata": "preserved: turn.native_fields.display_metadata",
    "display_identity": "preserved: turn.native_fields.display_identity",
    "display_order": "preserved: turn.native_fields.display_order",
    "message_uid": "preserved: turn.native_fields.message_uid",
    "absorbed_message_uids": "preserved: turn.native_fields.absorbed_message_uids",
    "tool_call_uids": "preserved: turn.native_fields.tool_call_uids",
    "tool_call_uid": "preserved: tool.native_fields.tool_call_uid",
}

# ---------------------------------------------------------------------------
# sessions (62 columns). The session row is session-level context: it is kept
# verbatim under native_meta.session so nothing about the session's shape is
# lost, and the few fields with a dedicated home are named explicitly.
# ---------------------------------------------------------------------------

_SESSION_DEDICATED = {
    "id": "preserved: session_uid",
    "source": "preserved: native_meta.source",
    "model": "preserved: native_meta.session_model (initial model; per-turn model is turn.native_model)",
    "system_prompt": "preserved: native_meta.system_prompt",
    "system_prompt_hash": "preserved: native_meta.system_prompt_hash",
    "parent_session_id": "preserved: native_meta.parent_session_id (session-level link; a cross-session SUBAGENT edge belongs to the Model layer, not this per-session canonical layer)",
    "cwd": "preserved: state.cwd + native_meta.cwd",
    "git_branch": "preserved: native_meta.git_branch",
    "git_repo_root": "preserved: native_meta.git_repo_root",
    "started_at": "preserved: native_meta.started_at",
    "ended_at": "preserved: native_meta.ended_at",
    "end_reason": "preserved: native_meta.end_reason",
    "title": "preserved: native_meta.title",
    "title_source": "preserved: native_meta.title_source",
    "message_count": "preserved: native_meta.message_count",
    "tool_call_count": "preserved: native_meta.tool_call_count",
}

# Every remaining sessions column is session-level metadata kept verbatim.
_SESSION_VERBATIM = [
    "created_source", "user_id", "session_key", "chat_id", "chat_type",
    "thread_id", "display_name", "origin_json", "expiry_finalized",
    "model_config", "input_tokens", "output_tokens", "cache_read_tokens",
    "cache_write_tokens", "reasoning_tokens", "git_metadata_generation",
    "billing_provider", "billing_base_url", "billing_mode",
    "estimated_cost_usd", "actual_cost_usd", "cost_status", "cost_source",
    "pricing_version", "last_activity_at", "last_activity_description",
    "last_activity_provenance", "api_call_count", "handoff_state",
    "handoff_platform", "handoff_error", "compression_failure_cooldown_until",
    "compression_failure_error", "compression_fallback_streak",
    "compression_ineffective_count", "compression_recovery_deadline",
    "compression_overload_streak", "profile_name", "transport_profile",
    "rewind_count", "archived", "auto_archived", "pinned", "hidden",
    "last_read_at", "tool_names",
]

SESSIONS_FATE: dict[str, str] = dict(_SESSION_DEDICATED)
for _col in _SESSION_VERBATIM:
    SESSIONS_FATE[_col] = f"preserved: native_meta.session.{_col}"

# ---------------------------------------------------------------------------
# session_model_usage (17 columns). The producer keeps per-(session, model,
# billing) token/cost telemetry here — the independent oracle for token
# accounting. Every column is preserved under native_meta.model_usage.
# ---------------------------------------------------------------------------

SESSION_MODEL_USAGE_FATE: dict[str, str] = {
    "session_id": "preserved: native_meta.model_usage.session_id",
    "model": "preserved: native_meta.models_in_session + native_meta.model_usage.model",
    "billing_provider": "preserved: native_meta.model_usage.billing_provider",
    "billing_base_url": "preserved: native_meta.model_usage.billing_base_url",
    "billing_mode": "preserved: native_meta.model_usage.billing_mode",
    "task": "preserved: native_meta.model_usage.task",
    "api_call_count": "preserved: native_meta.model_usage.api_call_count",
    "input_tokens": "preserved: native_meta.model_usage.input_tokens",
    "output_tokens": "preserved: native_meta.model_usage.output_tokens",
    "cache_read_tokens": "preserved: native_meta.model_usage.cache_read_tokens",
    "cache_write_tokens": "preserved: native_meta.model_usage.cache_write_tokens",
    "reasoning_tokens": "preserved: native_meta.model_usage.reasoning_tokens",
    "estimated_cost_usd": "preserved: native_meta.model_usage.estimated_cost_usd",
    "actual_cost_usd": "preserved: native_meta.model_usage.actual_cost_usd",
    "cost_status": "preserved: native_meta.model_usage.cost_status",
    "cost_source": "preserved: native_meta.model_usage.cost_source",
    "first_seen": "preserved: native_meta.model_usage.first_seen",
    "last_seen": "preserved: native_meta.model_usage.last_seen",
}

# ---------------------------------------------------------------------------
# Concept: reasoning — resolved across four homes, by model.
# model_profiles records what each model was OBSERVED to use, so a known model
# switching homes is drift, not silence.
# ---------------------------------------------------------------------------

REASONING = ConceptResolution(
    concept="reasoning",
    # priority order: plain text beats structured beats encrypted
    homes=("reasoning_content", "reasoning", "reasoning_details", "codex_reasoning_items"),
    model_profiles={
        # codex family: encrypted CoT only
        "gpt-5.3-codex-spark": "codex_reasoning_items",
        "gpt-5.6-luna-900k": "codex_reasoning_items",
        "gpt-5.6-terra-900k": "codex_reasoning_items",
        "muse-spark-1.3-contributor-free": "codex_reasoning_items",
        "muse-spark-1.3-contributor": "codex_reasoning_items",
        # plain CoT
        "Qwen3.6-27B-IQ3_XXS.gguf": "reasoning_content",
        "poolside/laguna-s-2.1:free": "reasoning_content",
        "meituan/longcat-2.0:free": "reasoning_content",
        "meituan/longcat-2.5-preview:free": "reasoning_content",
        "longcat-2.5-preview-free": "reasoning_content",
        "tencent/hy3:free": "reasoning_content",
        "x-preview-f-free": "reasoning_content",
        # no reasoning at all (a fact, not a loss)
        "gemini-3.8-flash-tiered": "",
    },
)

# ---------------------------------------------------------------------------
# system_prompts (2 columns). The exact prompt TEXT. ``sessions.system_prompt``
# is usually NULL; the real prompt is only reachable through
# ``sessions.system_prompt_hash`` -> ``system_prompts.hash`` (1734 sessions
# resolvable on the capture). Normalization resolves the text into
# ``native_meta.system_prompt_resolved``.
# ---------------------------------------------------------------------------

SYSTEM_PROMPTS_FATE: dict[str, str] = {
    "hash": "preserved: native_meta.system_prompt_resolved (key on sessions.system_prompt_hash)",
    "prompt": "preserved: native_meta.system_prompt_resolved.prompt (exact prompt text)",
}

# ---------------------------------------------------------------------------
# async_delegations (19 columns). The delegation graph: which session spawned an
# async subagent and how its result was delivered. ``origin_session`` links back
# to a session (38/38 resolvable). Normalization attaches the rows to the origin
# session under ``native_meta.async_delegations``.
# ---------------------------------------------------------------------------

_ASYNC_DELEGATION_PRESERVED = [
    "delegation_id", "origin_session", "origin_ui_session_id",
    "parent_session_id", "state", "dispatched_at", "completed_at",
    "updated_at", "event_json", "result_json", "delivery_state",
    "delivery_attempts", "delivered_at", "task_json", "delivery_claim",
    "delivery_claimed_at", "origin_session_id",
]
ASYNC_DELEGATIONS_FATE: dict[str, str] = {
    c: f"preserved: native_meta.async_delegations.{c}" for c in _ASYNC_DELEGATION_PRESERVED
}
# The process owner of a delegation is live infrastructure state, not session
# truth: it is declared out, not carried into the canonical graph.
ASYNC_DELEGATIONS_FATE["owner_pid"] = "declared: live process owner, not session truth"
ASYNC_DELEGATIONS_FATE["owner_started_at"] = "declared: live process owner, not session truth"

# ---------------------------------------------------------------------------
# state_meta (2 columns) — store provenance. Kept as a verbatim map.
# ---------------------------------------------------------------------------

STATE_META_FATE: dict[str, str] = {
    "key": "preserved: native_meta.store_meta (verbatim key)",
    "value": "preserved: native_meta.store_meta[key]",
}

# ---------------------------------------------------------------------------
# schema_version (1 column) — the α2 anchor the contract pins to.
# ---------------------------------------------------------------------------

SCHEMA_VERSION_FATE: dict[str, str] = {
    "version": "preserved: native_meta.producer_schema_version",
}

# ---------------------------------------------------------------------------
# Tables declared OUT OF SCOPE. Not every producer table is session truth:
# locks, leases, heartbeats, routing and hygiene state are LIVE OPERATIONAL
# INFRASTRUCTURE — ephemeral, reconstructible, and NOT a statement about what a
# session was. Modeling them would manufacture false authority. They are named
# here with a reason so the closure test proves each table was decided, never
# silently skipped. (FTS shadow tables and sqlite_sequence are SQLite internals,
# not independent data tables.)
# ---------------------------------------------------------------------------

_OUT_OF_SCOPE: dict[str, str] = {
    "compression_locks": "operational: ephemeral lock rows, not session truth",
    "session_turn_leases": "operational: ephemeral turn lease rows, not session truth",
    "gateway_heartbeats": "operational: gateway liveness heartbeats, not session truth",
    "gateway_hygiene_state": "operational: gateway hygiene counters, not session truth",
    "gateway_routing": "operational: routing table, not session truth",
    "conversation_generations": "operational: generation counters (0 rows on this capture)",
}

CONTRACT = ConservationContract(
    producer=PRODUCER,
    producer_schema_version=PRODUCER_SCHEMA_VERSION,
    producer_schema_source=PRODUCER_SCHEMA_SOURCE,
    field_fate={
        **{f"messages.{k}": v for k, v in MESSAGES_FATE.items()},
        **{f"sessions.{k}": v for k, v in SESSIONS_FATE.items()},
        **{f"session_model_usage.{k}": v for k, v in SESSION_MODEL_USAGE_FATE.items()},
        **{f"system_prompts.{k}": v for k, v in SYSTEM_PROMPTS_FATE.items()},
        **{f"async_delegations.{k}": v for k, v in ASYNC_DELEGATIONS_FATE.items()},
        **{f"state_meta.{k}": v for k, v in STATE_META_FATE.items()},
        **{f"schema_version.{k}": v for k, v in SCHEMA_VERSION_FATE.items()},
    },
    concepts=(REASONING,),
    transforms={
        "turn.timestamp_iso": "unix seconds -> ISO-8601 UTC with trailing Z",
    },
    out_of_scope=_OUT_OF_SCOPE,
)
