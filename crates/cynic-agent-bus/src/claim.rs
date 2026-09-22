use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::types::*;
#[async_trait::async_trait]
pub trait ClaimStore: Send + Sync {
    async fn acquire(&self, claim: Claim) -> Result<(), String>;
    async fn release(&self, claim_id: &str) -> Result<(), ClaimError>;
    async fn active_claims(&self) -> Vec<Claim>;
    async fn get_claim(&self, claim_id: &str) -> Option<Claim>;
    async fn find_by_scope(&self, path: &str) -> Vec<Claim>;
}

pub struct InMemoryClaimStore {
    pub claims: Arc<Mutex<HashMap<String, Claim>>>,
}

impl InMemoryClaimStore {
    pub fn new() -> Self {
        Self {
            claims: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn scope_overlap(existing: &Scope, new: &Scope) -> bool {
        match (&existing.kind, &new.kind) {
            (ScopeKind::File, ScopeKind::File) => existing.path == new.path,
            (ScopeKind::Module, ScopeKind::Module) => existing.path == new.path,
            (ScopeKind::Crate, ScopeKind::Crate) => existing.path == new.path,
            (ScopeKind::Crate, _) => new.path.starts_with(&existing.path),
            (_, ScopeKind::Crate) => existing.path.starts_with(&new.path),
            (ScopeKind::Module, ScopeKind::File) => new.path.starts_with(&existing.path),
            (ScopeKind::File, ScopeKind::Module) => existing.path.starts_with(&new.path),
        }
    }
}

#[async_trait::async_trait]
impl ClaimStore for InMemoryClaimStore {
    async fn acquire(&self, claim: Claim) -> Result<(), String> {
        let mut claims = self.claims.lock().await;
        if claims.contains_key(&claim.claim_id) {
            return Err(format!("claim {} already exists", claim.claim_id));
        }
        for existing in claims.values() {
            if matches!(existing.status, ClaimStatus::Active) {
                if Self::scope_overlap(&existing.scope, &claim.scope) {
                    return Err(format!(
                        "scope {} is locked by claim {}",
                        claim.scope.path, existing.claim_id
                    ));
                }
            }
        }
        claims.insert(claim.claim_id.clone(), claim);
        Ok(())
    }

    async fn release(&self, claim_id: &str) -> Result<(), ClaimError> {
        let mut claims = self.claims.lock().await;
        let claim = claims
            .get_mut(claim_id)
            .ok_or_else(|| ClaimError::NotFound(claim_id.to_string()))?;
        if matches!(claim.status, ClaimStatus::Released) {
            return Err(ClaimError::NotFound(claim_id.to_string()));
        }
        claim.status = ClaimStatus::Released;
        Ok(())
    }

    async fn active_claims(&self) -> Vec<Claim> {
        self.claims
            .lock()
            .await
            .values()
            .filter(|c| matches!(c.status, ClaimStatus::Active))
            .cloned()
            .collect()
    }

    async fn get_claim(&self, claim_id: &str) -> Option<Claim> {
        self.claims.lock().await.get(claim_id).cloned()
    }

    async fn find_by_scope(&self, path: &str) -> Vec<Claim> {
        self.claims
            .lock()
            .await
            .values()
            .filter(|c| c.scope.path == path)
            .cloned()
            .collect()
    }
}

pub struct ClaimManager {
    pub store: Arc<dyn ClaimStore>,
}

impl ClaimManager {
    pub fn new(store: Arc<dyn ClaimStore>) -> Self {
        Self { store }
    }

    pub async fn claim(&self, task: &Task) -> Result<Claim, String> {
        let claim = Claim {
            claim_id: format!("claim_{}", task.id),
            task_id: task.id.clone(),
            agent: task.assigned_to,
            status: ClaimStatus::Active,
            budget: task.budget,
            consumed_actions: 0,
            scope: Scope {
                kind: ScopeKind::File,
                path: task.subject.clone(),
            },
            acquired_at_unix_s: now_unix_s(),
            ttl_seconds: 1800,
            parent_claim_id: None,
        };
        self.store.acquire(claim.clone()).await?;
        Ok(claim)
    }

    pub async fn release(&self, claim_id: &str) -> Result<(), ClaimError> {
        self.store.release(claim_id).await
    }

    pub fn has_budget(&self, claim: &Claim) -> bool {
        claim.consumed_actions < claim.budget.actions
    }

    pub fn consume_action(&self, claim: &mut Claim) {
        claim.consumed_actions += 1;
    }
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn claim_acquire_and_release() {
        let store = Arc::new(InMemoryClaimStore::new());
        let manager = ClaimManager::new(store);
        let task = Task {
            id: "t1".to_string(),
            parent_task_id: None,
            created_by: AgentId([0u8; 16]),
            assigned_to: AgentId([1u8; 16]),
            kind: TaskKind::SoloFile,
            subject: "src/foo.rs".to_string(),
            body: "edit foo".to_string(),
            status: TaskStatus::Queued,
            result: None,
            error: None,
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
            expires_at_unix_s: 0,
            budget: Budget {
                actions: 10,
                wall_seconds: 60,
            },
        };
        let claim = manager.claim(&task).await.unwrap();
        assert_eq!(claim.claim_id, "claim_t1");
        assert_eq!(claim.status, ClaimStatus::Active);
        manager.release("claim_t1").await.unwrap();
    }

    #[tokio::test]
    async fn claim_scope_conflict() {
        let store = Arc::new(InMemoryClaimStore::new());
        let manager = ClaimManager::new(store);
        let task1 = Task {
            id: "t1".to_string(),
            parent_task_id: None,
            created_by: AgentId([0u8; 16]),
            assigned_to: AgentId([1u8; 16]),
            kind: TaskKind::SoloFile,
            subject: "src/foo.rs".to_string(),
            body: "edit foo".to_string(),
            status: TaskStatus::Queued,
            result: None,
            error: None,
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
            expires_at_unix_s: 0,
            budget: Budget {
                actions: 10,
                wall_seconds: 60,
            },
        };
        let task2 = Task {
            id: "t2".to_string(),
            parent_task_id: None,
            created_by: AgentId([0u8; 16]),
            assigned_to: AgentId([2u8; 16]),
            kind: TaskKind::SoloFile,
            subject: "src/foo.rs".to_string(),
            body: "edit foo again".to_string(),
            status: TaskStatus::Queued,
            result: None,
            error: None,
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
            expires_at_unix_s: 0,
            budget: Budget {
                actions: 10,
                wall_seconds: 60,
            },
        };
        manager.claim(&task1).await.unwrap();
        let result = manager.claim(&task2).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("locked"));
    }
}
