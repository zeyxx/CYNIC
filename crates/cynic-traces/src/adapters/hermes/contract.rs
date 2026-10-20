//! Hermes conservation contract — MACHINE-TRANSCRIBED.
//!
//! Do not hand-edit. Regenerate with:
//!   PYTHONPATH=apps/cynic-data/src python3 crates/cynic-traces/tools/gen_hermes_contract.py
//! Prose rationale lives in the Python source
//! (apps/cynic-data/src/cynic_data/canonical/adapters/normalizers/hermes_contract.py).

use std::collections::HashMap;

use crate::conservation::{ConceptResolution, ConservationContract};

pub const PRODUCER: &str = "hermes-agent";
pub const PRODUCER_SCHEMA_VERSION: i64 = 31;
pub const PRODUCER_SCHEMA_SOURCE: &str = "hermes-agent/hermes_state_common.py:SCHEMA_SQL";

pub fn contract() -> ConservationContract {
    let mut field_fate = HashMap::new();
    field_fate.insert("async_delegations.completed_at".to_owned(), "preserved: native_meta.async_delegations.completed_at".to_owned());
    field_fate.insert("async_delegations.delegation_id".to_owned(), "preserved: native_meta.async_delegations.delegation_id".to_owned());
    field_fate.insert("async_delegations.delivered_at".to_owned(), "preserved: native_meta.async_delegations.delivered_at".to_owned());
    field_fate.insert("async_delegations.delivery_attempts".to_owned(), "preserved: native_meta.async_delegations.delivery_attempts".to_owned());
    field_fate.insert("async_delegations.delivery_claim".to_owned(), "preserved: native_meta.async_delegations.delivery_claim".to_owned());
    field_fate.insert("async_delegations.delivery_claimed_at".to_owned(), "preserved: native_meta.async_delegations.delivery_claimed_at".to_owned());
    field_fate.insert("async_delegations.delivery_state".to_owned(), "preserved: native_meta.async_delegations.delivery_state".to_owned());
    field_fate.insert("async_delegations.dispatched_at".to_owned(), "preserved: native_meta.async_delegations.dispatched_at".to_owned());
    field_fate.insert("async_delegations.event_json".to_owned(), "preserved: native_meta.async_delegations.event_json".to_owned());
    field_fate.insert("async_delegations.origin_session".to_owned(), "preserved: native_meta.async_delegations.origin_session".to_owned());
    field_fate.insert("async_delegations.origin_session_id".to_owned(), "preserved: native_meta.async_delegations.origin_session_id".to_owned());
    field_fate.insert("async_delegations.origin_ui_session_id".to_owned(), "preserved: native_meta.async_delegations.origin_ui_session_id".to_owned());
    field_fate.insert("async_delegations.owner_pid".to_owned(), "declared: live process owner, not session truth".to_owned());
    field_fate.insert("async_delegations.owner_started_at".to_owned(), "declared: live process owner, not session truth".to_owned());
    field_fate.insert("async_delegations.parent_session_id".to_owned(), "preserved: native_meta.async_delegations.parent_session_id".to_owned());
    field_fate.insert("async_delegations.result_json".to_owned(), "preserved: native_meta.async_delegations.result_json".to_owned());
    field_fate.insert("async_delegations.state".to_owned(), "preserved: native_meta.async_delegations.state".to_owned());
    field_fate.insert("async_delegations.task_json".to_owned(), "preserved: native_meta.async_delegations.task_json".to_owned());
    field_fate.insert("async_delegations.updated_at".to_owned(), "preserved: native_meta.async_delegations.updated_at".to_owned());
    field_fate.insert("messages._compressed_summary".to_owned(), "preserved: turn.native_fields._compressed_summary + COMPACTION edge".to_owned());
    field_fate.insert("messages.absorbed_message_uids".to_owned(), "preserved: turn.native_fields.absorbed_message_uids".to_owned());
    field_fate.insert("messages.active".to_owned(), "preserved: turn.native_fields.active (0 = superseded by compaction)".to_owned());
    field_fate.insert("messages.api_content".to_owned(), "preserved: turn.native_fields.api_content (carries model-switch markers)".to_owned());
    field_fate.insert("messages.codex_message_items".to_owned(), "preserved: turn.native_fields.codex_message_items".to_owned());
    field_fate.insert("messages.codex_reasoning_items".to_owned(), "preserved: reasoning.native_fields.codex_reasoning_items (encrypted CoT; presence and bytes kept, never decoded)".to_owned());
    field_fate.insert("messages.compacted".to_owned(), "preserved: state.compacted + COMPACTION edge".to_owned());
    field_fate.insert("messages.content".to_owned(), "preserved: turn.content".to_owned());
    field_fate.insert("messages.display_identity".to_owned(), "preserved: turn.native_fields.display_identity".to_owned());
    field_fate.insert("messages.display_kind".to_owned(), "preserved: turn.native_fields.display_kind".to_owned());
    field_fate.insert("messages.display_metadata".to_owned(), "preserved: turn.native_fields.display_metadata".to_owned());
    field_fate.insert("messages.display_order".to_owned(), "preserved: turn.native_fields.display_order".to_owned());
    field_fate.insert("messages.effect_disposition".to_owned(), "preserved: turn.native_fields.effect_disposition".to_owned());
    field_fate.insert("messages.finish_reason".to_owned(), "preserved: turn.native_fields.finish_reason".to_owned());
    field_fate.insert("messages.id".to_owned(), "preserved: turn.native_fields.native_id (ordering + native id)".to_owned());
    field_fate.insert("messages.message_uid".to_owned(), "preserved: turn.native_fields.message_uid".to_owned());
    field_fate.insert("messages.observed".to_owned(), "preserved: turn.native_fields.observed".to_owned());
    field_fate.insert("messages.platform_message_id".to_owned(), "preserved: turn.native_fields.platform_message_id".to_owned());
    field_fate.insert("messages.reasoning".to_owned(), "preserved: reasoning.content (via concept 'reasoning')".to_owned());
    field_fate.insert("messages.reasoning_content".to_owned(), "preserved: reasoning.content (via concept 'reasoning')".to_owned());
    field_fate.insert("messages.reasoning_details".to_owned(), "preserved: reasoning.native_fields.reasoning_details".to_owned());
    field_fate.insert("messages.role".to_owned(), "preserved: turn.role".to_owned());
    field_fate.insert("messages.session_id".to_owned(), "preserved: session_uid".to_owned());
    field_fate.insert("messages.timestamp".to_owned(), "preserved: turn.timestamp_iso".to_owned());
    field_fate.insert("messages.token_count".to_owned(), "declared: unpopulated in the producer (0 non-null rows)".to_owned());
    field_fate.insert("messages.tool_call_id".to_owned(), "preserved: tool.call_id (result -> call link)".to_owned());
    field_fate.insert("messages.tool_call_uid".to_owned(), "preserved: tool.native_fields.tool_call_uid".to_owned());
    field_fate.insert("messages.tool_call_uids".to_owned(), "preserved: turn.native_fields.tool_call_uids".to_owned());
    field_fate.insert("messages.tool_calls".to_owned(), "preserved: tool.command_or_args + TOOL_CALL edge".to_owned());
    field_fate.insert("messages.tool_name".to_owned(), "preserved: tool.native_fields.tool_name".to_owned());
    field_fate.insert("schema_version.version".to_owned(), "preserved: native_meta.producer_schema_version".to_owned());
    field_fate.insert("session_model_usage.actual_cost_usd".to_owned(), "preserved: native_meta.model_usage.actual_cost_usd".to_owned());
    field_fate.insert("session_model_usage.api_call_count".to_owned(), "preserved: native_meta.model_usage.api_call_count".to_owned());
    field_fate.insert("session_model_usage.billing_base_url".to_owned(), "preserved: native_meta.model_usage.billing_base_url".to_owned());
    field_fate.insert("session_model_usage.billing_mode".to_owned(), "preserved: native_meta.model_usage.billing_mode".to_owned());
    field_fate.insert("session_model_usage.billing_provider".to_owned(), "preserved: native_meta.model_usage.billing_provider".to_owned());
    field_fate.insert("session_model_usage.cache_read_tokens".to_owned(), "preserved: native_meta.model_usage.cache_read_tokens".to_owned());
    field_fate.insert("session_model_usage.cache_write_tokens".to_owned(), "preserved: native_meta.model_usage.cache_write_tokens".to_owned());
    field_fate.insert("session_model_usage.cost_source".to_owned(), "preserved: native_meta.model_usage.cost_source".to_owned());
    field_fate.insert("session_model_usage.cost_status".to_owned(), "preserved: native_meta.model_usage.cost_status".to_owned());
    field_fate.insert("session_model_usage.estimated_cost_usd".to_owned(), "preserved: native_meta.model_usage.estimated_cost_usd".to_owned());
    field_fate.insert("session_model_usage.first_seen".to_owned(), "preserved: native_meta.model_usage.first_seen".to_owned());
    field_fate.insert("session_model_usage.input_tokens".to_owned(), "preserved: native_meta.model_usage.input_tokens".to_owned());
    field_fate.insert("session_model_usage.last_seen".to_owned(), "preserved: native_meta.model_usage.last_seen".to_owned());
    field_fate.insert("session_model_usage.model".to_owned(), "preserved: native_meta.models_in_session + native_meta.model_usage.model".to_owned());
    field_fate.insert("session_model_usage.output_tokens".to_owned(), "preserved: native_meta.model_usage.output_tokens".to_owned());
    field_fate.insert("session_model_usage.reasoning_tokens".to_owned(), "preserved: native_meta.model_usage.reasoning_tokens".to_owned());
    field_fate.insert("session_model_usage.session_id".to_owned(), "preserved: native_meta.model_usage.session_id".to_owned());
    field_fate.insert("session_model_usage.task".to_owned(), "preserved: native_meta.model_usage.task".to_owned());
    field_fate.insert("sessions.actual_cost_usd".to_owned(), "preserved: native_meta.session.actual_cost_usd".to_owned());
    field_fate.insert("sessions.api_call_count".to_owned(), "preserved: native_meta.session.api_call_count".to_owned());
    field_fate.insert("sessions.archived".to_owned(), "preserved: native_meta.session.archived".to_owned());
    field_fate.insert("sessions.auto_archived".to_owned(), "preserved: native_meta.session.auto_archived".to_owned());
    field_fate.insert("sessions.billing_base_url".to_owned(), "preserved: native_meta.session.billing_base_url".to_owned());
    field_fate.insert("sessions.billing_mode".to_owned(), "preserved: native_meta.session.billing_mode".to_owned());
    field_fate.insert("sessions.billing_provider".to_owned(), "preserved: native_meta.session.billing_provider".to_owned());
    field_fate.insert("sessions.cache_read_tokens".to_owned(), "preserved: native_meta.session.cache_read_tokens".to_owned());
    field_fate.insert("sessions.cache_write_tokens".to_owned(), "preserved: native_meta.session.cache_write_tokens".to_owned());
    field_fate.insert("sessions.chat_id".to_owned(), "preserved: native_meta.session.chat_id".to_owned());
    field_fate.insert("sessions.chat_type".to_owned(), "preserved: native_meta.session.chat_type".to_owned());
    field_fate.insert("sessions.compression_failure_cooldown_until".to_owned(), "preserved: native_meta.session.compression_failure_cooldown_until".to_owned());
    field_fate.insert("sessions.compression_failure_error".to_owned(), "preserved: native_meta.session.compression_failure_error".to_owned());
    field_fate.insert("sessions.compression_fallback_streak".to_owned(), "preserved: native_meta.session.compression_fallback_streak".to_owned());
    field_fate.insert("sessions.compression_ineffective_count".to_owned(), "preserved: native_meta.session.compression_ineffective_count".to_owned());
    field_fate.insert("sessions.compression_overload_streak".to_owned(), "preserved: native_meta.session.compression_overload_streak".to_owned());
    field_fate.insert("sessions.compression_recovery_deadline".to_owned(), "preserved: native_meta.session.compression_recovery_deadline".to_owned());
    field_fate.insert("sessions.cost_source".to_owned(), "preserved: native_meta.session.cost_source".to_owned());
    field_fate.insert("sessions.cost_status".to_owned(), "preserved: native_meta.session.cost_status".to_owned());
    field_fate.insert("sessions.created_source".to_owned(), "preserved: native_meta.session.created_source".to_owned());
    field_fate.insert("sessions.cwd".to_owned(), "preserved: state.cwd + native_meta.cwd".to_owned());
    field_fate.insert("sessions.display_name".to_owned(), "preserved: native_meta.session.display_name".to_owned());
    field_fate.insert("sessions.end_reason".to_owned(), "preserved: native_meta.end_reason".to_owned());
    field_fate.insert("sessions.ended_at".to_owned(), "preserved: native_meta.ended_at".to_owned());
    field_fate.insert("sessions.estimated_cost_usd".to_owned(), "preserved: native_meta.session.estimated_cost_usd".to_owned());
    field_fate.insert("sessions.expiry_finalized".to_owned(), "preserved: native_meta.session.expiry_finalized".to_owned());
    field_fate.insert("sessions.git_branch".to_owned(), "preserved: native_meta.git_branch".to_owned());
    field_fate.insert("sessions.git_metadata_generation".to_owned(), "preserved: native_meta.session.git_metadata_generation".to_owned());
    field_fate.insert("sessions.git_repo_root".to_owned(), "preserved: native_meta.git_repo_root".to_owned());
    field_fate.insert("sessions.handoff_error".to_owned(), "preserved: native_meta.session.handoff_error".to_owned());
    field_fate.insert("sessions.handoff_platform".to_owned(), "preserved: native_meta.session.handoff_platform".to_owned());
    field_fate.insert("sessions.handoff_state".to_owned(), "preserved: native_meta.session.handoff_state".to_owned());
    field_fate.insert("sessions.hidden".to_owned(), "preserved: native_meta.session.hidden".to_owned());
    field_fate.insert("sessions.id".to_owned(), "preserved: session_uid".to_owned());
    field_fate.insert("sessions.input_tokens".to_owned(), "preserved: native_meta.session.input_tokens".to_owned());
    field_fate.insert("sessions.last_activity_at".to_owned(), "preserved: native_meta.session.last_activity_at".to_owned());
    field_fate.insert("sessions.last_activity_description".to_owned(), "preserved: native_meta.session.last_activity_description".to_owned());
    field_fate.insert("sessions.last_activity_provenance".to_owned(), "preserved: native_meta.session.last_activity_provenance".to_owned());
    field_fate.insert("sessions.last_read_at".to_owned(), "preserved: native_meta.session.last_read_at".to_owned());
    field_fate.insert("sessions.message_count".to_owned(), "preserved: native_meta.message_count".to_owned());
    field_fate.insert("sessions.model".to_owned(), "preserved: native_meta.session_model (initial model; per-turn model is turn.native_model)".to_owned());
    field_fate.insert("sessions.model_config".to_owned(), "preserved: native_meta.session.model_config".to_owned());
    field_fate.insert("sessions.origin_json".to_owned(), "preserved: native_meta.session.origin_json".to_owned());
    field_fate.insert("sessions.output_tokens".to_owned(), "preserved: native_meta.session.output_tokens".to_owned());
    field_fate.insert("sessions.parent_session_id".to_owned(), "preserved: native_meta.parent_session_id (session-level link; a cross-session SUBAGENT edge belongs to the Model layer, not this per-session canonical layer)".to_owned());
    field_fate.insert("sessions.pinned".to_owned(), "preserved: native_meta.session.pinned".to_owned());
    field_fate.insert("sessions.pricing_version".to_owned(), "preserved: native_meta.session.pricing_version".to_owned());
    field_fate.insert("sessions.profile_name".to_owned(), "preserved: native_meta.session.profile_name".to_owned());
    field_fate.insert("sessions.reasoning_tokens".to_owned(), "preserved: native_meta.session.reasoning_tokens".to_owned());
    field_fate.insert("sessions.rewind_count".to_owned(), "preserved: native_meta.session.rewind_count".to_owned());
    field_fate.insert("sessions.session_key".to_owned(), "preserved: native_meta.session.session_key".to_owned());
    field_fate.insert("sessions.source".to_owned(), "preserved: native_meta.source".to_owned());
    field_fate.insert("sessions.started_at".to_owned(), "preserved: native_meta.started_at".to_owned());
    field_fate.insert("sessions.system_prompt".to_owned(), "preserved: native_meta.system_prompt".to_owned());
    field_fate.insert("sessions.system_prompt_hash".to_owned(), "preserved: native_meta.system_prompt_hash".to_owned());
    field_fate.insert("sessions.thread_id".to_owned(), "preserved: native_meta.session.thread_id".to_owned());
    field_fate.insert("sessions.title".to_owned(), "preserved: native_meta.title".to_owned());
    field_fate.insert("sessions.title_source".to_owned(), "preserved: native_meta.title_source".to_owned());
    field_fate.insert("sessions.tool_call_count".to_owned(), "preserved: native_meta.tool_call_count".to_owned());
    field_fate.insert("sessions.tool_names".to_owned(), "preserved: native_meta.session.tool_names".to_owned());
    field_fate.insert("sessions.transport_profile".to_owned(), "preserved: native_meta.session.transport_profile".to_owned());
    field_fate.insert("sessions.user_id".to_owned(), "preserved: native_meta.session.user_id".to_owned());
    field_fate.insert("state_meta.key".to_owned(), "preserved: native_meta.store_meta (verbatim key)".to_owned());
    field_fate.insert("state_meta.value".to_owned(), "preserved: native_meta.store_meta[key]".to_owned());
    field_fate.insert("system_prompts.hash".to_owned(), "preserved: native_meta.system_prompt_resolved (key on sessions.system_prompt_hash)".to_owned());
    field_fate.insert("system_prompts.prompt".to_owned(), "preserved: native_meta.system_prompt_resolved.prompt (exact prompt text)".to_owned());
    let mut out_of_scope = HashMap::new();
    out_of_scope.insert("compression_locks".to_owned(), "operational: ephemeral lock rows, not session truth".to_owned());
    out_of_scope.insert("conversation_generations".to_owned(), "operational: generation counters (0 rows on this capture)".to_owned());
    out_of_scope.insert("gateway_heartbeats".to_owned(), "operational: gateway liveness heartbeats, not session truth".to_owned());
    out_of_scope.insert("gateway_hygiene_state".to_owned(), "operational: gateway hygiene counters, not session truth".to_owned());
    out_of_scope.insert("gateway_routing".to_owned(), "operational: routing table, not session truth".to_owned());
    out_of_scope.insert("session_turn_leases".to_owned(), "operational: ephemeral turn lease rows, not session truth".to_owned());
    let reasoning = ConceptResolution {
        concept: "reasoning".to_owned(),
        homes: vec!["reasoning_content".to_owned(), "reasoning".to_owned(), "reasoning_details".to_owned(), "codex_reasoning_items".to_owned()],
        model_profiles: HashMap::new(),
    };
    let mut profiles = reasoning.model_profiles.clone();
    profiles.insert("Qwen3.6-27B-IQ3_XXS.gguf".to_owned(), "reasoning_content".to_owned());
    profiles.insert("gemini-3.8-flash-tiered".to_owned(), "".to_owned());
    profiles.insert("gpt-5.3-codex-spark".to_owned(), "codex_reasoning_items".to_owned());
    profiles.insert("gpt-5.6-luna-900k".to_owned(), "codex_reasoning_items".to_owned());
    profiles.insert("gpt-5.6-terra-900k".to_owned(), "codex_reasoning_items".to_owned());
    profiles.insert("longcat-2.5-preview-free".to_owned(), "reasoning_content".to_owned());
    profiles.insert("meituan/longcat-2.0:free".to_owned(), "reasoning_content".to_owned());
    profiles.insert("meituan/longcat-2.5-preview:free".to_owned(), "reasoning_content".to_owned());
    profiles.insert("muse-spark-1.3-contributor".to_owned(), "codex_reasoning_items".to_owned());
    profiles.insert("muse-spark-1.3-contributor-free".to_owned(), "codex_reasoning_items".to_owned());
    profiles.insert("poolside/laguna-s-2.1:free".to_owned(), "reasoning_content".to_owned());
    profiles.insert("tencent/hy3:free".to_owned(), "reasoning_content".to_owned());
    profiles.insert("x-preview-f-free".to_owned(), "reasoning_content".to_owned());
    let reasoning = ConceptResolution { model_profiles: profiles, ..reasoning };
    let mut transforms = HashMap::new();
    transforms.insert("turn.timestamp_iso".to_owned(), "unix seconds -> ISO-8601 UTC with trailing Z".to_owned());
    ConservationContract {
        producer: PRODUCER.to_owned(),
        producer_schema_version: PRODUCER_SCHEMA_VERSION,
        producer_schema_source: PRODUCER_SCHEMA_SOURCE.to_owned(),
        field_fate,
        concepts: vec![reasoning],
        transforms,
        out_of_scope,
    }
}
