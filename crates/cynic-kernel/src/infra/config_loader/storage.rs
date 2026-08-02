use crate::infra::config_schema::{BackendsFile, DefaultsEntry, StorageFile};
use crate::infra::config_types::{
    DogThreshold, DogThresholds, StorageConfig, StorageIdentityConfig,
};
use cynic_protocol::{SecretClass, SecretProvider, SecretReference};

pub fn load_storage_config(path: &std::path::Path) -> StorageConfig {
    let defaults = StorageConfig::default();
    let from_toml = match super::helpers::load_file(path) {
        Some(content) => match toml::from_str::<StorageFile>(&content) {
            Ok(file) => file.storage,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "storage.toml parse failed — falling back to explicit storage env vars"
                );
                None
            }
        },
        None => {
            tracing::warn!(
                path = %path.display(),
                "storage config file unavailable — falling back to explicit storage env vars"
            );
            None
        }
    };

    let url = from_toml
        .as_ref()
        .and_then(|s| s.url.clone())
        .or_else(|| std::env::var("SURREALDB_URL").ok())
        .unwrap_or(defaults.url);

    let namespace = from_toml
        .as_ref()
        .and_then(|s| s.namespace.clone())
        .or_else(|| std::env::var("SURREALDB_NS").ok())
        .unwrap_or(defaults.namespace);

    let database = from_toml
        .as_ref()
        .and_then(|s| s.database.clone())
        .or_else(|| std::env::var("SURREALDB_DB").ok())
        .unwrap_or(defaults.database);
    let allow_loopback = from_toml
        .as_ref()
        .and_then(|storage| storage.allow_loopback)
        .unwrap_or(false);

    StorageConfig {
        url,
        allow_loopback,
        namespace,
        database,
        identity: load_storage_identity(from_toml.as_ref(), &defaults.identity),
    }
}

fn load_storage_identity(
    entry: Option<&crate::infra::config_schema::StorageEntry>,
    defaults: &StorageIdentityConfig,
) -> StorageIdentityConfig {
    let username = entry
        .and_then(|storage| storage.username.clone())
        .or_else(|| std::env::var("SURREALDB_USER").ok())
        .unwrap_or_else(|| defaults.username.clone());
    let env_name = entry
        .and_then(|storage| storage.secret_env.clone())
        .or_else(|| std::env::var("SURREALDB_PASS_ENV").ok())
        .unwrap_or_else(|| "SURREALDB_PASS".to_string());
    let credential_id = entry
        .and_then(|storage| storage.os_credential.clone())
        .or_else(|| std::env::var("SURREALDB_PASS_CREDENTIAL").ok())
        .unwrap_or_else(|| "storage-runtime-password".to_string());

    let mut secret_sources = vec![secret_reference(
        "storage-runtime-env",
        SecretProvider::ProcessEnvironment,
        env_name,
    )];

    if let Some(path) = entry
        .and_then(|storage| storage.secret_file.clone())
        .or_else(|| std::env::var("SURREALDB_PASS_FILE").ok())
    {
        secret_sources.push(secret_reference(
            "storage-runtime-explicit-file",
            SecretProvider::PrincipalOwnedFile,
            path,
        ));
    }

    let state_root = std::env::var("CYNIC_STATE_ROOT").unwrap_or_else(|_| "/var/lib/cynic".into());
    secret_sources.push(secret_reference(
        "storage-runtime-principal-file",
        SecretProvider::PrincipalOwnedFile,
        std::path::Path::new(&state_root)
            .join(".config/cynic/storage-runtime-password")
            .display()
            .to_string(),
    ));

    let config_root = std::env::var("CYNIC_CONFIG_ROOT").unwrap_or_else(|_| "/etc/cynic".into());
    secret_sources.push(secret_reference(
        "storage-runtime-installed-file",
        SecretProvider::PrincipalOwnedFile,
        std::path::Path::new(&config_root)
            .join("secrets/storage-runtime-password")
            .display()
            .to_string(),
    ));
    secret_sources.push(secret_reference(
        "storage-runtime-os-credential",
        SecretProvider::OsCredentialStore,
        credential_id,
    ));

    StorageIdentityConfig {
        username,
        secret_sources,
    }
}

fn secret_reference(id: &str, provider: SecretProvider, locator: String) -> SecretReference {
    SecretReference {
        id: id.to_string(),
        class: SecretClass::RuntimeMachineIdentity,
        provider,
        locator,
    }
}

pub fn load_system_contract(path: &std::path::Path) -> crate::domain::contract::SystemContract {
    let expected_dogs = super::backend::load_backends(path)
        .into_iter()
        .map(|backend| backend.name)
        .collect();

    crate::domain::contract::SystemContract::new(expected_dogs, true)
}

