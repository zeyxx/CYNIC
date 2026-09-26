//! Thin CLI facade: parse → reconcile → delegate → render.
//! No policy tables. Delegates to injected port.
use crate::ports::{BuildRunnerPort, EvidenceObserver, SubstrateDiscovery};
use crate::types::BuildRequest;

pub fn build_check(req: BuildRequest, runner: &impl BuildRunnerPort, obs: &impl EvidenceObserver, disc: &impl SubstrateDiscovery) -> crate::types::BuildProof {
    let target = disc.resolve();
    let plan = req.reconcile(target.clone());
    let result = runner.run_build(&plan);
    let proof = crate::types::BuildProof { request: req, result, builder: target };
    obs.emit(proof.clone());
    proof
}
