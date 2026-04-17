//! Google Calendar REST adapter with OAuth2 (NOT MCP, per spec §3.3).

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

use crate::anchor::{AnchorId, AnchorProvider};
use crate::error::AskesisError;

const CALENDAR_API_BASE: &str = "https://www.googleapis.com/calendar/v3";
const SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";

pub struct GoogleCalendarAnchor {
    http: reqwest::Client,
    calendar_id: String,
    client_secret_path: PathBuf,
    creds_cache_path: PathBuf,
}

impl std::fmt::Debug for GoogleCalendarAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleCalendarAnchor")
            .field("calendar_id", &self.calendar_id)
            .field("client_secret_path", &self.client_secret_path)
            .field("creds_cache_path", &self.creds_cache_path)
            .finish_non_exhaustive()
    }
}

/// Path discovery helper. Respects $HOME.
pub fn default_creds_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".cynic/askesis/gcal-creds.json")
}

impl GoogleCalendarAnchor {
    pub async fn setup(
        client_secret_path: PathBuf,
        creds_cache_path: PathBuf,
        calendar_id: impl Into<String>,
    ) -> crate::Result<Self> {
        if let Some(parent) = creds_cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Validate the secret can be parsed (fail early).
        let _secret = yup_oauth2::read_application_secret(&client_secret_path)
            .await
            .map_err(|e| AskesisError::OAuth(format!("read secret: {e}")))?;

        Ok(Self {
            http: reqwest::Client::builder()
                .build()
                .map_err(|e| AskesisError::GoogleCalendar(format!("http client: {e}")))?,
            calendar_id: calendar_id.into(),
            client_secret_path,
            creds_cache_path,
        })
    }

    async fn get_token(&self) -> crate::Result<String> {
        let secret = yup_oauth2::read_application_secret(&self.client_secret_path)
            .await
            .map_err(|e| AskesisError::OAuth(format!("read secret: {e}")))?;

        let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
            secret,
            yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk(&self.creds_cache_path)
        .build()
        .await
        .map_err(|e| AskesisError::OAuth(format!("build authenticator: {e}")))?;

        let tok = auth
            .token(&[SCOPE])
            .await
            .map_err(|e| AskesisError::OAuth(format!("fetch token: {e}")))?;

        tok.token()
            .map(|s| s.to_string())
            .ok_or_else(|| AskesisError::OAuth("token empty".into()))
    }
}

#[derive(Serialize)]
struct EventDateTime<'a> {
    #[serde(rename = "dateTime")]
    date_time: String,
    #[serde(rename = "timeZone")]
    time_zone: &'a str,
}

#[derive(Serialize)]
struct CreateEventBody<'a> {
    summary: String,
    description: &'a str,
    start: EventDateTime<'a>,
    end: EventDateTime<'a>,
    recurrence: Vec<&'a str>,
}

#[derive(Serialize)]
struct PatchEventBody<'a> {
    description: &'a str,
}

#[derive(Deserialize)]
struct EventResponse {
    id: String,
}

#[async_trait]
impl AnchorProvider for GoogleCalendarAnchor {
    async fn create_recurring(
        &self,
        domain: &str,
        at: NaiveTime,
        description: &str,
    ) -> crate::Result<AnchorId> {
        let tz = "Europe/Paris";
        let today = chrono::Local::now().date_naive();
        let start_naive = today.and_time(at);
        let end_naive = start_naive + chrono::Duration::minutes(40);

        let start_dt = start_naive.format("%Y-%m-%dT%H:%M:%S").to_string();
        let end_dt = end_naive.format("%Y-%m-%dT%H:%M:%S").to_string();

        let body = CreateEventBody {
            summary: format!("\u{1f4aa} cynic-askesis: {domain}"),
            description,
            start: EventDateTime {
                date_time: start_dt,
                time_zone: tz,
            },
            end: EventDateTime {
                date_time: end_dt,
                time_zone: tz,
            },
            recurrence: vec!["RRULE:FREQ=DAILY"],
        };

        let url = format!(
            "{CALENDAR_API_BASE}/calendars/{cid}/events",
            cid = urlencoding::encode(&self.calendar_id)
        );
        let token = self.get_token().await?;
        let resp = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AskesisError::GoogleCalendar(format!("POST events: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AskesisError::GoogleCalendar(format!(
                "status {status}: {text}"
            )));
        }

        let parsed: EventResponse = resp
            .json()
            .await
            .map_err(|e| AskesisError::GoogleCalendar(format!("parse response: {e}")))?;
        Ok(AnchorId::new(parsed.id))
    }

    async fn update_description(&self, id: AnchorId, new: &str) -> crate::Result<()> {
        let url = format!(
            "{CALENDAR_API_BASE}/calendars/{cid}/events/{eid}",
            cid = urlencoding::encode(&self.calendar_id),
            eid = urlencoding::encode(id.as_str()),
        );
        let token = self.get_token().await?;
        let body = PatchEventBody { description: new };
        let resp = self
            .http
            .patch(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AskesisError::GoogleCalendar(format!("PATCH event: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AskesisError::GoogleCalendar(format!(
                "status {status}: {text}"
            )));
        }
        Ok(())
    }
}
