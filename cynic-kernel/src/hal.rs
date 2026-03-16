//! MuscleHAL gRPC service — dispatches inference through BackendRouter.

use crate::cynic_v2::muscle_hal_server::MuscleHal;
use crate::cynic_v2::{
    MctsInferenceRequest, MctsInferenceResponse,
    InferenceToken, HalProfile, PulseRequest,
};
use crate::backend::{InferenceRequest, InferenceRouter};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;

pub struct MuscleService {
    router: Arc<dyn InferenceRouter>,
}

impl MuscleService {
    pub fn new(router: Arc<dyn InferenceRouter>) -> Self {
        Self { router }
    }
}

fn to_domain(req: &MctsInferenceRequest) -> InferenceRequest {
    InferenceRequest {
        trace_id: req.meta.as_ref().map(|m| m.trace_id.clone()).unwrap_or_default(),
        system_prompt: req.system_prompt.clone(),
        context: req.context.clone(),
        num_branches: req.num_branches.max(1) as u32,
        temperature: req.temperature,
        model_hint: None,
    }
}

#[tonic::async_trait]
impl MuscleHal for MuscleService {
    type StreamInferenceStream = ReceiverStream<Result<InferenceToken, Status>>;

    async fn request_inference(
        &self,
        request: Request<MctsInferenceRequest>,
    ) -> Result<Response<MctsInferenceResponse>, Status> {
        let req = request.into_inner();
        let meta = req.meta.clone();
        let domain_req = to_domain(&req);
        let n = req.num_branches.max(1) as u32;

        let trace = meta.as_ref().map(|m| m.trace_id.as_str()).unwrap_or("none");
        println!("[MuscleHAL] Inference | trace={} branches={}", trace, n);

        let result = if n > 1 {
            self.router.fan_out(domain_req, n).await
        } else {
            self.router.route(domain_req).await
        };

        match result {
            Ok(resp) => {
                println!("[MuscleHAL] OK | model={} latency={}ms hypotheses={}",
                    resp.model_used, resp.latency_ms, resp.hypotheses.len());
                Ok(Response::new(MctsInferenceResponse {
                    meta,
                    hypotheses: resp.hypotheses,
                    latency_ms: resp.latency_ms as f32,
                    model_used: resp.model_used,
                }))
            }
            Err(e) => {
                println!("[MuscleHAL] ERROR | {}", e);
                Err(Status::unavailable(e.to_string()))
            }
        }
    }

    async fn stream_inference(
        &self,
        request: Request<MctsInferenceRequest>,
    ) -> Result<Response<Self::StreamInferenceStream>, Status> {
        let req = request.into_inner();
        let meta = req.meta.clone();
        let domain_req = to_domain(&req);

        let trace = meta.as_ref().map(|m| m.trace_id.as_str()).unwrap_or("none");
        println!("[MuscleHAL] StreamInference | trace={}", trace);

        let (tx, rx) = mpsc::channel(32);
        let router = Arc::clone(&self.router);

        tokio::spawn(async move {
            match router.route(domain_req).await {
                Ok(resp) => {
                    // Emit the full response as a single final token
                    // TODO: Wire to actual streaming when backend supports SSE
                    for hypothesis in &resp.hypotheses {
                        let _ = tx.send(Ok(InferenceToken {
                            meta: meta.clone(),
                            token: hypothesis.clone(),
                            is_final: false,
                            latency_ms: resp.latency_ms as f32,
                            model_used: resp.model_used.clone(),
                        })).await;
                    }
                    let _ = tx.send(Ok(InferenceToken {
                        meta,
                        token: String::new(),
                        is_final: true,
                        latency_ms: resp.latency_ms as f32,
                        model_used: resp.model_used,
                    })).await;
                }
                Err(e) => {
                    let _ = tx.send(Err(Status::unavailable(e.to_string()))).await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_active_hal(
        &self,
        request: Request<PulseRequest>,
    ) -> Result<Response<HalProfile>, Status> {
        let req = request.into_inner();
        Ok(Response::new(HalProfile {
            meta: req.meta,
            backend: "llama.cpp".to_string(),
            gpu_name: "discovered-at-runtime".to_string(),
            vram_used_gb: 0.0,
            vram_total_gb: 0.0,
        }))
    }
}
