use std::fs;
use std::process::ExitCode;

use clap::Subcommand;
use serde::Serialize;

const TOKEN_PATH: &str = "/etc/cynic/secrets/forgejo-token";
const API_BASE: &str = "http://forgejo.synapse.cynic:3000/api/v1";
const REPO: &str = "jeanterre13/CYNIC";

#[derive(Subcommand)]
pub enum ForgejoCommand {
    /// Create a pull request
    PrCreate {
        #[arg(long)]
        title: String,
        #[arg(long)]
        head: String,
        #[arg(long)]
        base: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long)]
        json: bool,
    },
    /// List pull requests
    PrList {
        #[arg(long, value_enum)]
        state: Option<PrState>,
        #[arg(long)]
        json: bool,
    },
    /// Get a pull request
    PrGet {
        number: u64,
        #[arg(long)]
        json: bool,
    },
    /// Comment on a pull request
    PrComment {
        number: u64,
        #[arg(long)]
        body: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum PrState {
    Open,
    Closed,
    All,
}

#[derive(Serialize)]
struct PrSummary {
    number: u64,
    title: String,
    state: String,
    head: String,
    base: String,
    url: String,
}

#[derive(Serialize)]
struct CommentSummary {
    id: u64,
    url: String,
}

pub(crate) fn execute_forgejo(cmd: ForgejoCommand) -> ExitCode {
    let token = match load_token() {
        Ok(t) => t,
        Err(e) => return crate::fail(e),
    };

    match cmd {
        ForgejoCommand::PrCreate {
            title,
            head,
            base,
            body,
            json,
        } => {
            let req = serde_json::json!({
                "title": title,
                "head": head,
                "base": base,
                "body": body,
            });
            match api_request("POST", &format!("repos/{REPO}/pulls"), Some(req), &token) {
                Ok(resp) => {
                    let summary = PrSummary {
                        number: resp["number"].as_u64().unwrap_or(0),
                        title: resp["title"].as_str().unwrap_or("").to_string(),
                        state: resp["state"].as_str().unwrap_or("").to_string(),
                        head: resp["head"]["ref"].as_str().unwrap_or("").to_string(),
                        base: resp["base"]["ref"].as_str().unwrap_or("").to_string(),
                        url: resp["html_url"].as_str().unwrap_or("").to_string(),
                    };
                    if json {
                        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
                    } else {
                        println!(
                            "#{} [{}] {} ({} -> {})",
                            summary.number,
                            summary.state,
                            summary.title,
                            summary.head,
                            summary.base
                        );
                        println!("{}", summary.url);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => crate::fail(e),
            }
        }
        ForgejoCommand::PrList { state, json } => {
            let state_str = match state {
                Some(PrState::Open) => "open",
                Some(PrState::Closed) => "closed",
                Some(PrState::All) | None => "all",
            };
            match api_request(
                "GET",
                &format!("repos/{REPO}/pulls?state={state_str}"),
                None,
                &token,
            ) {
                Ok(resp) => {
                    let prs: Vec<PrSummary> = resp
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|p| PrSummary {
                            number: p["number"].as_u64().unwrap_or(0),
                            title: p["title"].as_str().unwrap_or("").to_string(),
                            state: p["state"].as_str().unwrap_or("").to_string(),
                            head: p["head"]["ref"].as_str().unwrap_or("").to_string(),
                            base: p["base"]["ref"].as_str().unwrap_or("").to_string(),
                            url: p["html_url"].as_str().unwrap_or("").to_string(),
                        })
                        .collect();
                    if json {
                        println!("{}", serde_json::to_string_pretty(&prs).unwrap());
                    } else {
                        for pr in &prs {
                            println!(
                                "#{} [{}] {} ({} -> {})",
                                pr.number, pr.state, pr.title, pr.head, pr.base
                            );
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => crate::fail(e),
            }
        }
        ForgejoCommand::PrGet { number, json } => {
            match api_request("GET", &format!("repos/{REPO}/pulls/{number}"), None, &token) {
                Ok(resp) => {
                    let summary = PrSummary {
                        number: resp["number"].as_u64().unwrap_or(0),
                        title: resp["title"].as_str().unwrap_or("").to_string(),
                        state: resp["state"].as_str().unwrap_or("").to_string(),
                        head: resp["head"]["ref"].as_str().unwrap_or("").to_string(),
                        base: resp["base"]["ref"].as_str().unwrap_or("").to_string(),
                        url: resp["html_url"].as_str().unwrap_or("").to_string(),
                    };
                    if json {
                        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
                    } else {
                        println!(
                            "#{} [{}] {} ({} -> {})",
                            summary.number,
                            summary.state,
                            summary.title,
                            summary.head,
                            summary.base
                        );
                        println!("{}", summary.url);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => crate::fail(e),
            }
        }
        ForgejoCommand::PrComment { number, body, json } => {
            let req = serde_json::json!({ "body": body });
            match api_request(
                "POST",
                &format!("repos/{REPO}/issues/{number}/comments"),
                Some(req),
                &token,
            ) {
                Ok(resp) => {
                    let summary = CommentSummary {
                        id: resp["id"].as_u64().unwrap_or(0),
                        url: resp["html_url"].as_str().unwrap_or("").to_string(),
                    };
                    if json {
                        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
                    } else {
                        println!("Comment posted: {}", summary.url);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => crate::fail(e),
            }
        }
    }
}

fn load_token() -> Result<String, String> {
    match fs::read_to_string(TOKEN_PATH) {
        Ok(content) => {
            let token = content.trim().to_string();
            if token.is_empty() {
                Err("token vide".to_string())
            } else {
                Ok(token)
            }
        }
        Err(_) => Err(format!("token absent de {TOKEN_PATH}")),
    }
}

fn api_request(
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
    token: &str,
) -> Result<serde_json::Value, String> {
    let url = format!("{API_BASE}/{path}");
    let client = reqwest::blocking::Client::new();
    let mut builder = match method {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        _ => return Err(format!("unsupported method: {method}")),
    };
    builder = builder
        .header("Authorization", format!("token {token}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json");

    let result = if let Some(b) = body {
        builder.json(&b).send()
    } else {
        builder.send()
    };

    match result {
        Ok(resp) => {
            if resp.status().is_success() {
                resp.json().map_err(|e| format!("parse json: {e}"))
            } else {
                let status = resp.status();
                let text = resp.text().unwrap_or_default();
                Err(format!("HTTP {status}: {text}"))
            }
        }
        Err(e) => Err(format!("request failed: {e}")),
    }
}
