//! Re-export and trait adapter for SurrealDB storage identity administration.
//!
//! The implementation lives in `cynic_surreal`; this module only adapts it to
//! the host-local `StorageIdentityAdminPort`.

use cynic_persistence::identity::StorageIdentitySpec;
use cynic_surreal::SurrealStorageIdentityAdminAdapter;

use crate::secret_material::SecretMaterial;

use super::{StorageAdminObservation, StorageIdentityAdminPort};

pub(crate) use cynic_surreal::SurrealStorageIdentityAdminAdapter;

impl StorageIdentityAdminPort for SurrealStorageIdentityAdminAdapter {
    fn observe(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
    ) -> StorageAdminObservation {
        self.observe(spec, admin_secret)
    }

    fn create(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
        runtime_secret: &SecretMaterial,
    ) -> Result<(), String> {
        self.create(spec, admin_secret, runtime_secret)
    }

    fn remove(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
    ) -> Result<(), String> {
        self.remove(spec, admin_secret)
    }

    fn verify_runtime(
        &self,
        spec: &StorageIdentitySpec,
        runtime_secret: &SecretMaterial,
    ) -> Result<(), String> {
        self.verify_runtime(spec, runtime_secret)
    }
}