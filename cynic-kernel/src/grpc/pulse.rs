use crate::cynic_v2::k_pulse_server::KPulse;
use crate::cynic_v2::{
    SomaticPulse, PulseRequest, HeresyNotice,
    SovereigntyAdvice, PublishAck, HealthReport, HealthState,
};
use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct PulseService {}

#[tonic::async_trait]
impl KPulse for PulseService {
    type StreamPulseStream = ReceiverStream<Result<SomaticPulse, Status>>;

    async fn stream_pulse(
        &self,
        request: Request<PulseRequest>,
    ) -> Result<Response<Self::StreamPulseStream>, Status> {
        let req = request.into_inner();
        let meta = req.meta.clone();
        
        println!("[Pulse] New Somatic Stream request | Node: {}", 
            meta.as_ref().map(|m| m.node_id.as_str()).unwrap_or("unknown")
        );

        let (tx, rx) = mpsc::channel(10);
        
        // Simulation of somatic feedback loop
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
            loop {
                interval.tick().await;
                let pulse = SomaticPulse {
                    meta: meta.clone(),
                    somatic_score: 0.85,
                    cognitive_score: 0.92,
                    logic_score: 0.99,
                    immune_score: 1.0,
                    universal_score: 0.618,
                    active_backend: "Vulkan".to_string(),
                    tokens_per_sec: 45.0,
                    metabolic_drift: 0.02,
                };
                if tx.send(Ok(pulse)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn report_heresy(
        &self,
        request: Request<HeresyNotice>,
    ) -> Result<Response<PublishAck>, Status> {
        let heresy = request.into_inner();
        let meta = heresy.meta.clone();

        println!("[Omniscience] HERESY DETECTED: [{}] {} | Severity: {:?}",
            heresy.heresy_type,
            heresy.diagnosis,
            HealthState::try_from(heresy.severity).unwrap_or(HealthState::HealthUnknown)
        );

        Ok(Response::new(PublishAck {
            meta,
            success: true,
        }))
    }

    async fn get_sovereign_advice(
        &self,
        request: Request<PulseRequest>,
    ) -> Result<Response<SovereigntyAdvice>, Status> {
        let req = request.into_inner();

        Ok(Response::new(SovereigntyAdvice {
            meta: req.meta,
            recommendations: vec![
                "Industrialize VRAM allocation.".to_string(),
                "Shift sidecar memory to D: volumes.".to_string(),
                "Stabilize gRPC trace-IDs across Ring 3.".to_string()
            ],
            optimization_target: "Metabolic Efficiency".to_string(),
        }))
    }

    async fn get_health(
        &self,
        request: Request<PulseRequest>,
    ) -> Result<Response<HealthReport>, Status> {
        let req = request.into_inner();

        // TODO: Wire to real backend health from router when PulseService holds Arc<BackendRouter>
        let services: HashMap<String, i32> = HashMap::from([
            ("vascular".to_string(), HealthState::HealthHealthy as i32),
            ("pulse".to_string(), HealthState::HealthHealthy as i32),
            ("cognitive".to_string(), HealthState::HealthHealthy as i32),
            ("muscle".to_string(), HealthState::HealthUnknown as i32),
        ]);

        Ok(Response::new(HealthReport {
            meta: req.meta,
            overall: HealthState::HealthHealthy as i32,
            services,
            backends: HashMap::new(),
        }))
    }
}
