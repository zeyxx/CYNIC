<<<<<<< HEAD
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
=======
use std::time::Duration;
use std::path::Path;
use std::sync::Arc;
>>>>>>> github/main

use cynic_inference::{
    BuiltinToolInvoker, ChatEndpointSource, ChatService, ChatSessionStore, FsChatSessionStore,
    HttpOpenAiChatClient, LocalRuntimeServiceAny, LocalRuntimeStatus, MockChatClient,
    bind_preferred_chat_model, default_tool_specs, probe_live_model_ids, resolve_chat_endpoint,
    resolve_chat_state_dir, select_preferred_alias,
};

pub(crate) struct ChatRuntime {
    pub store: Arc<FsChatSessionStore>,
    pub service: Option<ChatService>,
    pub ready: bool,
    pub use_mock: bool,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub source: String,
    pub detail: String,
    pub session_root: String,
    pub available_models: Vec<String>,
    pub context_turns: Option<usize>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub reasoning_profile: String,
    pub context_source: String,
}

#[derive(Debug)]
struct LocalModelSelection {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    readiness_deferred: bool,
    log_path: Option<String>,
}

const DEFAULT_CHAT_LOCAL_READY_DEFER_GB: f64 = 4.0;
const CHAT_LOCAL_WAIT_FOR_READY_ENV: &str = "CYNIC_CHAT_WAIT_FOR_READY";
const CHAT_LOCAL_READY_DEFER_GB_ENV: &str = "CYNIC_CHAT_LOCAL_READY_DEFER_GB";

pub(crate) fn should_defer_local_readiness_for_chat(size_bytes: Option<u64>) -> bool {
    match parse_env_bool(CHAT_LOCAL_WAIT_FOR_READY_ENV) {
        Some(wait) => !wait,
        None => match size_bytes {
            Some(bytes) => {
                let threshold_gb = std::env::var(CHAT_LOCAL_READY_DEFER_GB_ENV)
                    .ok()
                    .and_then(|raw| raw.parse::<f64>().ok())
                    .unwrap_or(DEFAULT_CHAT_LOCAL_READY_DEFER_GB);
                if threshold_gb <= 0.0 {
                    false
                } else {
                    let size_gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                    size_gb >= threshold_gb
                }
            }
            None => false,
        },
    }
}

fn parse_env_bool(name: &str) -> Option<bool> {
    std::env::var(name).ok().and_then(|raw| {
        let raw = raw.to_ascii_lowercase();
        match raw.as_str() {
            "1" | "true" | "on" | "yes" | "y" => Some(true),
            "0" | "false" | "off" | "no" | "n" => Some(false),
            _ => None,
        }
    })
}

fn extract_log_path_from_message(message: &str) -> Option<String> {
    message
        .split(';')
<<<<<<< HEAD
        .find_map(|part| {
            part.trim()
                .strip_prefix("log=")
                .map(|value| value.to_string())
        })
=======
        .find_map(|part| part.trim().strip_prefix("log=").map(|value| value.to_string()))
>>>>>>> github/main
        .or_else(|| {
            message
                .split_once("log=")
                .map(|(_, value)| value.split_whitespace().next().unwrap_or("").to_string())
        })
        .filter(|s| !s.is_empty())
}

