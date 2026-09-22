use std::collections::HashMap;
use std::sync::Arc;

use cynic_agent_bus::*;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub const DEFAULT_SPAN_OF_CONTROL: usize = 5;
pub const MAX_SPAN_OF_CONTROL: usize = 7;
pub const MIN_SPAN_OF_CONTROL: usize = 3;

/// Domain lead — manages a vertical (Forensics, Kernel, Inference, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub name: String,
    pub domain: Domain,
    pub endpoint: String,
    pub max_workers: usize,
    pub active_workers: usize,
}

/// Domain verticals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    Forensics,
    Kernel,
    Inference,
    Security,
    Protocol,
    Mcp,
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::Forensics => write!(f, "forensics"),
            Domain::Kernel => write!(f, "kernel"),
            Domain::Inference => write!(f, "inference"),
            Domain::Security => write!(f, "security"),
            Domain::Protocol => write!(f, "protocol"),
            Domain::Mcp => write!(f, "mcp"),
        }
    }
}

/// Fleet topology — the military pyramid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetTopology {
    pub coordinator_id: String,
    pub leads: HashMap<Domain, Lead>,
}

/// Coordinator — decomposes plans and assigns to leads.
pub struct Coordinator {
    topology: Arc<RwLock<FleetTopology>>,
    ledger: Arc<InMemoryLedger>,
    claim_store: Arc<InMemoryClaimStore>,
    span_of_control: usize,
}

impl Coordinator {
    pub fn new(
        coordinator_id: String,
        ledger: Arc<InMemoryLedger>,
        claim_store: Arc<InMemoryClaimStore>,
    ) -> Self {
        Self {
            topology: Arc::new(RwLock::new(FleetTopology {
                coordinator_id,
                leads: HashMap::new(),
            })),
            ledger,
            claim_store,
            span_of_control: DEFAULT_SPAN_OF_CONTROL,
        }
    }

    pub fn with_span_of_control(mut self, span: usize) -> Self {
        assert!(
            (MIN_SPAN_OF_CONTROL..=MAX_SPAN_OF_CONTROL).contains(&span),
            "span of control must be between {MIN_SPAN_OF_CONTROL} and {MAX_SPAN_OF_CONTROL}"
        );
        self.span_of_control = span;
        self
    }

    /// Register a domain lead.
    pub async fn register_lead(
        &self,
        domain: Domain,
        name: &str,
        endpoint: &str,
    ) -> Result<(), String> {
        let mut topology = self.topology.write().await;
        topology.leads.insert(
            domain,
            Lead {
                name: name.to_string(),
                domain,
                endpoint: endpoint.to_string(),
                max_workers: self.span_of_control,
                active_workers: 0,
            },
        );
        Ok(())
    }

