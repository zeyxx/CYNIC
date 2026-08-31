use std::fs;

use crate::chat_surface::ChatRuntime;
use cynic_inference::{ChatEvent, ChatEventKind};

use super::types::{ChatLine, Focus, TuiTaskEvent, TuiWindow};

fn apply_reasoning_profile(
    name: &str,
    current_context: Option<usize>,
    current_max_tokens: Option<u32>,
    current_temperature: Option<f32>,
    current_top_p: Option<f32>,
) -> Result<(Option<usize>, Option<u32>, Option<f32>, Option<f32>), String> {
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() {
        return Ok((
            current_context,
            current_max_tokens,
            current_temperature,
            current_top_p,
        ));
    }

    match name.as_str() {
        "balanced" => Ok((None, None, None, None)),
        "fast" => Ok((Some(16), Some(768), Some(0.35), Some(0.8))),
        "creative" => Ok((Some(96), Some(1536), Some(1.0), Some(0.95))),
        "deep" | "reasoning" => Ok((Some(256), Some(3072), Some(0.65), Some(0.9))),
        _ => Err(format!(
            "unknown profile `{name}` (fast | balanced | creative | deep | reasoning)"
        )),
    }
}

pub(super) fn events_to_lines(events: &[ChatEvent]) -> Vec<ChatLine> {
    let mut lines = Vec::new();
    for e in events {
        match e.kind {
            ChatEventKind::User => lines.push(ChatLine {
                role: "you".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::Assistant => lines.push(ChatLine {
                role: "cynic".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::ToolCall => lines.push(ChatLine {
                role: "tool→".into(),
                text: format!(
                    "{} {}",
                    e.tool_name.as_deref().unwrap_or("?"),
                    e.tool_args_json.as_deref().unwrap_or("")
                ),
            }),
            ChatEventKind::ToolResult => lines.push(ChatLine {
                role: "tool←".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::System => lines.push(ChatLine {
                role: "system".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::Error => lines.push(ChatLine {
                role: "error".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::StreamMeta => {}
        }
    }
    lines
}

pub(super) fn format_status(
    runtime: &ChatRuntime,
    paths: &PrincipalPaths,
    session_id: &str,
    busy: bool,
) -> String {
    let mode = if runtime.use_mock {
        "mock"
    } else {
        runtime.source.as_str()
    };
    let ep = runtime.endpoint.as_deref().unwrap_or("-");
    let model = runtime.model.as_deref().unwrap_or("-");
    let busy_s = if busy { " · thinking…" } else { "" };
    let context = runtime
        .context_turns
        .map(|v| format!("cxt={v}"))
        .unwrap_or_else(|| "cxt=full".into());
    let temperature = runtime
        .temperature
        .map(|v| format!("temp={v:.2}"))
        .unwrap_or_else(|| "temp=auto".into());
    let max_tokens = runtime
        .max_tokens
        .map(|v| format!("max={v}"))
        .unwrap_or_else(|| "max=auto".into());
    let top_p = runtime
        .top_p
        .map(|v| format!("top_p={v:.2}"))
        .unwrap_or_else(|| "top_p=auto".into());
    let detail = if runtime.detail.is_empty() {
        String::new()
    } else {
        format!(" · {}", runtime.detail)
    };
    format!(
        "{mode} · {ep} · {model} · @{user} · {session} · {context} · {temperature} · {max_tokens} · {top_p}{detail}{busy_s}",
        user = paths.runtime_user,
        session = short_id(session_id),
    )
}

fn short_id(id: &str) -> &str {
    if id.len() > 18 {
        &id[id.len() - 18..]
    } else {
        id
    }
}

fn parse_tail_lines_from_command(raw: &str, default: usize) -> Result<usize, String> {
    if raw.is_empty() {
        return Ok(chat_tui_log_tail_lines(default));
    }

    match raw.parse::<usize>() {
        Ok(value) => Ok(value),
        Err(_) => Err(
            "/tail expects optional positive integer (default from CYNIC_CHAT_LOG_TAIL_LINES)"
                .into(),
        ),
    }
}

fn chat_tui_log_tail_lines(default: usize) -> usize {
    std::env::var(CHAT_TUI_LOG_TAIL_LINES_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn extract_log_path_from_message(message: &str) -> Option<String> {
    message
        .split(';')
        .find_map(|part| {
            part.trim()
                .strip_prefix("log=")
                .map(|value| value.to_string())
        })
        .or_else(|| {
            message
                .split_once("log=")
                .map(|(_, value)| value.split_whitespace().next().unwrap_or("").to_string())
        })
        .filter(|s| !s.is_empty())
}

fn read_log_tail(log_path: &str, tail: usize) -> Vec<String> {
    if tail == 0 || log_path.is_empty() {
        return Vec::new();
    }
    let content = match fs::read_to_string(log_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    let lines: Vec<&str> = content.lines().filter(|line| !line.is_empty()).collect();
    let start = lines.len().saturating_sub(tail);
    lines[start..].iter().map(|line| line.to_string()).collect()
}

fn local_runtime_status_tail(runtime: &ChatRuntime, tail: usize) -> Option<Vec<String>> {
    let service = cynic_inference::LocalRuntimeServiceAny::from_env();
    let endpoint = runtime.endpoint.as_deref()?;
    let status = service.status().ok()?;
    let status_matches = status.endpoint.as_deref() == Some(endpoint)
        || status
            .alias
            .as_deref()
            .is_some_and(|alias| Some(alias) == runtime.model.as_deref());
    if !status_matches {
        return None;
    }
    let log_path = status
        .message
        .as_deref()
        .and_then(extract_log_path_from_message)?;
    let lines = read_log_tail(&log_path, tail);
    if lines.is_empty() { None } else { Some(lines) }
}

#[cfg(test)]
mod tests {
    use super::apply_reasoning_profile;

    #[test]
    fn applies_balanced_preset() {
        let got =
            apply_reasoning_profile("balanced", Some(8), Some(64), Some(0.5), Some(0.7)).unwrap();
        assert_eq!(got, (None, None, None, None));
    }

    #[test]
    fn applies_fast_preset() {
        let got = apply_reasoning_profile("fast", None, None, None, None).unwrap();
        assert_eq!(got, (Some(16), Some(768), Some(0.35), Some(0.8)));
    }

    #[test]
    fn rejects_unknown_preset() {
        assert!(apply_reasoning_profile("quantum", None, None, None, None).is_err());
    }
}

fn detect_local_runtime_endpoint(alias: &str) -> Option<String> {
    let service = cynic_inference::LocalRuntimeServiceAny::from_env();
    let expected = alias.to_ascii_lowercase();
    for _ in 0..12 {
        if let Ok(status) = service.status() {
            if let Some(endpoint) = local_runtime_endpoint_from_status(&status, &expected) {
                return Some(endpoint);
            }
        }
        std::thread::sleep(Duration::from_millis(250));
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

fn local_runtime_endpoint_from_status(
    status: &LocalRuntimeStatus,
    expected_alias: &str,
) -> Option<String> {
    if let Some(current_alias) = status.alias.as_deref()
        && !current_alias.eq_ignore_ascii_case(expected_alias)
    {
        return None;
    }

    status.endpoint.clone().or_else(|| {
        status
            .listen_host
            .as_ref()
            .zip(status.listen_port)
            .map(|(host, port)| format!("http://{host}:{port}/v1"))
    })
}

fn looks_like_local_selector(selector: &str) -> bool {
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
}

pub(super) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
