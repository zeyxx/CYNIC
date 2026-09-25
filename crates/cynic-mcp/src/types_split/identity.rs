#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdentityAttestParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdentitySmokeCheckParams {
    pub providers: Option<Vec<String>>,
}