/// Build principal session store + chat service (async endpoint resolve).
pub(crate) async fn build_chat_runtime(
    mock: bool,
    prefer_ods: bool,
    model: Option<&str>,
    auto_init_local: bool,
) -> Result<ChatRuntime, String> {
    let params = infer_default_chat_params();

    let root = if let Ok(p) = std::env::var("CYNIC_CHAT_STATE") {
        std::path::PathBuf::from(p)
    } else {
        resolve_chat_state_dir()
    };
    let store = FsChatSessionStore::new(&root).map_err(|e| e.to_string())?;
    let store = Arc::new(store);

    let env_mock = std::env::var("CYNIC_CHAT_MOCK")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let use_mock = mock || env_mock;
    if use_mock {
        let client: Arc<dyn cynic_inference::ChatCompletionClient> =
            Arc::new(MockChatClient::plain("(mock ready)"));
        let tools = Arc::new(BuiltinToolInvoker);
        let (specs, max_tools) = chat_tool_policy();
        let service = ChatService::new(
            store.clone() as Arc<dyn ChatSessionStore>,
            client,
            tools,
            specs,
            cynic_inference::ChatServiceConfig::new(
                Some("mock://cynic-chat".into()),
                Some("mock-model".into()),
                max_tools,
                true,
            ),
        );
        return Ok(ChatRuntime {
            store,
            service: Some(service),
            ready: true,
            use_mock: true,
            endpoint: Some("mock://cynic-chat".into()),
            model: Some("mock-model".into()),
            source: "mock".into(),
            detail: "explicit mock".into(),
            session_root: root.display().to_string(),
            available_models: vec!["mock-model".to_string()],
            context_turns: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            reasoning_profile: "mock".into(),
            context_source: "static".into(),
        });
    }

    let requested_model = model
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    let local_selection = if !prefer_ods {
        if let Some(selector) = requested_model.as_deref() {
            select_local_model(selector, auto_init_local).await?
        } else if auto_init_local {
            select_auto_local_model(auto_init_local).await
        } else {
            None
        }
    } else {
        None
    };

    let (endpoint, selected_model, source, detail, api_key, available_models) =
        if let Some(local) = local_selection {
            let model = local.model;
            let detail = {
                let mut detail = format!(
                    "local alias auto-started{}",
                    requested_model
                        .as_deref()
                        .map(|model| format!(" ({model})"))
                        .unwrap_or_default()
                );
                if local.readiness_deferred {
                    detail.push_str(" (readiness deferred)");
                }
                if let Some(path) = local.log_path.as_ref() {
                    detail.push_str(&format!(" log={path}"));
                }
                detail
            };
            (
                local.endpoint,
                model.clone(),
                "local".to_string(),
                detail,
                local.api_key,
                vec![model],
            )
        } else {
            let resolution = resolve_chat_endpoint(prefer_ods).await;
            if !resolution.ready {
                return Ok(ChatRuntime {
                    store,
                    service: None,
                    ready: false,
                    use_mock: false,
                    endpoint: None,
                    model: None,
                    source: format!("{:?}", resolution.source).to_lowercase(),
                    detail: resolution.detail,
                    session_root: root.display().to_string(),
                    available_models: vec![],
                    context_turns: params.context_turns,
                    max_tokens: params.max_tokens,
                    temperature: params.temperature,
                    top_p: params.top_p,
                    reasoning_profile: params
                        .reasoning_profile
                        .clone()
                        .unwrap_or_else(|| "balanced".into()),
                    context_source: "resolution".into(),
                });
            }

            let endpoint = resolution
                .endpoint
                .clone()
                .ok_or_else(|| "endpoint missing despite ready".to_string())?;
            let discovered_models =
                probe_live_model_ids(&endpoint, resolution.api_key.as_deref()).await;

            let selected_model = requested_model
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    std::env::var("CYNIC_CHAT_MODEL")
                        .ok()
                        .filter(|s| !s.is_empty())
                })
                .or_else(|| discovered_models.first().cloned().filter(|s| !s.is_empty()))
                .or_else(|| resolution.model.clone().filter(|s| !s.is_empty()))
                .unwrap_or_else(|| "default".into());

            let source = match resolution.source {
                ChatEndpointSource::Env => "env",
                ChatEndpointSource::InferenceConfig => "inference_config",
                ChatEndpointSource::Dns => "dns",
                ChatEndpointSource::Ods => "ods",
                ChatEndpointSource::NativeProbe => "native",
                ChatEndpointSource::Mock => "mock",
                ChatEndpointSource::None => "none",
            }
            .to_string();

            (
                endpoint,
                selected_model,
                source,
                format!(
                    "{} (dynamic models: {})",
                    resolution.detail,
                    discovered_models.len()
                ),
                resolution.api_key,
                discovered_models,
            )
        };

    let client: Arc<dyn cynic_inference::ChatCompletionClient> = Arc::new(
        HttpOpenAiChatClient::with_api_key(&endpoint, &selected_model, api_key)
            .map_err(|e| e.to_string())?
            .with_runtime_params(params.max_tokens, params.temperature, params.top_p),
    );
    let tools = Arc::new(BuiltinToolInvoker);
    // Tools off by default: small/local models emit raw tool XML when advertised.
    // Opt-in: CYNIC_CHAT_TOOLS=1
    let (specs, max_tools) = chat_tool_policy();
    let service = ChatService::new(
        store.clone() as Arc<dyn ChatSessionStore>,
        client,
        tools,
        specs,
        cynic_inference::ChatServiceConfig::new(
            Some(endpoint.clone()),
            Some(selected_model.clone()),
            max_tools,
            true,
        )
        .context_turns(params.context_turns)
        .max_tokens(params.max_tokens)
        .temperature(params.temperature)
        .top_p(params.top_p)
        .reasoning_profile(
            params
                .reasoning_profile
                .clone()
                .unwrap_or_else(|| "balanced".into()),
        ),
    );

    Ok(ChatRuntime {
        store,
        service: Some(service),
        ready: true,
        use_mock: false,
        endpoint: Some(endpoint),
        model: Some(selected_model),
        source,
        detail,
        session_root: root.display().to_string(),
        available_models,
        context_turns: params.context_turns,
        max_tokens: params.max_tokens,
        temperature: params.temperature,
        top_p: params.top_p,
        reasoning_profile: params
            .reasoning_profile
            .clone()
            .unwrap_or_else(|| "balanced".into()),
        context_source: params.context_source.clone(),
    })
}

