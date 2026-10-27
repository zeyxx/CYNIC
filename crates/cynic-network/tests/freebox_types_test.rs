use cynic_network::{
    FreeboxDmzConfig, FreeboxNatRule, FreeboxNatRuleSpec, FreeboxPort, FreeboxWanStatus,
    FREEBOX_DMZ_CONFIG_SCHEMA, FREEBOX_NAT_RULE_SCHEMA, FREEBOX_WAN_STATUS_SCHEMA,
};

#[test]
fn test_freebox_schemas_and_types() {
    let wan = FreeboxWanStatus {
        schema: FREEBOX_WAN_STATUS_SCHEMA.to_string(),
        state: "up".to_string(),
        ipv4: Some("82.65.26.228".to_string()),
        ipv6: Some("2a01:e0a:xxx::1".to_string()),
        rate_down_bps: Some(1000000),
        rate_up_bps: Some(500000),
        bytes_down_total: Some(123456789),
        bytes_up_total: Some(987654321),
    };
    assert_eq!(wan.schema, "cynic.network.freebox_wan_status.v1");
    assert_eq!(wan.state, "up");

    let dmz = FreeboxDmzConfig {
        schema: FREEBOX_DMZ_CONFIG_SCHEMA.to_string(),
        enabled: true,
        ip: "10.10.10.127".to_string(),
    };
    assert_eq!(dmz.schema, "cynic.network.freebox_dmz_config.v1");
    assert!(dmz.enabled);

    let rule = FreeboxNatRule {
        schema: FREEBOX_NAT_RULE_SCHEMA.to_string(),
        id: 42,
        enabled: true,
        ip: "10.10.10.127".to_string(),
        protocol: "tcp".to_string(),
        wan_port_start: 443,
        wan_port_end: 443,
        lan_port: 443,
        comment: "cynic-talaria-relay".to_string(),
    };
    assert_eq!(rule.schema, "cynic.network.freebox_nat_rule.v1");
    assert_eq!(rule.id, 42);

    let spec = FreeboxNatRuleSpec {
        enabled: true,
        ip: "10.10.10.127".to_string(),
        protocol: "tcp".to_string(),
        wan_port_start: 80,
        wan_port_end: 80,
        lan_port: 80,
        comment: "http-caddy".to_string(),
    };
    assert_eq!(spec.lan_port, 80);
}
