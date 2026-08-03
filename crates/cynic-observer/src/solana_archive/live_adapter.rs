use super::{
    BoxSourceFuture, Commitment, CreditConfidence, CreditMeasurement, HeliusCreditCatalog,
    HeliusPlan, IndexedHistoryPage, IndexedHistoryQuery, IndexedHistorySource,
    IndexedTransactionRecord, ProviderKind, RawTransactionObservation, RequestMetrics,
    SignaturePage, SignatureQuery, SignatureRecord, SignatureSource, SortOrder, SourceError,
    SourceRole, TransactionQuery, TransactionSource,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct JsonRpcAdapter {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
    provider: ProviderKind,
    helius_plan: Option<HeliusPlan>,
    credit_catalog: HeliusCreditCatalog,
}

impl JsonRpcAdapter {
    pub fn helius(endpoint: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::helius_with_plan(endpoint, api_key, HeliusPlan::Unknown)
    }

    pub fn helius_with_plan(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        plan: HeliusPlan,
    ) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
            api_key: Some(api_key.into()),
            provider: ProviderKind::Helius,
            helius_plan: Some(plan),
            credit_catalog: HeliusCreditCatalog::documentation_snapshot_2026_08_03(),
        }
    }

    pub fn public_rpc(endpoint: impl Into<String>) -> Self {
        Self::with_provider_endpoint(endpoint, ProviderKind::PublicRpc)
    }

    /// Builds a raw JSON-RPC adapter for a provider whose authenticated
    /// endpoint is already fully resolved by the secret/configuration layer.
    /// The adapter never guesses whether a provider uses a query key, path
    /// token or header token.
    pub fn with_provider_endpoint(endpoint: impl Into<String>, provider: ProviderKind) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
            api_key: None,
            provider,
            helius_plan: None,
            credit_catalog: HeliusCreditCatalog::documentation_snapshot_2026_08_03(),
        }
    }

    fn request_url(&self) -> String {
        match &self.api_key {
            Some(api_key) if !api_key.is_empty() => {
                let separator = if self.endpoint.contains('?') {
                    '&'
                } else {
                    '?'
                };
                format!("{}{}api-key={}", self.endpoint, separator, api_key)
            }
            _ => self.endpoint.clone(),
        }
    }
}

impl SignatureSource for JsonRpcAdapter {
    fn fetch_signatures(&self, query: SignatureQuery) -> BoxSourceFuture<SignaturePage> {
        let adapter = self.clone();
        Box::pin(async move {
            let request_query = query.clone();
            let params = json!([request_query.target, {
                "before": request_query.before,
                "until": request_query.until,
                "limit": request_query.limit,
                "commitment": commitment_name(&request_query.commitment)
            }]);
            let mut receipt = rpc_request(&adapter, "getSignaturesForAddress", params).await?;
            let payload = &receipt.payload;
            let records = payload
                .as_array()
                .ok_or_else(|| {
                    SourceError::InvalidResponse("signature result is not an array".to_string())
                })?
                .iter()
                .map(parse_signature_record)
                .collect::<Result<Vec<_>, _>>()?;
            let next_before = records.last().map(|record| record.signature.clone());
            receipt.metrics.records_returned = records.len() as u64;
            Ok(SignaturePage {
                provider: adapter.provider,
                role: SourceRole::RawIndependent,
                query,
                records,
                next_before,
                response_digest: receipt.response_digest,
                metrics: receipt.metrics,
            })
        })
    }
}

