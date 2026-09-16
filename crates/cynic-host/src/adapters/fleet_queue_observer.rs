//! Fleet queue observer — Fleet<Queue>Observer emitting fleet QueuePlan report.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use cynic_protocol::queue::QueuePlan;
use crate::adapters::local_queue::LocalQueueAdapter;
use crate::adapters::remote_queue::RemoteQueueAdapter;
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};
pub const FLEET_QUEUE_SCHEMA: &str = "cynic.fleet_queue.v1";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="outcome", rename_all="snake_case")]
pub enum FleetQueueNode { Observed{ node_id: String, role: String, ip: String, report: QueuePlan }, Failed{ node_id: String, role: String, ip: String, error: String } }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetQueueReport { pub schema: String, pub corpus_unix_s: u64, pub total_nodes: usize, pub observed_nodes: usize, pub failed_nodes: usize, pub nodes: Vec<FleetQueueNode>, pub residual_unknowns: Vec<String>, pub budget: Option<cynic_protocol::task_pipeline::EnergyBudget>, pub poll_cost_ms: Option<u64> }
fn poll_budget_ms()->u64{ std::env::var("CYNIC_POLL_BUDGET_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(5000) }
pub struct FleetQueueObserver<R: ArgvRunner>{ registry_path: String, ssh_target_template: String, runner: Arc<R> }
impl FleetQueueObserver<LocalArgvRunner>{ pub fn new(p: impl Into<String>)->Self{ Self::with_runner(p, LocalArgvRunner) } }
impl<R: ArgvRunner> FleetQueueObserver<R>{
    pub fn with_runner(p: impl Into<String>, runner: R)->Self{ Self{ registry_path:p.into(), ssh_target_template:"cynic@{ip}".into(), runner: Arc::new(runner) } }
    pub fn with_ssh_target(mut self,t: impl Into<String>)->Self{ self.ssh_target_template=t.into(); self }
    fn ssh_target_for(&self, ip:&str)->String{ self.ssh_target_template.replace("{ip}", ip) }
    pub fn observe(&self)->FleetQueueReport{
        let now=SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let start=std::time::Instant::now();
        let discovery=FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname=observed_hostname();
        let mut nodes=Vec::with_capacity(discovery.nodes.len());
        let (mut obs,mut fail)=(0,0);
        for d in &discovery.nodes {
            let is_local=hostname==d.node_id||hostname==d.ip;
            let node=if is_local{ match LocalQueueAdapter.observe() { Ok(r)=>{obs+=1; FleetQueueNode::Observed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), report:r }}, Err(e)=>{fail+=1; FleetQueueNode::Failed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), error:e } } } } else { let tgt=self.ssh_target_for(&d.ip); match RemoteQueueAdapter::new(self.runner.clone(), tgt.clone()){ Ok(a)=> match a.observe(){ Ok(r)=>{obs+=1; FleetQueueNode::Observed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), report:r }}, Err(e)=>{fail+=1; FleetQueueNode::Failed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), error:e }}} , Err(e)=>{fail+=1; FleetQueueNode::Failed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), error:e }}} };
            nodes.push(node);
        }
        let cost=start.elapsed().as_millis() as u64; let mut ru=discovery.residual_unknowns; if cost>poll_budget_ms(){ ru.push("poll_cost_exceeded_budget".into()); }
        FleetQueueReport{ schema:FLEET_QUEUE_SCHEMA.into(), corpus_unix_s:now, total_nodes:discovery.nodes.len(), observed_nodes:obs, failed_nodes:fail, nodes, residual_unknowns:ru, budget:Some(cynic_protocol::task_pipeline::EnergyBudget{ wall_time_ms:5000, inference_tokens:None, cpu_ms:Some(5000), ram_bytes_s:None }), poll_cost_ms:Some(cost) }
    }
}
