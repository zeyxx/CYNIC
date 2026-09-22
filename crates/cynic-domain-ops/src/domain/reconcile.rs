//! Zone reconciliation: diff registrar records against Cloudflare records.
//!
//! Pure, zero-I/O function that produces a `ZoneRecordDiff` (add/remove/update).
//! The runner consumes this diff to issue only the necessary API calls.
//!
//! Catches CNAME coexistence conflicts (the article's "refuses ambiguous
//! multi-record updates unless I ask to replace all") before any provider
//! API call is made.

use std::collections::HashMap;

use cynic_protocol::domain_ops::{DnsRecordShape, DnsRecordType, ProxyPolicy, ZoneRecordDiff};

use crate::domain::transform::{TransformResult, transform_record_for_cloudflare};

/// Record key for diffing: identity is (name, type).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RecordKey(String, DnsRecordType);

/// CNAME coexistence conflict detected during reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictDetection {
    /// The name where the conflict occurs.
    pub name: String,
    /// The conflicting record type already in place.
    pub existing_type: DnsRecordType,
    /// The conflicting record type being added.
    pub incoming_type: DnsRecordType,
}

impl ConflictDetection {
    pub fn message(&self) -> String {
        format!(
            "CNAME coexistence conflict at '{}': cannot have both {:?} and {:?} records",
            self.name, self.existing_type, self.incoming_type
        )
    }
}