async fn select_local_model(
    selector: &str,
    auto_init_local: bool,
) -> Result<Option<LocalModelSelection>, String> {
    let selector = selector.trim();
    if selector.is_empty() {
        return Ok(None);
    }

    let service = LocalRuntimeServiceAny::from_env();
    if let Err(error) = cynic_inference::ensure_profiles_from_vault() {
        eprintln!("local vault reconcile skipped: {error}");
    }

    let inventory = service
        .list_inventory()
        .map_err(|error| format!("local inventory unavailable: {error}"))?;

    let Some(resolved_alias) = resolve_selector_to_alias(selector, &inventory) else {
        if selector_looks_like_local(selector) {
            return Err(local_selector_error(selector, &inventory));
        }
        return Ok(None);
    };

    let profile_size = inventory
        .models
        .iter()
        .find(|p| p.alias == resolved_alias)
        .and_then(|p| p.size_bytes);
    let wait_for_ready = if auto_init_local {
        !should_defer_local_readiness_for_chat(profile_size)
    } else {
        true
    };

    bind_preferred_local_alias(&service, &inventory, &resolved_alias, wait_for_ready)
        .await
        .map(Some)
        .ok_or_else(|| {
            format!(
                "local model alias `{resolved_alias}` could not be resolved to a running endpoint"
            )
        })
}

async fn select_auto_local_model(auto_init_local: bool) -> Option<LocalModelSelection> {
    let service = LocalRuntimeServiceAny::from_env();

    let _ = cynic_inference::ensure_profiles_from_vault().map_err(|error| {
        eprintln!("local vault reconcile skipped: {error}");
    });

    let inventory = service.list_inventory().ok()?;
    let preferred_alias = select_preferred_alias(&inventory.models)?;
    let profile_size = inventory
        .models
        .iter()
        .find(|p| p.alias == preferred_alias)
        .and_then(|p| p.size_bytes);
    let wait_for_ready = if auto_init_local {
        !should_defer_local_readiness_for_chat(profile_size)
    } else {
        true
    };

    bind_preferred_local_alias(&service, &inventory, &preferred_alias, wait_for_ready).await
}

async fn bind_preferred_local_alias(
    service: &LocalRuntimeServiceAny,
    inventory: &cynic_inference::LocalModelInventory,
    alias: &str,
    wait_for_ready: bool,
) -> Option<LocalModelSelection> {
    let alias = alias.trim();
    if alias.is_empty() {
        return None;
    }

    let status = service.status().ok();
    let status_endpoint = status
        .as_ref()
        .and_then(|status| local_endpoint_from_status(status, alias));
    let status_log_path = status
        .as_ref()
        .and_then(|status| status.message.as_deref())
        .and_then(extract_log_path_from_message);

    let mut endpoint = status_endpoint.or_else(|| profile_endpoint_for_alias(inventory, alias));
    if endpoint.is_none() {
        let report = service.start_with_readiness(alias, wait_for_ready).ok()?;
        let readiness_deferred = report
            .message
            .as_ref()
            .is_some_and(|message| message.contains("ready=deferred"));
        let log_path = report
            .message
            .as_deref()
            .and_then(extract_log_path_from_message)
            .or(status_log_path);
        if !report.ok {
            eprintln!(
                "local model alias `{alias}` start failed: {}",
                report
                    .message
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| "unknown".into())
            );
            return None;
        }

        endpoint = report
            .status
            .endpoint
            .clone()
            .or_else(|| profile_endpoint_for_alias(inventory, alias));
        if endpoint.is_none() {
            endpoint = await_local_endpoint(service, alias).await;
        }
        let endpoint = endpoint?;
<<<<<<< HEAD
        return bind_alias_to_chat_model(&endpoint, alias, &log_path, readiness_deferred).await;
=======
        return bind_alias_to_chat_model(
            &endpoint,
            alias,
            &log_path,
            readiness_deferred,
        )
        .await;
>>>>>>> github/main
    }

    let endpoint = endpoint?;
    bind_alias_to_chat_model(&endpoint, alias, &status_log_path, false).await
}

