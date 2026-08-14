use std::path::Path;

use cynic_host::network_actuation::{NetworkActuationRequest, NetworkActuationWireRequest};

pub(crate) fn read_network_actuation_request(
    path: &Path,
) -> Result<NetworkActuationRequest, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "failed to read network actuation request {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice::<NetworkActuationWireRequest>(&bytes)
        .map_err(|error| {
            format!(
                "invalid network actuation request JSON {}: {error}",
                path.display()
            )
        })?
        .into_request()
}

#[cfg(test)]
mod tests {
    use super::read_network_actuation_request;

    #[test]
    fn rejects_unknown_request_schema() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        std::fs::write(
            file.path(),
            serde_json::to_vec(&serde_json::json!({
                "schema": "unknown",
                "plan": {
                    "schema": "cynic.host.network_actuation_plan.v1",
                    "id": "plan",
                    "profile": "synapse-route",
                    "target": "guest",
                    "commands": [],
                    "preconditions": [],
                    "rollback_intent": "restore",
                    "expected_proofs": []
                },
                "admission": {},
                "authorization": {}
            }))
            .expect("json"),
        )
        .expect("write");

        let error = read_network_actuation_request(file.path()).expect_err("invalid request");

        assert!(error.contains("invalid network actuation request JSON"));
    }
}