/// Reconcile a registrar zone against the existing Cloudflare zone.
///
/// Returns a `ZoneRecordDiff` representing the minimal set of changes needed
/// to bring Cloudflare in line with the desired state. Records that should be
/// skipped (e.g. NS) are excluded from the diff entirely.
///
/// If a CNAME conflict is detected (CNAME at the same name as A/AAAA), the
/// conflict is reported but the diff is still produced — the runner must
/// inspect `conflicts` and refuse unless `replace_all` is requested.
pub fn reconcile_zone(
    registrar_records: &[DnsRecordShape],
    cloudflare_records: &[DnsRecordShape],
    policy: &ProxyPolicy,
) -> ZoneRecordDiff {
    let desired: Vec<DnsRecordShape> = registrar_records
        .iter()
        .filter_map(|r| match transform_record_for_cloudflare(r, policy) {
            TransformResult::Transformed(t) => Some(t),
            TransformResult::Skipped { .. } => None,
        })
        .collect();

    let desired_map: HashMap<RecordKey, &DnsRecordShape> = desired
        .iter()
        .map(|r| (RecordKey(r.name.clone(), r.r#type), r))
        .collect();

    let cf_map: HashMap<RecordKey, &DnsRecordShape> = cloudflare_records
        .iter()
        .map(|r| (RecordKey(r.name.clone(), r.r#type), r))
        .collect();

    let mut add = Vec::new();
    let mut remove = Vec::new();
    let mut update = Vec::new();

    // Add: in desired, not in CF
    for (key, desired_rec) in &desired_map {
        if !cf_map.contains_key(key) {
            add.push((*desired_rec).clone());
        }
    }

    // Remove: in CF, not in desired
    for (key, cf_rec) in &cf_map {
        if !desired_map.contains_key(key) {
            remove.push((*cf_rec).clone());
        }
    }

    // Update: in both, but differ
    for (key, desired_rec) in &desired_map {
        if let Some(cf_rec) = cf_map.get(key) {
            if desired_rec != cf_rec {
                update.push((*desired_rec).clone());
            }
        }
    }

    ZoneRecordDiff {
        add,
        remove,
        update,
    }
}

/// Detect CNAME coexistence conflicts in a record set.
///
/// A CNAME record at name X conflicts with any non-CNAME record at the same name.
/// Returns the first conflict found, if any.
pub fn detect_cname_conflict(records: &[DnsRecordShape]) -> Option<ConflictDetection> {
    // Group records by name
    let mut by_name: HashMap<String, Vec<DnsRecordType>> = HashMap::new();
    for record in records {
        by_name
            .entry(record.name.clone())
            .or_default()
            .push(record.r#type);
    }

    for (name, types) in &by_name {
        let has_cname = types.contains(&DnsRecordType::CNAME);
        if has_cname && types.len() > 1 {
            // Find the conflicting non-CNAME type
            let other = types.iter().find(|t| **t != DnsRecordType::CNAME);
            if let Some(other_type) = other {
                return Some(ConflictDetection {
                    name: name.clone(),
                    existing_type: *other_type,
                    incoming_type: DnsRecordType::CNAME,
                });
            }
        }
    }
    None
}

/// Detect conflicts between a desired record set and existing records.
///
/// Used when the runner checks whether an incoming CNAME conflicts with
/// existing A/AAAA records at the same name.
pub fn detect_conflict_between(
    existing: &[DnsRecordShape],
    incoming: &[DnsRecordShape],
) -> Option<ConflictDetection> {
    for inc in incoming {
        if inc.r#type == DnsRecordType::CNAME {
            for ex in existing {
                if ex.name == inc.name && ex.r#type != DnsRecordType::CNAME {
                    return Some(ConflictDetection {
                        name: inc.name.clone(),
                        existing_type: ex.r#type,
                        incoming_type: DnsRecordType::CNAME,
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::domain_ops::DnsRecordShape;

    fn record(name: &str, r#type: DnsRecordType, content: &str, proxied: bool) -> DnsRecordShape {
        DnsRecordShape {
            name: name.to_string(),
            r#type,
            content: content.to_string(),
            ttl: 300,
            priority: None,
            proxied,
        }
    }

    #[test]
    fn empty_zones_produce_empty_diff() {
        let diff = reconcile_zone(&[], &[], &ProxyPolicy::default());
        assert!(diff.is_empty());
    }

    #[test]
    fn all_new_records_go_to_add() {
        let registrar = vec![
            record("www.example.com", DnsRecordType::A, "1.2.3.4", true),
            record(
                "mail.example.com",
                DnsRecordType::MX,
                "mail.example.com",
                false,
            ),
        ];
        let diff = reconcile_zone(&registrar, &[], &ProxyPolicy::default());
        assert_eq!(diff.add.len(), 2);
        assert_eq!(diff.remove.len(), 0);
        assert_eq!(diff.update.len(), 0);
    }

    #[test]
    fn all_existing_records_go_to_remove() {
        let cf = vec![record("old.example.com", DnsRecordType::A, "5.6.7.8", true)];
        let diff = reconcile_zone(&[], &cf, &ProxyPolicy::default());
        assert_eq!(diff.add.len(), 0);
        assert_eq!(diff.remove.len(), 1);
        assert_eq!(diff.update.len(), 0);
    }

    #[test]
    fn unchanged_records_produce_no_diff() {
        let r = record("www.example.com", DnsRecordType::A, "1.2.3.4", true);
        let diff = reconcile_zone(&[r.clone()], &[r.clone()], &ProxyPolicy::default());
        assert!(diff.is_empty());
    }

    #[test]
    fn changed_content_goes_to_update() {
        let registrar = vec![record("www.example.com", DnsRecordType::A, "1.2.3.4", true)];
        let cloudflare = vec![record("www.example.com", DnsRecordType::A, "5.6.7.8", true)];
        let diff = reconcile_zone(&registrar, &cloudflare, &ProxyPolicy::default());
        assert_eq!(diff.add.len(), 0);
        assert_eq!(diff.remove.len(), 0);
        assert_eq!(diff.update.len(), 1);
    }

    #[test]
    fn changed_proxy_flag_goes_to_update() {
        let registrar = vec![record("www.example.com", DnsRecordType::A, "1.2.3.4", true)];
        let cloudflare = vec![record(
            "www.example.com",
            DnsRecordType::A,
            "1.2.3.4",
            false,
        )];
        let diff = reconcile_zone(&registrar, &cloudflare, &ProxyPolicy::default());
        assert_eq!(diff.update.len(), 1);
    }

    #[test]
    fn ns_records_are_skipped_from_diff() {
        let registrar = vec![
            record("www.example.com", DnsRecordType::A, "1.2.3.4", true),
            record(
                "ns1.example.com",
                DnsRecordType::NS,
                "ns1.registrar.com",
                false,
            ),
            record(
                "ns2.example.com",
                DnsRecordType::NS,
                "ns2.registrar.com",
                false,
            ),
        ];
        let diff = reconcile_zone(&registrar, &[], &ProxyPolicy::default());
        assert_eq!(diff.add.len(), 1);
        assert_eq!(diff.add[0].name, "www.example.com");
    }

    #[test]
    fn mx_records_are_dns_only_after_transform() {
        let registrar = vec![record(
            "mail.example.com",
            DnsRecordType::MX,
            "mail.example.com",
            false,
        )];
        let diff = reconcile_zone(&registrar, &[], &ProxyPolicy::default());
        assert_eq!(diff.add.len(), 1);
        assert!(!diff.add[0].proxied);
    }

    #[test]
    fn mixed_zone_produces_correct_diff() {
        let registrar = vec![
            record("www.example.com", DnsRecordType::A, "1.2.3.4", true),
            record(
                "mail.example.com",
                DnsRecordType::MX,
                "mail.example.com",
                false,
            ),
        ];
        let cloudflare = vec![
            record("www.example.com", DnsRecordType::A, "99.99.99.99", true), // stale
            record("old.example.com", DnsRecordType::A, "5.6.7.8", true),     // to remove
        ];
        let diff = reconcile_zone(&registrar, &cloudflare, &ProxyPolicy::default());
        assert_eq!(diff.add.len(), 1); // mail.example.com MX
        assert_eq!(diff.remove.len(), 1); // old.example.com A
        assert_eq!(diff.update.len(), 1); // www.example.com A content changed
    }

    #[test]
    fn detect_cname_conflict_at_same_name() {
        let records = vec![
            record(
                "www.example.com",
                DnsRecordType::CNAME,
                "target.pages.dev",
                true,
            ),
            record("www.example.com", DnsRecordType::A, "1.2.3.4", true),
        ];
        let conflict = detect_cname_conflict(&records);
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.name, "www.example.com");
    }

    #[test]
    fn no_conflict_without_cname() {
        let records = vec![
            record("www.example.com", DnsRecordType::A, "1.2.3.4", true),
            record("www.example.com", DnsRecordType::AAAA, "::1", true),
        ];
        assert!(detect_cname_conflict(&records).is_none());
    }

    #[test]
    fn no_conflict_with_only_cname() {
        let records = vec![record(
            "www.example.com",
            DnsRecordType::CNAME,
            "target.pages.dev",
            true,
        )];
        assert!(detect_cname_conflict(&records).is_none());
    }

    #[test]
    fn detect_conflict_between_existing_and_incoming() {
        let existing = vec![record("www.example.com", DnsRecordType::A, "1.2.3.4", true)];
        let incoming = vec![record(
            "www.example.com",
            DnsRecordType::CNAME,
            "target.pages.dev",
            true,
        )];
        let conflict = detect_conflict_between(&existing, &incoming);
        assert!(conflict.is_some());
    }

    #[test]
    fn no_false_conflict_between_different_names() {
        let existing = vec![record("a.example.com", DnsRecordType::A, "1.2.3.4", true)];
        let incoming = vec![record(
            "b.example.com",
            DnsRecordType::CNAME,
            "target.pages.dev",
            true,
        )];
        assert!(detect_conflict_between(&existing, &incoming).is_none());
    }
}
