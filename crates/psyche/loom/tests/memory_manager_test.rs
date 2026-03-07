use loom::memory::types::{MemoryFragment, MemoryKind};
use loom::services::db_migration::Migrator;
use loom::services::memory::manager::MemoryManager;
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use testcontainers_modules::{mysql::Mysql, testcontainers::runners::AsyncRunner};
use time::OffsetDateTime;

async fn setup_test_db() -> (
    testcontainers::ContainerAsync<Mysql>,
    sea_orm::DatabaseConnection,
) {
    let container = Mysql::default()
        .start()
        .await
        .expect("Failed to start MySQL container");

    let host_port = container.get_host_port_ipv4(3306).await.unwrap();
    let connection_string = format!("mysql://root@127.0.0.1:{}/test", host_port);

    let db = Database::connect(&connection_string)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    (container, db)
}

fn create_test_fragment(content: &str, kind: MemoryKind) -> MemoryFragment {
    MemoryFragment {
        id: 0, // Will be assigned by manager
        content: content.to_string(),
        timestamp: OffsetDateTime::now_utc(),
        kind,
    }
}

#[tokio::test]
async fn test_save_and_get_memory() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    let fragment = create_test_fragment("Test memory content", MemoryKind::Event);
    let ids = manager.append(&mut vec![fragment.clone()]).await.unwrap();

    assert_eq!(ids.len(), 1);
    let saved_id = ids[0];
    assert!(saved_id > 0);

    // Verify we can retrieve it
    let retrieved = manager.get_one(saved_id).await.unwrap();
    assert_eq!(retrieved.content, "Test memory content");
    assert_eq!(retrieved.kind, MemoryKind::Event);
}

#[tokio::test]
async fn test_save_multiple_memories() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    let mut fragments = vec![
        create_test_fragment("First thought", MemoryKind::Thought),
        create_test_fragment("Second action", MemoryKind::Action),
        create_test_fragment("Third message", MemoryKind::Event),
    ];

    let ids = manager.append(&mut fragments).await.unwrap();
    assert_eq!(ids.len(), 3);

    // All IDs should be unique
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[1], ids[2]);
    assert_ne!(ids[0], ids[2]);
}

#[tokio::test]
async fn test_get_recent_memories() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    // Create multiple memories with small delays to ensure different timestamps
    let mut fragments = vec![
        create_test_fragment("Memory 1", MemoryKind::Event),
        create_test_fragment("Memory 2", MemoryKind::Event),
        create_test_fragment("Memory 3", MemoryKind::Event),
    ];

    manager.append(&mut fragments).await.unwrap();

    // Get recent memories
    let recent = manager.get_recent(2).await.unwrap();
    assert_eq!(recent.len(), 2);
}

#[tokio::test]
async fn test_delete_memory() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    let fragment = create_test_fragment("To be deleted", MemoryKind::Event);
    let ids = manager.append(&mut vec![fragment.clone()]).await.unwrap();
    let saved_id = ids[0];

    // Verify it exists
    manager.get_one(saved_id).await.unwrap();

    // Delete it
    manager.delete(&[saved_id]).await.unwrap();

    // Verify it's gone
    let result = manager.get_one(saved_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_range() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    let now = OffsetDateTime::now_utc();
    let start = now - time::Duration::hours(1);
    let end = now + time::Duration::hours(1);

    let mut fragments = vec![
        create_test_fragment("Range test 1", MemoryKind::Thought),
        create_test_fragment("Range test 2", MemoryKind::Action),
    ];

    manager.append(&mut fragments).await.unwrap();

    // Query within range
    let results = manager.get_range(start, end, None, None).await.unwrap();
    assert!(results.len() >= 2);
}

#[tokio::test]
async fn test_get_range_with_pagination() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    let now = OffsetDateTime::now_utc();
    let start = now - time::Duration::hours(1);
    let end = now + time::Duration::hours(1);

    // Create 5 memories
    let mut fragments: Vec<MemoryFragment> = (0..5)
        .map(|i| create_test_fragment(&format!("Memory {}", i), MemoryKind::Event))
        .collect();

    manager.append(&mut fragments).await.unwrap();

    // Get with limit
    let results = manager.get_range(start, end, Some(2), None).await.unwrap();
    assert_eq!(results.len(), 2);

    // Get with offset
    let results_offset = manager.get_range(start, end, Some(2), Some(2)).await.unwrap();
    assert_eq!(results_offset.len(), 2);
}
