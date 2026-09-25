use std::sync::Arc;

pub struct InferenceClient {
    pub endpoint: String,
}

impl InferenceClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }
}
