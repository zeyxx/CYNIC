use crate::cynic_v2::muscle_hal_server::MuscleHal;
use crate::cynic_v2::{
    MCTSInferenceRequest, MCTSInferenceResponse, 
    HALProfile, PulseRequest
};
use crate::storage::CynicStorage;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct MuscleService {
    pub storage: Arc<CynicStorage>,
}

impl MuscleService {
    pub fn new(storage: Arc<CynicStorage>) -> Self {
        Self { storage }
    }
}

#[tonic::async_trait]
impl MuscleHal for MuscleService {
    async fn request_inference(
        &self,
        request: Request<MCTSInferenceRequest>,
    ) -> Result<Response<MCTSInferenceResponse>, Status> {
        let req = request.into_inner();
        let meta = req.meta.clone();

        println!("[MuscleHAL] Inference Request | Trace: {}", 
            meta.as_ref().map(|m| m.trace_id.as_str()).unwrap_or("none")
        );

        // Simulation for Phase 1
        Ok(Response::new(MCTSInferenceResponse {
            meta,
            hypotheses: vec!["Reflexive thought simulated.".to_string()],
            latency_ms: 42.0,
            model_used: "phi-3-mini".to_string(),
        }))
    }

    async fn get_active_hal(
        &self,
        request: Request<PulseRequest>,
    ) -> Result<Response<HALProfile>, Status> {
        let req = request.into_inner();
        
        Ok(Response::new(HALProfile {
            meta: req.meta,
            backend: "Vulkan".to_string(),
            gpu_name: "AMD Radeon".to_string(),
            vram_used_gb: 1.2,
            vram_total_gb: 8.0,
        }))
    }
}
