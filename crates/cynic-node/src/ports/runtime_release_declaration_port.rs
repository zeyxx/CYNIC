use cynic_protocol::RuntimeReleaseDeclaration;

/// Kernel mutation boundary for an already-formed runtime release declaration.
#[async_trait::async_trait]
pub trait RuntimeReleaseDeclarationPort: Send + Sync {
    async fn declare_runtime_release(
        &self,
        declaration: &RuntimeReleaseDeclaration,
    ) -> Result<RuntimeReleaseDeclarationReadBack, String>;
}

pub struct RuntimeReleaseDeclarationReadBack {
    pub http_status: u16,
    pub body: serde_json::Value,
}
