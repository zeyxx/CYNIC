//! REST handlers for agent task queue — dispatch, poll, complete.
//! Mirrors MCP agent_tools for HTTP clients (Python daemons, curl, future brain agent).

pub mod handlers_mutation;
pub mod handlers_proposals;
pub mod handlers_query;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

pub use handlers_mutation::{claim_task_handler, complete_task_handler, dispatch_task_handler};
pub use handlers_proposals::{list_agent_proposals_handler, review_agent_proposal_handler};
pub use handlers_query::{list_completed_tasks_handler, list_pending_tasks, list_tasks_handler, watch_tasks_handler};
pub use types::{
    default_task_kind, default_task_limit, default_watch_timeout_secs, ClaimTaskRequest,
    DispatchTaskRequest, ReviewAgentProposalRequest, UpdateTaskResultRequest, WatchTasksQuery,
    AGENT_PROPOSAL_REVIEW_SCHEMA, AGENT_PROPOSAL_SCHEMA, AGENT_TASK_STORAGE_TIMEOUT,
    VALID_AGENT_PROPOSAL_REVIEW_STATES,
};
pub use validation::{
    agent_proposal_review_value, event_matches_pending_task_watch, extract_agent_proposal,
    filter_proposal_projection, task_proposal_projection, validate_agent_proposal_review_request,
    validate_agent_task_content, wait_for_matching_task_event,
};