impl TransactionSource for JsonRpcAdapter {
    fn fetch_transaction(
        &self,
        query: TransactionQuery,
    ) -> BoxSourceFuture<RawTransactionObservation> {
        let adapter = self.clone();
        Box::pin(async move {
            let request_query = query.clone();
            let params = json!([request_query.signature, {
                "encoding": "jsonParsed",
                "commitment": commitment_name(&request_query.commitment),
                "maxSupportedTransactionVersion": 0
            }]);
            let mut receipt = rpc_request(&adapter, "getTransaction", params).await?;
            receipt.metrics.records_returned = u64::from(!receipt.payload.is_null());
            Ok(RawTransactionObservation {
                provider: adapter.provider,
                role: SourceRole::RawIndependent,
                query,
                payload: receipt.payload,
                response_digest: receipt.response_digest,
                retrieved_at_unix_ms: now_unix_ms(),
                metrics: receipt.metrics,
            })
        })
    }
}

impl IndexedHistorySource for JsonRpcAdapter {
    fn fetch_history(&self, query: IndexedHistoryQuery) -> BoxSourceFuture<IndexedHistoryPage> {
        let adapter = self.clone();
        Box::pin(async move {
            if adapter.provider != ProviderKind::Helius {
                return Err(SourceError::Blocked(
                    "indexed history requires a Helius adapter".to_string(),
                ));
            }
            let plan = adapter.helius_plan.as_ref().unwrap_or(&HeliusPlan::Unknown);
            if !HeliusCreditCatalog::supports_indexed_history(plan) {
                return Err(SourceError::Blocked(
                    "getTransactionsForAddress requires a Helius Developer plan or above"
                        .to_string(),
                ));
            }
            if matches!(
                query.commitment,
                Commitment::Processed | Commitment::Unknown
            ) {
                return Err(SourceError::Blocked(
                    "indexed history requires confirmed or finalized commitment".to_string(),
                ));
            }
            if query.limit == 0 || query.limit > 100 {
                return Err(SourceError::Blocked(
                    "full indexed history page limit must be between 1 and 100".to_string(),
                ));
            }
            let request_query = query.clone();
            let mut filters = serde_json::Map::new();
            if request_query.filters.start_slot.is_some()
                || request_query.filters.end_slot.is_some()
            {
                let mut slot = serde_json::Map::new();
                if let Some(start) = request_query.filters.start_slot {
                    slot.insert("gte".to_string(), json!(start));
                }
                if let Some(end) = request_query.filters.end_slot {
                    slot.insert("lte".to_string(), json!(end));
                }
                filters.insert("slot".to_string(), Value::Object(slot));
            }
            if let Some(status) = &request_query.filters.status {
                filters.insert("status".to_string(), json!(status));
            }
            if let Some(token_accounts) = &request_query.filters.token_accounts {
                filters.insert("tokenAccounts".to_string(), json!(token_accounts));
            }
            let params = json!([request_query.target, {
                "transactionDetails": "full",
                "encoding": "jsonParsed",
                "maxSupportedTransactionVersion": 0,
                "sortOrder": match request_query.sort_order {
                    SortOrder::Ascending => "asc",
                    SortOrder::Descending => "desc"
                },
                "limit": request_query.limit,
                "paginationToken": request_query.pagination_token,
                "commitment": commitment_name(&request_query.commitment),
                "filters": Value::Object(filters)
            }]);
            let mut receipt = rpc_request(&adapter, "getTransactionsForAddress", params).await?;
            let data = receipt
                .payload
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    SourceError::InvalidResponse("indexed history data missing".to_string())
                })?;
            let records = data
                .iter()
                .map(parse_indexed_transaction)
                .collect::<Result<Vec<_>, _>>()?;
            receipt.metrics.records_returned = records.len() as u64;
            let next_pagination_token = receipt
                .payload
                .get("paginationToken")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            Ok(IndexedHistoryPage {
                provider: adapter.provider,
                role: SourceRole::IndexedPrimary,
                query,
                records,
                next_pagination_token,
                response_digest: receipt.response_digest,
                metrics: receipt.metrics,
            })
        })
    }
}

struct RpcReceipt {
    payload: Value,
    response_digest: String,
    metrics: RequestMetrics,
}

