use cynic_protocol::{
    STORAGE_IDENTITY_OBSERVATION_SCHEMA, StorageEvidenceState, StorageIdentityDecision,
    StorageIdentityObservation, StorageIdentitySpec, reconcile_storage_identity,
};

use crate::runtime_secret_install::RuntimeSecretInstallSpec;
use crate::secret_material::SecretMaterial;

use super::{
    StorageIdentityAdminPort, StorageIdentityEnrollmentReport, StorageIdentitySecretPort,
    StorageIdentityStage,
};

pub(crate) fn enroll_storage_identity(
    spec: &StorageIdentitySpec,
    admin_secret: &SecretMaterial,
    admin: &dyn StorageIdentityAdminPort,
    secret_store: &dyn StorageIdentitySecretPort,
) -> StorageIdentityEnrollmentReport {
    let file_spec = RuntimeSecretInstallSpec::storage_runtime();
    let admin_before = admin.observe(spec, admin_secret);
    let secret_before = secret_store.observe(&file_spec);
    let observed = observation(
        spec,
        admin_before.database_identity,
        admin_before.role,
        if secret_before.present {
            StorageEvidenceState::Present
        } else if secret_before.unknowns.is_empty() {
            StorageEvidenceState::Absent
        } else {
            StorageEvidenceState::Unknown
        },
        StorageEvidenceState::Absent,
        true,
        admin_before
            .unknowns
            .iter()
            .chain(&secret_before.unknowns)
            .cloned()
            .collect(),
    );
    let plan = reconcile_storage_identity(spec, &observed);
    if plan.decision != StorageIdentityDecision::EnrollAtomically {
        let residual = plan.blockers.clone();
        return StorageIdentityEnrollmentReport::blocked(plan, residual);
    }

    let runtime_secret = match SecretMaterial::generate_runtime_identity() {
        Ok(secret) => secret,
        Err(error) => return StorageIdentityEnrollmentReport::blocked(plan, vec![error]),
    };
    let mut report = StorageIdentityEnrollmentReport {
        schema: "cynic.storage_identity_enrollment_report.v1",
        status: "applying".to_string(),
        identity_id: spec.identity_id.clone(),
        plan,
        mutated: false,
        compensated: false,
        compensation_failed: false,
        secret_exposed: false,
        stages: Vec::new(),
        proofs: Vec::new(),
        residual_unknowns: Vec::new(),
    };

    if let Err(error) = admin.create(spec, admin_secret, &runtime_secret) {
        fail(&mut report, "database_identity_create", error);
        compensate(
            &mut report,
            spec,
            admin_secret,
            admin,
            &runtime_secret,
            None,
            secret_store,
        );
        return report;
    }
    report.mutated = true;
    complete(&mut report, "database_identity_create");

    let created = admin.observe(spec, admin_secret);
    if created.database_identity != StorageEvidenceState::Present || created.role != Some(spec.role)
    {
        fail(
            &mut report,
            "database_identity_read_back",
            "database identity or role did not converge".to_string(),
        );
        compensate(
            &mut report,
            spec,
            admin_secret,
            admin,
            &runtime_secret,
            None,
            secret_store,
        );
        return report;
    }
    complete(&mut report, "database_identity_read_back");

    if let Err(error) = admin.verify_runtime(spec, &runtime_secret) {
        fail(
            &mut report,
            "database_identity_authentication_read_back",
            error,
        );
        compensate(
            &mut report,
            spec,
            admin_secret,
            admin,
            &runtime_secret,
            None,
            secret_store,
        );
        return report;
    }
    complete(&mut report, "database_identity_authentication_read_back");

    if let Err(error) = secret_store.install(&file_spec, &runtime_secret) {
        fail(&mut report, "local_credential_install", error);
        compensate(
            &mut report,
            spec,
            admin_secret,
            admin,
            &runtime_secret,
            None,
            secret_store,
        );
        return report;
    }
    complete(&mut report, "local_credential_install");

    if let Err(error) = admin.verify_runtime(spec, &runtime_secret) {
        fail(&mut report, "runtime_authentication_read_back", error);
        compensate(
            &mut report,
            spec,
            admin_secret,
            admin,
            &runtime_secret,
            Some((&file_spec, &runtime_secret)),
            secret_store,
        );
        return report;
    }
    complete(&mut report, "runtime_authentication_read_back");

    let final_admin = admin.observe(spec, admin_secret);
    let final_secret = secret_store.observe(&file_spec);
    let final_observation = observation(
        spec,
        final_admin.database_identity,
        final_admin.role,
        if final_secret.present {
            StorageEvidenceState::Present
        } else {
            StorageEvidenceState::Unknown
        },
        StorageEvidenceState::Present,
        true,
        final_admin
            .unknowns
            .iter()
            .chain(&final_secret.unknowns)
            .cloned()
            .collect(),
    );
    let final_plan = reconcile_storage_identity(spec, &final_observation);
    if final_plan.decision != StorageIdentityDecision::Observe {
        fail(
            &mut report,
            "final_reconciliation",
            "post-action state did not converge".to_string(),
        );
        compensate(
            &mut report,
            spec,
            admin_secret,
            admin,
            &runtime_secret,
            Some((&file_spec, &runtime_secret)),
            secret_store,
        );
        return report;
    }

    report.status = "completed".to_string();
    report.proofs = final_plan.expected_proofs;
    complete(&mut report, "final_reconciliation");
    report
}

