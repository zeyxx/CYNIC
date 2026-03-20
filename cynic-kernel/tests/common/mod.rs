use cynic_kernel::storage::SurrealHttpStorage;
use std::time::{SystemTime, UNIX_EPOCH};

/// Create an isolated test database and return a connected SurrealHttpStorage.
/// Each test gets its own DB: `test_<unix_millis>_<suffix>`.
/// Requires SURREALDB_PASS env var and SurrealDB running on localhost:8000.
pub async fn setup_test_db(suffix: &str) -> SurrealHttpStorage {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let db_name = format!("test_{}_{}", millis, suffix);
    SurrealHttpStorage::init_with("http://localhost:8000", "cynic_test", &db_name)
        .await
        .expect("Failed to connect to test SurrealDB — is it running on :8000?")
}

/// Drop the test database to clean up.
pub async fn teardown_test_db(db: &SurrealHttpStorage) {
    let _ = db.query(&format!("REMOVE DATABASE `{}`;", db.db_name())).await;
}
