mod fixtures;

use fixtures::setup_test_db;

#[tokio::test]
async fn test_container_starts() {
    let (_container, db) = setup_test_db().await;

    // Just verify we can ping the database
    let result = db.ping().await;
    assert!(result.is_ok(), "Database ping should succeed");
}
