use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdentityAttestParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdentitySmokeCheckParams {
    pub providers: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdentityTrustChainStatusParams {
    pub key_path: Option<String>,
    pub cert_path: Option<String>,
}