pub fn load_dog_thresholds(path: &std::path::Path) -> DogThresholds {
    let mut result = DogThresholds::default();

    let Some(content) = super::helpers::load_file(path) else {
        tracing::warn!(path = %path.display(), "cannot read dog thresholds from backends.toml");
        return result;
    };

    let file: BackendsFile = match toml::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "invalid TOML in dog thresholds"
            );
            return result;
        }
    };

    if let Some(defaults_entry) = file.defaults {
        defaults_from_entry(&mut result, &defaults_entry);
    }

    for (dog_name, entry) in file.dog {
        result.dogs.insert(
            dog_name.clone(),
            DogThreshold {
                dog_name: dog_name.clone(),
                kind: entry.kind.unwrap_or_else(|| "inference".to_string()),
                enabled: entry.enabled.unwrap_or(true),
                failure_rate_threshold: entry.failure_rate_threshold,
                collapse_threshold: entry.collapse_threshold,
                api_error_threshold: entry.api_error_threshold,
                evaluation_timeout_secs: entry.evaluation_timeout_secs,
                priority: entry.priority,
                quota_status: entry.quota_status,
                quota_pattern: entry.quota_pattern,
                skip_reason: entry.skip_reason,
                skip_until: entry.skip_until,
            },
        );
    }

    if let Some(error_entry) = file.error_detection {
        if let Some(patterns) = error_entry.quota_patterns {
            result.error_detection.quota_patterns = patterns;
        }
        if let Some(patterns) = error_entry.transient_patterns {
            result.error_detection.transient_patterns = patterns;
        }
        if let Some(patterns) = error_entry.critical_patterns {
            result.error_detection.critical_patterns = patterns;
        }
    }

    if let Some(verdict_entry) = file.verdict {
        if let Some(v) = verdict_entry.min_voters {
            result.verdict.min_voters = v;
        }
        if let Some(v) = verdict_entry.min_confident_voters {
            result.verdict.min_confident_voters = v;
        }
        if let Some(v) = verdict_entry.contested_threshold {
            result.verdict.contested_threshold = v;
        }
    }

    if let Some(circuit_entry) = file.circuit {
        if let Some(v) = circuit_entry.open_on_consecutive_failures {
            result.circuit.open_on_consecutive_failures = v;
        }
        if let Some(v) = circuit_entry.open_on_failure_rate {
            result.circuit.open_on_failure_rate = v;
        }
        if let Some(v) = circuit_entry.open_duration_secs {
            result.circuit.open_duration_secs = v;
        }
        if let Some(v) = circuit_entry.half_open_eval_budget {
            result.circuit.half_open_eval_budget = v;
        }
        if let Some(v) = circuit_entry.failure_rate_recovery_window_secs {
            result.circuit.failure_rate_recovery_window_secs = v;
        }
    }

    if let Some(monitoring_entry) = file.monitoring {
        if let Some(v) = monitoring_entry.failure_rate_check_interval_secs {
            result.monitoring.failure_rate_check_interval_secs = v;
        }
        if let Some(v) = monitoring_entry.alert_threshold_critical {
            result.monitoring.alert_threshold_critical = v;
        }
        if let Some(v) = monitoring_entry.alert_threshold_warning {
            result.monitoring.alert_threshold_warning = v;
        }
        if let Some(v) = monitoring_entry.quota_exhaustion_alert {
            result.monitoring.quota_exhaustion_alert = v;
        }
        if let Some(v) = monitoring_entry.circuit_breaker_alert {
            result.monitoring.circuit_breaker_alert = v;
        }
    }

    if let Some(ks) = &file.kscore {
        if let Some(v) = ks.weight_diamond_hands {
            result.kscore.weight_diamond_hands = v;
        }
        if let Some(v) = ks.weight_organic_growth {
            result.kscore.weight_organic_growth = v;
        }
        if let Some(v) = ks.weight_longevity {
            result.kscore.weight_longevity = v;
        }
        if let Some(v) = ks.accumulator_threshold {
            result.kscore.accumulator_threshold = v;
        }
        if let Some(v) = ks.holder_threshold {
            result.kscore.holder_threshold = v;
        }
        if let Some(v) = ks.reducer_threshold {
            result.kscore.reducer_threshold = v;
        }
        if let Some(v) = ks.top_n_wallets {
            result.kscore.top_n_wallets = v as usize;
        }
        if let Some(v) = ks.swap_history_limit {
            result.kscore.swap_history_limit = v as usize;
        }
    }

    if let Some(conv) = &file.convergence {
        if let Some(v) = conv.threshold {
            result.convergence.threshold = v;
        }
        if let Some(v) = conv.window_hours {
            result.convergence.window_hours = v;
        }
        if let Some(v) = conv.cooldown_hours {
            result.convergence.cooldown_hours = v;
        }
    }

    crate::klog!(
        "[config] Dog thresholds loaded: {} dogs configured, kscore weights: DH={:.2}/OG={:.2}/L={:.2}, convergence: threshold={}/window={}h/cooldown={}h, error patterns: quota({}) transient({}) critical({})",
        result.dogs.len(),
        result.kscore.weight_diamond_hands,
        result.kscore.weight_organic_growth,
        result.kscore.weight_longevity,
        result.convergence.threshold,
        result.convergence.window_hours,
        result.convergence.cooldown_hours,
        result.error_detection.quota_patterns.len(),
        result.error_detection.transient_patterns.len(),
        result.error_detection.critical_patterns.len()
    );

    result
}

fn defaults_from_entry(result: &mut DogThresholds, defaults_entry: &DefaultsEntry) {
    if let Some(v) = defaults_entry.failure_rate_threshold {
        result.defaults.failure_rate_threshold = v;
    }
    if let Some(v) = defaults_entry.api_error_threshold {
        result.defaults.api_error_threshold = v;
    }
    if let Some(v) = defaults_entry.transient_error_max {
        result.defaults.transient_error_max = v;
    }
    if let Some(v) = defaults_entry.quota_grace_period_secs {
        result.defaults.quota_grace_period_secs = v;
    }
    if let Some(v) = defaults_entry.evaluation_timeout_secs {
        result.defaults.evaluation_timeout_secs = v;
    }
}
