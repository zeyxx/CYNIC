mod apply;
mod decision;
mod filesystem;
mod observe;
mod registry;
mod rollback;
mod types;

pub(crate) use apply::apply_dns_activation;
pub(crate) use decision::plan_dns_activation;
pub(crate) use observe::observe_dns_activation;
pub(crate) use registry::resolve_dns_activation_spec;

#[cfg(test)]
mod tests;
