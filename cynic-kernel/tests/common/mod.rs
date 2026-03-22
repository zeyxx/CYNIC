use cynic_kernel::storage::SurrealHttpStorage;
use std::time::{SystemTime, UNIX_EPOCH};

/// Create an isolated test database and return a connected SurrealHttpStorage.
/// Each test gets its own DB: `test_<unix_millis>_<suffix>`.
/// Requires SURREALDB_PASS env var and SurrealDB running on localhost:8000.
/// Returns None (with a skip message) if SurrealDB is unreachable.
pub async fn setup_test_db(suffix: &str) -> Option<SurrealHttpStorage> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let db_name = format!("test_{}_{}", millis, suffix);
    match SurrealHttpStorage::init_with("http://localhost:8000", "cynic_test", &db_name).await {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("[SKIP] SurrealDB unavailable (localhost:8000): {} — skipping test", e);
            None
        }
    }
}

/// Drop the test database to clean up.
pub async fn teardown_test_db(db: &SurrealHttpStorage) {
    let _ = db.query(&format!("REMOVE DATABASE `{}`;", db.db_name())).await;
}