    /// Decompose a plan into tasks and assign to leads.
    pub async fn decompose_plan(&self, goal: &str) -> Result<Vec<Task>, String> {
        let topology = self.topology.read().await;
        if topology.leads.is_empty() {
            return Err("no leads registered".to_string());
        }

        // Simple round-robin assignment across domains.
        // In production: LLM-based decomposition.
        let domains: Vec<_> = topology.leads.keys().cloned().collect();
        let mut tasks = Vec::new();

        for (idx, line) in goal.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let domain = &domains[idx % domains.len()];
            let lead = topology.leads.get(domain).unwrap();

            let task = Task {
                id: format!("task_{idx}"),
                parent_task_id: None,
                created_by: AgentId([0u8; 16]),
                assigned_to: AgentId([idx as u8; 16]),
                kind: TaskKind::Solo,
                subject: line.to_string(),
                body: format!("Execute: {line}"),
                status: TaskStatus::Queued,
                result: None,
                error: None,
                created_at_unix_s: now(),
                updated_at_unix_s: now(),
                expires_at_unix_s: 0,
                budget: Budget {
                    actions: 10,
                    wall_seconds: 300,
                },
            };
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Assign a task to the appropriate lead.
    pub async fn assign_task(&self, task: &Task) -> Result<Assignment, String> {
        let topology = self.topology.read().await;

        // Find lead with capacity.
        for (domain, lead) in topology.leads.iter() {
            if lead.active_workers < lead.max_workers {
                return Ok(Assignment {
                    task_id: task.id.clone(),
                    lead_name: lead.name.clone(),
                    domain: *domain,
                });
            }
        }

        Err("all leads at capacity".to_string())
    }

    /// Check stagnation across all leads.
    pub async fn check_stagnation(&self, outputs: &[String]) -> bool {
        if outputs.len() < 3 {
            return false;
        }
        let recent = &outputs[outputs.len().saturating_sub(3)..];
        let normalized: Vec<String> = recent
            .iter()
            .map(|s| s.trim().to_lowercase())
            .collect();
        !normalized.is_empty() && normalized.iter().all(|s| !s.is_empty() && s == &normalized[0])
    }

    /// Get fleet status.
    pub async fn fleet_status(&self) -> FleetStatus {
        let topology = self.topology.read().await;
        let total_leads = topology.leads.len();
        let total_capacity: usize = topology
            .leads
            .values()
            .map(|l| l.max_workers)
            .sum();
        let active_workers: usize = topology
            .leads
            .values()
            .map(|l| l.active_workers)
            .sum();

        FleetStatus {
            coordinator_id: topology.coordinator_id.clone(),
            total_leads,
            total_capacity,
            active_workers,
            available_slots: total_capacity.saturating_sub(active_workers),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub task_id: String,
    pub lead_name: String,
    pub domain: Domain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetStatus {
    pub coordinator_id: String,
    pub total_leads: usize,
    pub total_capacity: usize,
    pub active_workers: usize,
    pub available_slots: usize,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn coordinator_registers_leads() {
        let ledger = Arc::new(InMemoryLedger::new());
        let claim_store = Arc::new(InMemoryClaimStore::new());
        let coord = Coordinator::new("coord_1".to_string(), ledger, claim_store);

        coord
            .register_lead(Domain::Kernel, "lead_kernel", "http://localhost:8001")
            .await
            .unwrap();
        coord
            .register_lead(Domain::Inference, "lead_inference", "http://localhost:8002")
            .await
            .unwrap();

        let status = coord.fleet_status().await;
        assert_eq!(status.total_leads, 2);
        assert_eq!(status.total_capacity, 10); // 5 per lead
    }

    #[tokio::test]
    async fn coordinator_decomposes_plan() {
        let ledger = Arc::new(InMemoryLedger::new());
        let claim_store = Arc::new(InMemoryClaimStore::new());
        let coord = Coordinator::new("coord_1".to_string(), ledger, claim_store);

        coord
            .register_lead(Domain::Kernel, "lead_kernel", "http://localhost:8001")
            .await
            .unwrap();
        coord
            .register_lead(Domain::Inference, "lead_inference", "http://localhost:8002")
            .await
            .unwrap();

        let goal = "Refactor organ.rs\nExtract domain types\nRun tests";
        let tasks = coord.decompose_plan(goal).await.unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[tokio::test]
    async fn coordinator_assigns_to_lead_with_capacity() {
        let ledger = Arc::new(InMemoryLedger::new());
        let claim_store = Arc::new(InMemoryClaimStore::new());
        let coord = Coordinator::new("coord_1".to_string(), ledger, claim_store);

        coord
            .register_lead(Domain::Kernel, "lead_kernel", "http://localhost:8001")
            .await
            .unwrap();

        let task = Task {
            id: "t1".to_string(),
            parent_task_id: None,
            created_by: AgentId([0u8; 16]),
            assigned_to: AgentId([1u8; 16]),
            kind: TaskKind::Solo,
            subject: "test".to_string(),
            body: "test".to_string(),
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

        let assignment = coord.assign_task(&task).await.unwrap();
        assert_eq!(assignment.lead_name, "lead_kernel");
    }

    #[tokio::test]
    async fn coordinator_detects_stagnation() {
        let ledger = Arc::new(InMemoryLedger::new());
        let claim_store = Arc::new(InMemoryClaimStore::new());
        let coord = Coordinator::new("coord_1".to_string(), ledger, claim_store);

        let outputs = vec![
            "response A".to_string(),
            "response A".to_string(),
            "response A".to_string(),
        ];
        assert!(coord.check_stagnation(&outputs).await);

        let outputs_ok = vec![
            "response A".to_string(),
            "response B".to_string(),
            "response C".to_string(),
        ];
        assert!(!coord.check_stagnation(&outputs_ok).await);
    }

    #[test]
    fn span_of_control_bounds() {
        assert!((MIN_SPAN_OF_CONTROL..=MAX_SPAN_OF_CONTROL).contains(&5));
        assert!(!(MIN_SPAN_OF_CONTROL..=MAX_SPAN_OF_CONTROL).contains(&8));
    }
}
