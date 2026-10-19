//! Verification of RFC 9000 QUIC pure domain predicates and invariants.

use cynic_network::quic::{
    CYNIC_ALPN_V1, QuicTransportParameters, StreamChannel, StreamDirection,
    decide_transport_admission,
};

#[test]
fn quic_alpn_matches_cynic_agent_v1() {
    assert_eq!(CYNIC_ALPN_V1, b"cynic-agent/1");
}

#[test]
fn control_channel_mandates_bidirectional_stream() {
    assert_eq!(
        StreamChannel::Control.mandated_direction(),
        StreamDirection::Bidirectional
    );
    assert!(!StreamChannel::Control.head_of_line_blocking());
}

#[test]
fn telemetry_and_artifact_channels_mandate_unidirectional_streams() {
    assert_eq!(
        StreamChannel::Telemetry.mandated_direction(),
        StreamDirection::Unidirectional
    );
    assert_eq!(
        StreamChannel::Artifact.mandated_direction(),
        StreamDirection::Unidirectional
    );
    assert!(!StreamChannel::Telemetry.head_of_line_blocking());
    assert!(!StreamChannel::Artifact.head_of_line_blocking());
}

#[test]
fn transport_admission_rejects_zero_bidi_streams() {
    let mut params = QuicTransportParameters::default();
    params.initial_max_streams_bidi = 0;
    let verdict = decide_transport_admission(&params);
    assert!(!verdict.accepted);
    assert!(verdict.reason.contains("bidirectional"));
}

#[test]
fn transport_admission_accepts_valid_default_profile() {
    let params = QuicTransportParameters::default();
    let verdict = decide_transport_admission(&params);
    assert!(verdict.accepted);
}

#[test]
fn quic_framing_control_roundtrip() {
    let msg = cynic_network::quic::ControlMessage::TaskDispatch {
        task_id: "task-42".to_string(),
        assignee: "agent-builder".to_string(),
        lease_ms: 120_000,
        payload_json: "{\"action\":\"build\"}".to_string(),
    };
    let frame = cynic_network::quic::AgentFrame::Control(msg.clone());
    let encoded = cynic_network::quic::encode_frame(&frame).expect("encode frame");
    let decoded = cynic_network::quic::decode_frame(&encoded).expect("decode frame");
    assert_eq!(decoded, cynic_network::quic::AgentFrame::Control(msg));
}

#[test]
fn quic_framing_telemetry_roundtrip() {
    let msg = cynic_network::quic::TelemetryMessage {
        timestamp_ms: 1_791_772_000,
        cpu_util_pct: 12.5,
        mem_util_pct: 45.0,
        open_streams_count: 8,
    };
    let frame = cynic_network::quic::AgentFrame::Telemetry(msg.clone());
    let encoded = cynic_network::quic::encode_frame(&frame).expect("encode frame");
    let decoded = cynic_network::quic::decode_frame(&encoded).expect("decode frame");
    assert_eq!(decoded, cynic_network::quic::AgentFrame::Telemetry(msg));
}

#[test]
fn quic_framing_artifact_chunk_roundtrip_and_hash_integrity() {
    let raw_payload = bytes::Bytes::from_static(b"CYNIC RELEASE BINARY CHUNK CONTENT TEST BLAKE3");
    let chunk = cynic_network::quic::ArtifactChunkMessage::new(0, 1, raw_payload);
    let frame = cynic_network::quic::AgentFrame::Artifact(chunk);
    let encoded = cynic_network::quic::encode_frame(&frame).expect("encode frame");
    let decoded = cynic_network::quic::decode_frame(&encoded).expect("decode frame");
    match decoded {
        cynic_network::quic::AgentFrame::Artifact(c) => {
            assert_eq!(c.chunk_index, 0);
            assert_eq!(c.total_chunks, 1);
            assert!(c.verify_integrity().is_ok());
        }
        _ => panic!("expected artifact frame"),
    }
}