fn observation(
    spec: &StorageIdentitySpec,
    database_identity: StorageEvidenceState,
    observed_role: Option<cynic_protocol::StorageIdentityRole>,
    local_credential: StorageEvidenceState,
    runtime_authentication: StorageEvidenceState,
    admin_authority_present: bool,
    unknowns: Vec<String>,
) -> StorageIdentityObservation {
    StorageIdentityObservation {
        schema: STORAGE_IDENTITY_OBSERVATION_SCHEMA.to_string(),
        identity_id: spec.identity_id.clone(),
        database_identity,
        observed_role,
        local_credential,
        runtime_authentication,
        admin_authority_present,
        secret_exposed: false,
        observed_at_ms: now_ms(),
        unknowns,
    }
}

fn compensate(
    report: &mut StorageIdentityEnrollmentReport,
    spec: &StorageIdentitySpec,
    admin_secret: &SecretMaterial,
    admin: &dyn StorageIdentityAdminPort,
    runtime_secret: &SecretMaterial,
    local: Option<(&RuntimeSecretInstallSpec, &SecretMaterial)>,
    secret_store: &dyn StorageIdentitySecretPort,
) {
    if let Err(error) = admin.verify_runtime(spec, runtime_secret) {
        report.compensation_failed = true;
        fail(
            report,
            "database_identity_compensation_ownership_read_back",
            error,
        );
        report.residual_unknowns.push(
            "database identity ownership is unproven; destructive compensation refused".to_string(),
        );
        if local.is_some() {
            report.residual_unknowns.push(
                "matching local credential retained because database identity may still require it"
                    .to_string(),
            );
        }
        report.status = "compensation_failed".to_string();
        return;
    }
    complete(report, "database_identity_compensation_ownership_read_back");
    match admin.remove(spec, admin_secret) {
        Ok(()) => complete(report, "database_identity_compensation"),
        Err(error) => {
            report.compensation_failed = true;
            fail(report, "database_identity_compensation", error);
            report
                .residual_unknowns
                .push("database_identity_may_remain_active".to_string());
            return;
        }
    }
    if let Some((file_spec, material)) = local {
        match secret_store.remove_if_matches(file_spec, material) {
            Ok(observed) if !observed.present => complete(report, "local_credential_compensation"),
            Ok(_) => {
                report.compensation_failed = true;
                report
                    .residual_unknowns
                    .push("local_credential_remains_present".to_string());
            }
            Err(error) => {
                report.compensation_failed = true;
                fail(report, "local_credential_compensation", error);
            }
        }
    }
    report.compensated = !report.compensation_failed;
    report.status = if report.compensation_failed {
        "compensation_failed"
    } else {
        "compensated"
    }
    .to_string();
}

fn complete(report: &mut StorageIdentityEnrollmentReport, id: &str) {
    report.stages.push(StorageIdentityStage {
        id: id.to_string(),
        status: "completed".to_string(),
        detail: "read-back complete".to_string(),
    });
}

