//! Pure parsers for `/etc/passwd` and `/etc/group` database contents.
//!
//! Format (both files share the layout for our purposes):
//! ```text
//! name:password:ID:rest...
//! ```
//! - `/etc/passwd`: `name:password:uid:gid:gecos:home:shell` — the numeric
//!   identity is field index 2 (uid).
//! - `/etc/group`: `name:password:gid:members` — the numeric identity is
//!   field index 2 (gid), members are field index 3.
//!
//! This module only parses in-memory text: it performs no I/O and holds no
//! policy. Callers that need a live lookup read the file themselves and pass
//! its contents, so every crate shares the single parsing implementation.
//!
//! Lookup requires an exact name match: `"oper"` does not resolve
//! `"operator"`. Malformed lines (too few fields, non-numeric ID) are
//! skipped, never treated as a match.

/// Parse one `/etc/passwd` or `/etc/group` line and return the numeric ID
/// (field index 2) when the entry name matches `expected` exactly.
pub fn parse_identity_id_line(line: &str, expected: &str) -> Option<u32> {
    let mut fields = line.split(':');
    let name = fields.next()?;
    let _password = fields.next()?;
    let id = fields.next()?.parse::<u32>().ok()?;
    (name == expected).then_some(id)
}

/// Look up the numeric ID of `expected` in a whole passwd/group database,
/// skipping malformed lines. Works for both `/etc/passwd` (uid) and
/// `/etc/group` (gid) since the ID sits at field index 2 in both.
pub fn lookup_identity_id(database: &str, expected: &str) -> Option<u32> {
    database
        .lines()
        .find_map(|line| parse_identity_id_line(line, expected))
}

/// Parse one `/etc/group` line: `(gid, members)` when the group name
/// matches `expected` exactly. A missing members field yields an empty
/// member list (the line is still a valid group entry).
pub fn parse_group_line(line: &str, expected: &str) -> Option<(u32, Vec<String>)> {
    let mut fields = line.split(':');
    let name = fields.next()?;
    let _password = fields.next()?;
    let group_id = fields.next()?.parse::<u32>().ok()?;
    let members = fields.next().unwrap_or_default();
    (name == expected).then(|| {
        (
            group_id,
            members
                .split(',')
                .filter(|member| !member.is_empty())
                .map(ToString::to_string)
                .collect(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str = "root:x:0:0:root:/root:/bin/sh\noperator:x:1000:1000::/home/operator:/bin/sh\n";

    #[test]
    fn passwd_uid_requires_an_exact_name() {
        assert_eq!(lookup_identity_id(PASSWD, "operator"), Some(1000));
        assert_eq!(lookup_identity_id(PASSWD, "root"), Some(0));
        assert_eq!(lookup_identity_id(PASSWD, "oper"), None);
        assert_eq!(lookup_identity_id(PASSWD, "missing"), None);
    }

    #[test]
    fn group_gid_uses_the_same_field_index_two() {
        let database = "root:x:0:\ncynic:x:1444:\n";
        assert_eq!(lookup_identity_id(database, "cynic"), Some(1444));
        assert_eq!(lookup_identity_id(database, "missing"), None);
    }

    #[test]
    fn malformed_lines_are_skipped_never_matched() {
        let database = "short\nbad:x:notanumber:0\noperator:x:1000:1000::/h:/bin/sh\n";
        assert_eq!(lookup_identity_id(database, "operator"), Some(1000));
        assert_eq!(lookup_identity_id(database, "bad"), None);
        assert_eq!(lookup_identity_id(database, "short"), None);
    }

    #[test]
    fn group_line_returns_id_and_members() {
        assert_eq!(
            parse_group_line("cynic-agent:x:1901:cynic,operator", "cynic-agent"),
            Some((1901, vec!["cynic".to_string(), "operator".to_string()]))
        );
        assert_eq!(parse_group_line("cynic:x:1900:cynic", "cynic-agent"), None);
        assert_eq!(
            parse_group_line("cynic:x:1900:", "cynic"),
            Some((1900, Vec::new()))
        );
        assert_eq!(parse_group_line("cynic:x:notanumber:cynic", "cynic"), None);
    }

    #[test]
    fn identity_id_line_rejects_a_non_numeric_id() {
        assert_eq!(parse_identity_id_line("cynic:x:abc:0", "cynic"), None);
        assert_eq!(parse_identity_id_line("cynic:x:1000", "cynic"), Some(1000));
    }
}
