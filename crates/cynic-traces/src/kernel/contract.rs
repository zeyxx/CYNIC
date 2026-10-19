//! Layer contracts.
//!
//! Strict port of `apps/cynic-data/src/cynic_data/kernel/contract.py`.
//! A layer is a contract with a guarantee and a DECLARED loss.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Capture,
    Canonical,
    Model,
    Projection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    Authoritative,
    Derived,
}

/// An explicit, named loss a layer admits to inflicting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossDeclaration {
    pub field: String,
    pub reason: String,
}

/// The guarantee a layer makes, and what it admits to losing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerContract {
    pub layer: Layer,
    pub authority: Authority,
    pub guarantee: String,
    pub reads_from: Option<Layer>,
    pub declared_losses: Vec<LossDeclaration>,
}

impl LayerContract {
    pub fn declares_loss_of(&self, field_name: &str) -> bool {
        self.declared_losses.iter().any(|l| l.field == field_name)
    }
}

pub fn capture_contract() -> LayerContract {
    LayerContract {
        layer: Layer::Capture,
        authority: Authority::Authoritative,
        guarantee: "Exact bytes emitted by a harness, immutable and content-addressed.".into(),
        reads_from: None,
        declared_losses: vec![],
    }
}

pub fn canonical_contract() -> LayerContract {
    LayerContract {
        layer: Layer::Canonical,
        authority: Authority::Authoritative,
        guarantee: "Typed harness-agnostic facts: turns, tools, reasoning, edges, state, human signal. No semantic loss versus Capture.".into(),
        reads_from: Some(Layer::Capture),
        declared_losses: vec![],
    }
}

pub fn model_contract() -> LayerContract {
    LayerContract {
        layer: Layer::Model,
        authority: Authority::Derived,
        guarantee: "The trajectory hypergraph, materialized and queryable.".into(),
        reads_from: Some(Layer::Canonical),
        declared_losses: vec![],
    }
}

pub fn projection_contract() -> LayerContract {
    LayerContract {
        layer: Layer::Projection,
        authority: Authority::Derived,
        guarantee: "A consumer view. Reconstructible from Model.".into(),
        reads_from: Some(Layer::Model),
        declared_losses: vec![],
    }
}