async fn bind_alias_to_chat_model(
    endpoint: &str,
    alias: &str,
    log_path: &Option<String>,
    readiness_deferred: bool,
) -> Option<LocalModelSelection> {
    match bind_preferred_chat_model(alias, Some(endpoint), "cynic chat --model").await {
        Ok(bound) => Some(LocalModelSelection {
            endpoint: endpoint.to_string(),
            model: bound.model_id,
            api_key: bound.api_key,
            readiness_deferred,
            log_path: log_path.clone(),
        }),
        Err(error) => {
            eprintln!("local alias `{alias}` bind failed, fallback model id = alias: {error}");
            Some(LocalModelSelection {
                endpoint: endpoint.to_string(),
                model: alias.to_string(),
                api_key: None,
                readiness_deferred,
                log_path: log_path.clone(),
            })
        }
    }
}

fn selector_looks_like_local(selector: &str) -> bool {
    let selector = selector.trim();
    if selector.is_empty() {
        return false;
    }
    let lower = selector.to_ascii_lowercase();
    selector.contains(std::path::MAIN_SEPARATOR)
        || selector.contains('/')
        || selector.ends_with(".gguf")
        || lower.contains(".gguf")
        || lower.starts_with("file:")
        || lower.contains("./")
        || lower.contains("../")
        || lower.contains("://")
}
fn local_selector_error(
    selector: &str,
    inventory: &cynic_inference::LocalModelInventory,
) -> String {
    let mut aliases: Vec<&str> = inventory.models.iter().map(|p| p.alias.as_str()).collect();
    aliases.sort_unstable();
    let alias_hint = if aliases.is_empty() {
        "no local aliases found in vault".into()
    } else {
        format!("known local aliases: {}", aliases.join(", "))
    };
    let action_hint = if aliases.is_empty() {
        "run: cynic runtime inference-models --ensure-profiles --json"
    } else {
        "run: cynic runtime inference-vault --json"
    };
    format!(
<<<<<<< HEAD
        "local model selector `{selector}` is not resolvable; it looks like a local GGUF/profile selector, but no matching alias exists in inventory. {alias_hint}. {action_hint}, then cynic models start <alias> --yes"
=======
        "local model selector `{selector}` is not resolvable; it looks like a local GGUF/profile selector, but no matching alias exists in inventory. {alias_hint}. {action_hint}, then cynic runtime inference-start <alias>"
>>>>>>> github/main
    )
}

pub(crate) fn resolve_selector_to_alias<'a>(
    selector: &'a str,
    inventory: &'a cynic_inference::LocalModelInventory,
) -> Option<String> {
    let selector = selector.trim();
    if selector.is_empty() {
        return None;
    }

    let selector_lower = selector.to_ascii_lowercase();
    let selector_basename_lower = Path::new(selector)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_else(|| selector_lower.clone());

    if let Some(profile) = inventory
        .models
        .iter()
        .find(|p| p.alias.eq_ignore_ascii_case(selector))
    {
        return Some(profile.alias.clone());
    }

    if let Some(profile) = inventory.models.iter().find(|p| {
        p.model_path.eq_ignore_ascii_case(selector)
            || p.model_path.to_ascii_lowercase() == selector_lower
    }) {
        return Some(profile.alias.clone());
    }

    if let Some(profile) = inventory.models.iter().find(|p| {
        Path::new(&p.model_path)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(selector_basename_lower.as_str()))
    }) {
        return Some(profile.alias.clone());
    }

    inventory
        .models
        .iter()
        .find(|p| {
            p.model_path.to_ascii_lowercase().ends_with(&selector_lower)
                || p.alias.to_ascii_lowercase().ends_with(&selector_lower)
        })
        .map(|p| p.alias.clone())
}

