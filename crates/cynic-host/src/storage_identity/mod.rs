mod enrollment;
mod lease;
mod port;
mod secret_adapter;
mod surreal_admin;
mod types;

pub(crate) use enrollment::enroll_storage_identity;
pub(crate) use lease::StorageIdentityEnrollmentLease;
pub(crate) use port::{StorageIdentityAdminPort, StorageIdentitySecretPort};
pub(crate) use secret_adapter::LocalStorageIdentitySecretAdapter;
pub(crate) use surreal_admin::SurrealStorageIdentityAdminAdapter;
pub(crate) use types::{
    StorageAdminObservation, StorageIdentityEnrollmentReport, StorageIdentityStage,
};