async fn rpc_request(
    adapter: &JsonRpcAdapter,
    method: &str,
    params: Value,
) -> Result<RpcReceipt, SourceError> {
    let started_at_unix_ms = now_unix_ms();
    let started = std::time::Instant::now();
    let body = json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params});
    let request_bytes = serde_json::to_vec(&body)
        .map_err(|error| SourceError::InvalidResponse(error.to_string()))?;
    let response = adapter
        .client
        .post(adapter.request_url())
        .header("content-type", "application/json")
        .body(request_bytes.clone())
        .send()
        .await
        .map_err(|error| SourceError::Transport(error.to_string()))?;
    let status = response.status();
    let retry_after_ms = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds * 1_000);
    let bytes = response
        .bytes()
        .await
        .map_err(|error| SourceError::Transport(error.to_string()))?;
    let response_digest = format!("blake3:{}", blake3::hash(&bytes).to_hex());
    if status.as_u16() == 429 {
        return Err(SourceError::RateLimited { retry_after_ms });
    }
    if !status.is_success() {
        return Err(SourceError::Http {
            status: status.as_u16(),
            detail: String::from_utf8_lossy(&bytes).to_string(),
        });
    }
    let envelope: Value = serde_json::from_slice(&bytes)
        .map_err(|error| SourceError::InvalidResponse(error.to_string()))?;
    if let Some(error) = envelope.get("error") {
        return Err(SourceError::Rpc {
            code: error.get("code").and_then(Value::as_i64),
            detail: error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("RPC error")
                .to_string(),
        });
    }
    let result = envelope
        .get("result")
        .cloned()
        .ok_or_else(|| SourceError::InvalidResponse("RPC result missing".to_string()))?;
    let credits = if adapter.provider == ProviderKind::Helius {
        adapter.credit_catalog.estimate_rpc_method(method)
    } else {
        CreditMeasurement {
            credits: None,
            confidence: CreditConfidence::Unknown,
            catalog_version: None,
        }
    };
    Ok(RpcReceipt {
        payload: result,
        response_digest,
        metrics: RequestMetrics {
            method: method.to_string(),
            provider: adapter.provider.clone(),
            started_at_unix_ms,
            latency_ms: started.elapsed().as_millis() as u64,
            request_bytes: request_bytes.len() as u64,
            response_bytes: bytes.len() as u64,
            http_status: status.as_u16(),
            records_returned: 0,
            credits,
        },
    })
}

fn parse_signature_record(value: &Value) -> Result<SignatureRecord, SourceError> {
    let signature = value
        .get("signature")
        .and_then(Value::as_str)
        .ok_or_else(|| SourceError::InvalidResponse("signature missing".to_string()))?;
    Ok(SignatureRecord {
        signature: signature.to_string(),
        slot: value.get("slot").and_then(Value::as_u64),
        block_time: value.get("blockTime").and_then(Value::as_i64),
        confirmation_status: value
            .get("confirmationStatus")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        err: value.get("err").cloned(),
    })
}

fn parse_indexed_transaction(value: &Value) -> Result<IndexedTransactionRecord, SourceError> {
    let signature = value
        .get("signature")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("transaction")
                .and_then(|transaction| transaction.get("signatures"))
                .and_then(Value::as_array)
                .and_then(|signatures| signatures.first())
                .and_then(Value::as_str)
        })
        .ok_or_else(|| {
            SourceError::InvalidResponse("indexed transaction signature missing".to_string())
        })?;
    Ok(IndexedTransactionRecord {
        signature: signature.to_string(),
        slot: value.get("slot").and_then(Value::as_u64),
        block_time: value.get("blockTime").and_then(Value::as_i64),
        transaction_index: value.get("transactionIndex").and_then(Value::as_u64),
        payload: value.clone(),
    })
}

fn commitment_name(commitment: &Commitment) -> &'static str {
    match commitment {
        Commitment::Processed => "processed",
        Commitment::Confirmed => "confirmed",
        Commitment::Finalized => "finalized",
        Commitment::Unknown => "finalized",
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