fn fail(report: &mut StorageIdentityEnrollmentReport, id: &str, detail: String) {
    report.status = "blocked".to_string();
    report.stages.push(StorageIdentityStage {
        id: id.to_string(),
        status: "blocked".to_string(),
        detail,
    });
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use cynic_protocol::{STORAGE_IDENTITY_SPEC_SCHEMA, StorageIdentityRole};

    use crate::runtime_secret_install::{RuntimeSecretInstallReport, RuntimeSecretObservation};

    use super::*;

    struct FakeAdmin {
        present: Cell<bool>,
        fail_create: bool,
        fail_remove: bool,
        verify_calls: Cell<usize>,
        fail_verify_calls: Vec<usize>,
    }

    impl FakeAdmin {
        fn healthy() -> Self {
            Self {
                present: Cell::new(false),
                fail_create: false,
                fail_remove: false,
                verify_calls: Cell::new(0),
                fail_verify_calls: Vec::new(),
            }
        }
    }

    impl StorageIdentityAdminPort for FakeAdmin {
        fn observe(
            &self,
            spec: &StorageIdentitySpec,
            _admin_secret: &SecretMaterial,
        ) -> super::super::StorageAdminObservation {
            super::super::StorageAdminObservation {
                database_identity: if self.present.get() {
                    StorageEvidenceState::Present
                } else {
                    StorageEvidenceState::Absent
                },
                role: self.present.get().then_some(spec.role),
                unknowns: Vec::new(),
            }
        }

        fn create(
            &self,
            _spec: &StorageIdentitySpec,
            _admin_secret: &SecretMaterial,
            _runtime_secret: &SecretMaterial,
        ) -> Result<(), String> {
            if self.fail_create {
                return Err("injected create failure".to_string());
            }
            self.present.set(true);
            Ok(())
        }

        fn remove(
            &self,
            _spec: &StorageIdentitySpec,
            _admin_secret: &SecretMaterial,
        ) -> Result<(), String> {
            if self.fail_remove {
                return Err("injected remove failure".to_string());
            }
            self.present.set(false);
            Ok(())
        }

        fn verify_runtime(
            &self,
            _spec: &StorageIdentitySpec,
            _runtime_secret: &SecretMaterial,
        ) -> Result<(), String> {
            let call = self.verify_calls.get() + 1;
            self.verify_calls.set(call);
            if self.fail_verify_calls.contains(&call) || !self.present.get() {
                Err("injected runtime verification failure".to_string())
            } else {
                Ok(())
            }
        }
    }

    struct FakeSecretStore {
        material: RefCell<Option<Vec<u8>>>,
        fail_install: bool,
        fail_remove: bool,
    }

    impl FakeSecretStore {
        fn healthy() -> Self {
            Self {
                material: RefCell::new(None),
                fail_install: false,
                fail_remove: false,
            }
        }

        fn observation(&self, spec: &RuntimeSecretInstallSpec) -> RuntimeSecretObservation {
            let present = self.material.borrow().is_some();
            RuntimeSecretObservation {
                identity_id: spec.identity_id.clone(),
                path: spec.target.display().to_string(),
                present,
                regular_file: present.then_some(true),
                symlink: present.then_some(false),
                owner_id: present.then_some(spec.owner_id),
                group_id: if present { spec.group_id } else { None },
                mode: present.then_some(spec.mode),
                non_empty: present.then_some(true),
                secret_exposed: false,
                unknowns: Vec::new(),
            }
        }
    }

    impl StorageIdentitySecretPort for FakeSecretStore {
        fn observe(&self, spec: &RuntimeSecretInstallSpec) -> RuntimeSecretObservation {
            self.observation(spec)
        }

        fn install(
            &self,
            spec: &RuntimeSecretInstallSpec,
            material: &SecretMaterial,
        ) -> Result<RuntimeSecretInstallReport, String> {
            if self.fail_install {
                return Err("injected local install failure".to_string());
            }
            let before = self.observation(spec);
            self.material
                .replace(Some(material.expose_bytes().to_vec()));
            let after = self.observation(spec);
            Ok(RuntimeSecretInstallReport {
                schema: "cynic.runtime_secret_install_report.v1",
                status: "completed",
                identity_id: spec.identity_id.clone(),
                mutated: true,
                before,
                after,
                required_secret_class: "RuntimeMachineIdentity",
                secret_exposed: false,
                proofs: Vec::new(),
                rollback_intent: "fake rollback".to_string(),
                unknowns: Vec::new(),
            })
        }

        fn remove_if_matches(
            &self,
            spec: &RuntimeSecretInstallSpec,
            material: &SecretMaterial,
        ) -> Result<RuntimeSecretObservation, String> {
            if self.fail_remove {
                return Err("injected local remove failure".to_string());
            }
            let matches = self
                .material
                .borrow()
                .as_deref()
                .is_some_and(|stored| stored == material.expose_bytes());
            if !matches {
                return Err("fake refused non-matching material".to_string());
            }
            self.material.replace(None);
            Ok(self.observation(spec))
        }
    }

    fn spec() -> StorageIdentitySpec {
        StorageIdentitySpec {
            schema: STORAGE_IDENTITY_SPEC_SCHEMA.to_string(),
            identity_id: "storage-runtime".to_string(),
            endpoint: "http://127.0.0.1:8000".to_string(),
            namespace: "cynic".to_string(),
            database: "main".to_string(),
            username: "cynic-runtime".to_string(),
            role: StorageIdentityRole::Editor,
            token_duration: "15m".to_string(),
            session_duration: "12h".to_string(),
        }
    }

    #[test]
    fn enrollment_proves_database_file_and_runtime_authentication() {
        let admin = FakeAdmin::healthy();
        let secrets = FakeSecretStore::healthy();

        let report = enroll_storage_identity(
            &spec(),
            &SecretMaterial::fixture("admin-secret"),
            &admin,
            &secrets,
        );

        assert_eq!(report.status, "completed");
        assert!(admin.present.get());
        assert!(secrets.material.borrow().is_some());
        assert!(!report.secret_exposed);
        assert!(
            report
                .proofs
                .contains(&"runtime_authentication_read_back".to_string())
        );
    }

    #[test]
    fn local_install_failure_compensates_database_identity() {
        let admin = FakeAdmin::healthy();
        let secrets = FakeSecretStore {
            fail_install: true,
            ..FakeSecretStore::healthy()
        };

        let report = enroll_storage_identity(
            &spec(),
            &SecretMaterial::fixture("admin-secret"),
            &admin,
            &secrets,
        );

        assert_eq!(report.status, "compensated");
        assert!(report.compensated);
        assert!(!admin.present.get());
        assert!(secrets.material.borrow().is_none());
    }

    #[test]
    fn runtime_auth_failure_compensates_database_then_local_secret() {
        let admin = FakeAdmin {
            fail_verify_calls: vec![2],
            ..FakeAdmin::healthy()
        };
        let secrets = FakeSecretStore::healthy();

        let report = enroll_storage_identity(
            &spec(),
            &SecretMaterial::fixture("admin-secret"),
            &admin,
            &secrets,
        );

        assert_eq!(report.status, "compensated");
        assert!(!admin.present.get());
        assert!(secrets.material.borrow().is_none());
        let database_compensation = report
            .stages
            .iter()
            .position(|stage| stage.id == "database_identity_compensation")
            .unwrap();
        let local_compensation = report
            .stages
            .iter()
            .position(|stage| stage.id == "local_credential_compensation")
            .unwrap();
        assert!(database_compensation < local_compensation);
    }

    #[test]
    fn unproven_compensation_retains_both_sides_as_explicit_residual_state() {
        let admin = FakeAdmin {
            fail_verify_calls: vec![2, 3],
            ..FakeAdmin::healthy()
        };
        let secrets = FakeSecretStore::healthy();

        let report = enroll_storage_identity(
            &spec(),
            &SecretMaterial::fixture("admin-secret"),
            &admin,
            &secrets,
        );

        assert_eq!(report.status, "compensation_failed");
        assert!(admin.present.get());
        assert!(secrets.material.borrow().is_some());
        assert!(report.compensation_failed);
        assert!(
            report
                .residual_unknowns
                .iter()
                .any(|unknown| unknown.contains("destructive compensation refused"))
        );
    }
}
