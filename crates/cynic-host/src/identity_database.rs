pub(crate) fn resolve_identity_id(database: &str, expected: &str) -> Option<u32> {
    std::fs::read_to_string(database)
        .ok()?
        .lines()
        .find_map(|line| {
            let mut fields = line.split(':');
            (fields.next()? == expected)
                .then(|| fields.nth(1)?.parse::<u32>().ok())
                .flatten()
        })
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
}
