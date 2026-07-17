use cynic_node::inference_node_ops::{DogNodeBinding, dog_to_node as resolve_dog_to_node};

use crate::infra::config::BackendConfig;

static DOG_NODE_BINDINGS: std::sync::OnceLock<Vec<DogNodeBinding>> = std::sync::OnceLock::new();

pub fn bindings_from_backends(backends: &[BackendConfig]) -> Vec<DogNodeBinding> {
    backends
        .iter()
        .filter_map(|config| {
            Some(DogNodeBinding {
                dog: config.name.clone(),
                node: config.fleet_node.clone()?,
            })
        })
        .collect()
}

pub fn resolve_dog_node(dog: &str, backends: &[BackendConfig]) -> String {
    let bindings = bindings_from_backends(backends);
    resolve_dog_to_node(dog, &bindings)
}

pub fn resolve_runtime_dog_node(dog: &str) -> String {
    let bindings = DOG_NODE_BINDINGS.get_or_init(|| {
        let path = crate::bootstrap::config::resolve_kernel_backends_path();
        let backends = crate::infra::config::load_backends(&path);
        bindings_from_backends(&backends)
    });
    resolve_dog_to_node(dog, bindings)
}
