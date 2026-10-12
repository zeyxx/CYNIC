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
