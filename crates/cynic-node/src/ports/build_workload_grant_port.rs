pub trait BuildWorkloadGrantSignerPort: Send + Sync {
    fn authority_key_id(&self) -> &str;
    fn sign(&self, payload: &[u8]) -> Result<String, String>;
}

pub trait BuildWorkloadGrantVerifierPort: Send + Sync {
    fn verify(
        &self,
        authority_key_id: &str,
        payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), String>;
}
