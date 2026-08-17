//! Multi-Session Concurrency and Multi-Model Co-Residence Arbitration.

use super::memory_estimator::ModelArchDescriptor;
use super::types::TopologyAssessment;
use cynic_protocol::{ContextTier, KvQuantization, TopologyStrategy};

/// Computes the co-resident memory feasibility of running multiple models concurrently.
pub fn assess_coresident_topology(
    models: &[(&str, u64, f32, ContextTier, KvQuantization, u32)], // (name, weight_bytes, approx_params_b, ctx, quant, slots)
    hardware_available_bytes: u64,
    system_reserve_bytes: u64,
) -> TopologyAssessment {
    let mut total_footprint: u64 = 0;
    let mut active_names = Vec::new();

    for (name, weight_bytes, params_b, ctx, quant, slots) in models {
        let arch = ModelArchDescriptor::for_parameter_count(*params_b);
        let model_footprint = arch.estimate_total_footprint(*weight_bytes, *ctx, *quant, 512, *slots);
        total_footprint += model_footprint;
        active_names.push(name.to_string());
    }

    let meets_budget = if hardware_available_bytes > system_reserve_bytes {
        total_footprint <= (hardware_available_bytes - system_reserve_bytes)
    } else {
        false
    };

    TopologyAssessment {
        strategy: if models.len() > 1 {
            TopologyStrategy::CoResident
        } else {
            TopologyStrategy::DedicatedSingle
        },
        active_models: active_names,
        total_ram_footprint_bytes: total_footprint,
        swap_overhead_secs: None,
        concurrency_throughput_agg_tok_sec: 0.0,
        meets_fleet_safety_budget: meets_budget,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_coresident_router_and_worker_budget() {
        let models = vec![
            ("MiniCPM-1B", 1_100_000_000, 1.2, ContextTier::Ctx4k, KvQuantization::Q8_0, 1),
            ("Qwen-14B", 9_500_000_000, 14.0, ContextTier::Ctx16k, KvQuantization::Q8_0, 1),
        ];
        let available_ram = 27 * 1024 * 1024 * 1024;
        let reserve = 4 * 1024 * 1024 * 1024;

        let assessment = assess_coresident_topology(&models, available_ram, reserve);
        assert!(assessment.meets_fleet_safety_budget, "Co-resident 1B + 14B should fit in 27 GB RAM");
        assert_eq!(assessment.strategy, TopologyStrategy::CoResident);
    }
}
