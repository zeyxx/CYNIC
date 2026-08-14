#[derive(Debug, Clone)]
pub(crate) struct ChatArgs {
    pub prompt: Option<String>,
    pub resume: Option<String>,
    pub session_id: Option<String>,
    pub stream: bool,
    pub json: bool,
    pub list_sessions: bool,
    pub export: Option<String>,
    pub mock: bool,
    /// Prefer ODS OpenAI-compat discovery for endpoint resolution.
    pub ods: bool,
    /// Force model selector:
    /// - local alias: starts that native local profile when available
    /// - otherwise: OpenAI model id override for selected endpoint.
    pub model: Option<String>,
    /// Force non-interactive path (even without prompt shows help / blocked JSON).
    pub no_tui: bool,
}
