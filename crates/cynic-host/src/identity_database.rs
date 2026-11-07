//! Adapter OS — live lookup of numeric IDs in passwd/group databases.
//!
//! The parsing itself is pure and shared: `cynic_types::identity_database`
//! holds the single implementation of the `name:password:ID:...` layout
//! (ID at field index 2 for both `/etc/passwd` and `/etc/group`). This
//! module only adds the file read, so host callers do not duplicate the
//! parsing (previously diverged between `identity_database`,
//! `adapters/agent_access`, `cynic-agent::probe`, `cynic-secrets`).

/// Resolve the numeric ID (uid for `/etc/passwd`, gid for `/etc/group`)
/// of `expected` by reading the database file at `database` (a path).
pub(crate) fn resolve_identity_id(database: &str, expected: &str) -> Option<u32> {
    cynic_types::identity_database::lookup_identity_id(
        &std::fs::read_to_string(database).ok()?,
        expected,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_numeric_id_without_hardcoding_current_principal() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("group");
        std::fs::write(&database, "root:x:0:\ncynic:x:1444:\n").unwrap();

        assert_eq!(
            resolve_identity_id(&database.display().to_string(), "cynic"),
            Some(1444)
        );
        assert_eq!(
            resolve_identity_id(&database.display().to_string(), "missing"),
            None
        );
    }

    #[test]
    fn resolves_passwd_uid_by_exact_name() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("passwd");
        std::fs::write(
            &database,
            "root:x:0:0:root:/root:/bin/sh\noperator:x:1000:1000::/home/operator:/bin/sh\n",
        )
        .unwrap();

        assert_eq!(
            resolve_identity_id(&database.display().to_string(), "operator"),
            Some(1000)
        );
        assert_eq!(
            resolve_identity_id(&database.display().to_string(), "oper"),
            None
        );
        assert_eq!(
            resolve_identity_id(&database.display().to_string(), "missing"),
            None
        );
    }
}
