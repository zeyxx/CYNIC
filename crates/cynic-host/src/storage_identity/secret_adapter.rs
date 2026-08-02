use crate::runtime_secret_install::{
    RuntimeSecretInstallReport, RuntimeSecretInstallSpec, RuntimeSecretObservation,
    install_runtime_secret, observe_runtime_secret, remove_runtime_secret_if_matches,
};
use crate::secret_material::SecretMaterial;

use super::StorageIdentitySecretPort;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct LocalStorageIdentitySecretAdapter;

impl StorageIdentitySecretPort for LocalStorageIdentitySecretAdapter {
    fn observe(&self, spec: &RuntimeSecretInstallSpec) -> RuntimeSecretObservation {
        let mut observed = observe_runtime_secret(spec);
        if spec.group_id.is_none() {
            observed
                .unknowns
                .push("runtime_principal_group_is_absent".to_string());
        }
        observed
    }

    fn install(
        &self,
        spec: &RuntimeSecretInstallSpec,
        material: &SecretMaterial,
    ) -> Result<RuntimeSecretInstallReport, String> {
        install_runtime_secret(spec, material)
    }

    fn remove_if_matches(
        &self,
        spec: &RuntimeSecretInstallSpec,
        material: &SecretMaterial,
    ) -> Result<RuntimeSecretObservation, String> {
        remove_runtime_secret_if_matches(spec, material)
    }
}
