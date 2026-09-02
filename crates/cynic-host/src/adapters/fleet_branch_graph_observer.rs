//! Fleet branch-graph observer — Fleet<BranchGraph>Observer emitting fleet report.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use cynic_protocol::branch_graph::BranchGraph;

use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};

pub const FLEET_BRANCH_GRAPH_SCHEMA: &str = "cynic.fleet_branch_graph.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="outcome", rename_all="snake_case")]
pub enum FleetBranchGraphNode { Observed{ node_id: String, role: String, ip: String, report: BranchGraph }, Failed{ node_id: String, role: String, ip: String, error: String } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetBranchGraphReport { pub schema: String, pub corpus_unix_s: u64, pub total_nodes: usize, pub observed_nodes: usize, pub failed_nodes: usize, pub nodes: Vec<FleetBranchGraphNode>, pub residual_unknowns: Vec<String>, pub budget: Option<cynic_protocol::task_pipeline::EnergyBudget>, pub poll_cost_ms: Option<u64> }

fn poll_budget_ms()->u64{ std::env::var("CYNIC_POLL_BUDGET_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(5000) }

pub struct FleetBranchGraphObserver<R: ArgvRunner>{ registry_path: String, ssh_target_template: String, runner: Arc<R> }
impl FleetBranchGraphObserver<LocalArgvRunner>{ pub fn new(p: impl Into<String>)->Self{ Self::with_runner(p, LocalArgvRunner) } }
impl<R: ArgvRunner> FleetBranchGraphObserver<R>{
    pub fn with_runner(p: impl Into<String>, runner: R)->Self{ Self{ registry_path:p.into(), ssh_target_template:"cynic@{ip}".into(), runner: Arc::new(runner) } }
    pub fn with_ssh_target(mut self,t: impl Into<String>)->Self{ self.ssh_target_template=t.into(); self }
    fn ssh_target_for(&self, ip:&str)->String{ self.ssh_target_template.replace("{ip}", ip) }
    pub fn observe(&self)->FleetBranchGraphReport{
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let start = std::time::Instant::now();
        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut nodes = Vec::with_capacity(discovery.nodes.len());
        let (mut obs, mut fail)=(0,0);
        for d in &discovery.nodes {
            let is_local = hostname==d.node_id || hostname==d.ip;
            let node = if is_local {
                match observe_local_branch_graph() { Ok(r)=>{ obs+=1; FleetBranchGraphNode::Observed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), report:r } }, Err(e)=>{ fail+=1; FleetBranchGraphNode::Failed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), error:e } } }
            } else {
                let tgt=self.ssh_target_for(&d.ip);
                match observe_remote_branch_graph(self.runner.clone(), &tgt) { Ok(r)=>{ obs+=1; FleetBranchGraphNode::Observed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), report:r } }, Err(e)=>{ fail+=1; FleetBranchGraphNode::Failed{ node_id:d.node_id.clone(), role:d.role.clone(), ip:d.ip.clone(), error:e } } }
            };
            nodes.push(node);
        }
        let cost=start.elapsed().as_millis() as u64;
        let mut ru=discovery.residual_unknowns; if cost>poll_budget_ms(){ ru.push("poll_cost_exceeded_budget".into()); }
        FleetBranchGraphReport{ schema:FLEET_BRANCH_GRAPH_SCHEMA.into(), corpus_unix_s:now, total_nodes:discovery.nodes.len(), observed_nodes:obs, failed_nodes:fail, nodes, residual_unknowns:ru, budget:Some(cynic_protocol::task_pipeline::EnergyBudget{ wall_time_ms:5000, inference_tokens:None, cpu_ms:Some(5000), ram_bytes_s:None }), poll_cost_ms:Some(cost) }
    }
}
fn observe_local_branch_graph()->Result<BranchGraph,String>{
    use crate::domain::branch_graph::probe::{enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph};
    use crate::process_observation::capture_command;
    let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let root=capture_command("git",&["rev-parse","HEAD"]).ok().filter(|c| c.success).map(|c| c.stdout.trim().to_string()).unwrap_or_default();
    let refs=capture_command("git",&["for-each-ref","--format=%(refname:short)|%(objectname)|%(upstream:short)|%(committerdate:unix)","refs/heads/"]).ok().filter(|c|c.success).map(|c|c.stdout).unwrap_or_default();
    let log=capture_command("git",&["log","--all","--pretty=format:%H %P"]).ok().filter(|c|c.success).map(|c|c.stdout).unwrap_or_default();
    Ok(enrich_with_parents(parse_git_for_each_ref(&refs,&root,now), &parse_git_log_graph(&log)))
}
fn observe_remote_branch_graph<R: ArgvRunner>(runner: Arc<R>, target:&str)->Result<BranchGraph,String>{
    use crate::domain::branch_graph::probe::{enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph};
    let run=|cmd:&[&str]|{ let mut a=vec!["ssh","-o","BatchMode=yes","-o","ConnectTimeout=5","-o","StrictHostKeyChecking=yes",target]; a.extend_from_slice(cmd); let o=runner.run(&a); if o.success(){Ok(o.stdout)} else {Err(o.spawn_error.unwrap_or(o.stderr))} };
    let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let root=run(&["git","rev-parse","HEAD"]).unwrap_or_default().trim().to_string();
    let refs=run(&["git","for-each-ref","--format=%(refname:short)|%(objectname)|%(upstream:short)|%(committerdate:unix)","refs/heads/"]).unwrap_or_default();
    let log=run(&["git","log","--all","--pretty=format:%H %P"]).unwrap_or_default();
    Ok(enrich_with_parents(parse_git_for_each_ref(&refs,&root,now), &parse_git_log_graph(&log)))
}
