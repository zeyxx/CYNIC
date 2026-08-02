use cynic_protocol::StorageIdentitySpec;

use crate::runtime_secret_install::{
    RuntimeSecretInstallReport, RuntimeSecretInstallSpec, RuntimeSecretObservation,
};
use crate::secret_material::SecretMaterial;

use super::StorageAdminObservation;

pub(crate) trait StorageIdentityAdminPort {
    fn observe(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
    ) -> StorageAdminObservation;

    fn create(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
        runtime_secret: &SecretMaterial,
    ) -> Result<(), String>;

    fn remove(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
    ) -> Result<(), String>;

    fn verify_runtime(
        &self,
        spec: &StorageIdentitySpec,
        runtime_secret: &SecretMaterial,
    ) -> Result<(), String>;
}

pub(crate) trait StorageIdentitySecretPort {
    fn observe(&self, spec: &RuntimeSecretInstallSpec) -> RuntimeSecretObservation;

    fn install(
        &self,
        spec: &RuntimeSecretInstallSpec,
        material: &SecretMaterial,
    ) -> Result<RuntimeSecretInstallReport, String>;

    fn remove_if_matches(
        &self,
        spec: &RuntimeSecretInstallSpec,
        material: &SecretMaterial,
    ) -> Result<RuntimeSecretObservation, String>;
}