async fn await_local_endpoint(
    service: &LocalRuntimeServiceAny,
    expected_alias: &str,
) -> Option<String> {
    for attempt in 0..12 {
        if let Ok(status) = service.status() {
            if let Some(endpoint) = local_endpoint_from_status(&status, expected_alias) {
                return Some(endpoint);
            }
        }
        if attempt < 11 {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    None
}

fn profile_endpoint_for_alias(
    inventory: &cynic_inference::LocalModelInventory,
    expected_alias: &str,
) -> Option<String> {
    let expected_alias = expected_alias.trim();
    if expected_alias.is_empty() {
        return None;
    }
    inventory
        .models
        .iter()
        .find(|p| p.alias == expected_alias || p.alias.eq_ignore_ascii_case(expected_alias))
        .map(|p| format!("http://{}:{}/v1", p.host, p.port))
}

fn local_endpoint_from_status(status: &LocalRuntimeStatus, expected_alias: &str) -> Option<String> {
    if let Some(current_alias) = status.alias.as_deref() {
        if current_alias != expected_alias {
            return None;
        }
    }

    status.endpoint.clone().or_else(|| {
        status
            .listen_host
            .as_ref()
            .zip(status.listen_port)
            .map(|(host, port)| format!("http://{host}:{port}/v1"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_inference::{LocalModelProfile, domain::local_runtime::LocalModelSource};

    fn fixture() -> cynic_inference::LocalModelInventory {
        cynic_inference::LocalModelInventory {
            schema: "cynic.inference.local_model_inventory.v1",
            surface: "native",
            vault_dir: "/vault".into(),
            profiles_dir: "/profiles".into(),
            unit_template: "llama-server@.service".into(),
            models: vec![
                LocalModelProfile {
                    alias: "mini-cpm".into(),
                    model_path: "/x/weights/MiniCPM5-1B-Claude-Opus-Fable5-Thinking-Q8_0.gguf"
                        .into(),
                    source: LocalModelSource::Vault,
                    model_exists: true,
                    size_bytes: Some(12),
                    host: "127.0.0.1".into(),
                    port: 8080,
                    ctx_size: 4096,
                    extra_args: String::new(),
                    memory_max: None,
                    cpu_quota: None,
                    loopback_only: true,
                },
                LocalModelProfile {
                    alias: "llama-70b-4bit".into(),
                    model_path: "/x/weights/llama-70b-Q5_K_M.gguf".into(),
                    source: LocalModelSource::Vault,
                    model_exists: true,
                    size_bytes: Some(14),
                    host: "127.0.0.1".into(),
                    port: 8081,
                    ctx_size: 4096,
                    extra_args: String::new(),
                    memory_max: None,
                    cpu_quota: None,
                    loopback_only: true,
                },
            ],
        }
    }

    #[test]
    fn resolve_selector_prefers_alias_exactly() {
        let inv = fixture();
        assert_eq!(
            resolve_selector_to_alias("Mini-CpM", &inv),
            Some("mini-cpm".into())
        );
    }

    #[test]
    fn resolve_selector_prefers_model_path_basename() {
        let inv = fixture();
        assert_eq!(
            resolve_selector_to_alias("MiniCPM5-1B-Claude-Opus-Fable5-Thinking-Q8_0.gguf", &inv),
            Some("mini-cpm".into())
        );
    }

    #[test]
    fn resolve_selector_accepts_full_path_case_insensitive() {
        let inv = fixture();
        assert_eq!(
            resolve_selector_to_alias(
                "/x/weights/minicpm5-1b-claude-opus-fable5-thinking-q8_0.GGUF",
                &inv
            ),
            Some("mini-cpm".into())
        );
    }

    #[test]
    fn selector_looks_local_when_gguf_like_or_path() {
        assert!(selector_looks_like_local("/x/weights/model.gguf"));
        assert!(selector_looks_like_local(
            "MiniCPM5-1B-Claude-Opus-Fable5-Thinking-Q8_0.gguf"
        ));
        assert!(!selector_looks_like_local("mistral-7b-instruct"));
    }

    #[test]
    fn local_selector_error_mentions_known_aliases_or_scan_hint() {
        let inv = fixture();
        let err = local_selector_error("MiniCPM5-1B-Claude-Opus-Fable5-Thinking-Q8_0.gguf", &inv);
        assert!(err.contains("local model selector"));
        assert!(err.contains("mini-cpm"));
        assert!(err.contains("cynic runtime inference-vault --json"));
    }

    #[test]
    fn should_defer_local_readiness_for_chat_for_large_model_sizes() {
        let four_gib = 4_u64 * 1024 * 1024 * 1024;
        assert!(should_defer_local_readiness_for_chat(Some(four_gib)));
        assert!(should_defer_local_readiness_for_chat(Some(four_gib + 1)));
<<<<<<< HEAD
        assert!(should_defer_local_readiness_for_chat(Some(
            4_u64 * 1024 * 1024 * 1024 + 1
        )));
=======
        assert!(should_defer_local_readiness_for_chat(Some(4_u64 * 1024 * 1024 * 1024 + 1)));
>>>>>>> github/main
    }

    #[test]
    fn should_not_defer_local_readiness_for_chat_for_small_model_sizes() {
<<<<<<< HEAD
        assert!(!should_defer_local_readiness_for_chat(Some(
            2_u64 * 1024 * 1024 * 1024
        )));
=======
        assert!(!should_defer_local_readiness_for_chat(Some(2_u64 * 1024 * 1024 * 1024)));
>>>>>>> github/main
        assert!(!should_defer_local_readiness_for_chat(Some(1)));
        assert!(!should_defer_local_readiness_for_chat(None));
    }
}

struct RuntimeChatParams {
    context_turns: Option<usize>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    reasoning_profile: Option<String>,
    context_source: String,
}

fn infer_default_chat_params() -> RuntimeChatParams {
    let profile = std::env::var("CYNIC_CHAT_REASONING_PROFILE")
        .ok()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            let fallback = std::env::var("CYNIC_CHAT_PROFILE").ok();
            fallback
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
        });

    let mut context_turns = parse_usize_var("CYNIC_CHAT_CONTEXT_TURNS");
    let mut max_tokens = parse_u32_var("CYNIC_CHAT_MAX_TOKENS");
    let mut temperature = parse_f32_var("CYNIC_CHAT_TEMPERATURE");
    let mut top_p = parse_f32_var("CYNIC_CHAT_TOP_P");

    let reasoning_profile = if let Some(named) = profile.as_deref() {
        match named {
            "fast" => {
                context_turns = context_turns.or(Some(16));
                max_tokens = max_tokens.or(Some(768));
                temperature = temperature.or(Some(0.35));
                top_p = top_p.or(Some(0.8));
                Some("fast".into())
            }
            "creative" => {
                context_turns = context_turns.or(Some(96));
                max_tokens = max_tokens.or(Some(1536));
                temperature = temperature.or(Some(1.0));
                top_p = top_p.or(Some(0.95));
                Some("creative".into())
            }
            "deep" | "reasoning" => {
                context_turns = context_turns.or(Some(256));
                max_tokens = max_tokens.or(Some(3072));
                temperature = temperature.or(Some(0.65));
                top_p = top_p.or(Some(0.9));
                Some("deep".into())
            }
            "balanced" => Some("balanced".into()),
            _ => Some(named.to_string()),
        }
    } else {
        None
    };

    RuntimeChatParams {
        context_turns,
        max_tokens,
        temperature,
        top_p,
        reasoning_profile,
        context_source: "env".into(),
    }
}

fn parse_u32_var(name: &str) -> Option<u32> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
}

fn parse_f32_var(name: &str) -> Option<f32> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<f32>().ok())
}

fn parse_usize_var(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
}

/// Tool advertising for chat. Default: none (local models mangle tool schema).
/// Enable with `CYNIC_CHAT_TOOLS=1`.
pub(crate) fn chat_tool_policy() -> (Vec<cynic_inference::ToolSpec>, u32) {
    let enabled = std::env::var("CYNIC_CHAT_TOOLS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if enabled {
        (default_tool_specs(), 4)
    } else {
        (vec![], 1)
    }
}

/// Rebuild chat client after preferred-alias pin (thin wiring; domain is in bind_*).
pub(crate) fn rebind_chat_service(
    store: Arc<dyn ChatSessionStore>,
    endpoint: &str,
    model_id: &str,
    api_key: Option<String>,
    context_turns: Option<usize>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    reasoning_profile: &str,
) -> Result<ChatService, String> {
    let client = HttpOpenAiChatClient::with_api_key(endpoint, model_id, api_key)
        .map_err(|e| e.to_string())?
        .with_runtime_params(max_tokens, temperature, top_p);
    let (specs, max_tools) = chat_tool_policy();
    Ok(ChatService::new(
        store,
        Arc::new(client),
        Arc::new(BuiltinToolInvoker),
        specs,
        cynic_inference::ChatServiceConfig::new(
            Some(endpoint.to_string()),
            Some(model_id.to_string()),
            max_tools,
            true,
        )
        .context_turns(context_turns)
        .max_tokens(max_tokens)
        .temperature(temperature)
        .top_p(top_p)
        .reasoning_profile(reasoning_profile),
    ))
}
