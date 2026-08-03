#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InferenceDiscoverParams {
    /// Persist discovered peers into the inference SSOT only when explicitly true.
    #[serde(default)]
    pub apply: bool,
}
