use std::sync::Arc;

use crate::domain::node_control::{NodeControlRuntime, NodeControlStorePort};

pub fn load_node_control_runtime(
    store: Option<Arc<dyn NodeControlStorePort>>,
) -> Option<Arc<NodeControlRuntime>> {
    store.map(|store| Arc::new(NodeControlRuntime { store }))
}
