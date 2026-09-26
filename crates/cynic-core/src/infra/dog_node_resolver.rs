use cynic_inference::application::node_runtime_ops::{
    DogNodeBinding, dog_to_node as resolve_dog_to_node,
};

use crate::infra::config::BackendConfig;

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
