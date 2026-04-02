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
    let results_offset = manager
        .get_range(start, end, Some(2), Some(2))
        .await
        .unwrap();
    assert_eq!(results_offset.len(), 2);
}

// ==================== Pin/Unpin Tests (Consolidated) ====================

use loom::services::memory::manager::MemoryError;

#[tokio::test]
async fn test_pin_operations() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    // 1. pin 不存在的 memory 应该报错
    let result = manager.pin(99999, Some("Nonexistent".to_string())).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, MemoryError::NotFound(id) if id == 99999));

    // 2. 创建 memory 并 pin 成功
    let mut fragments = vec![create_test_fragment("Memory to pin", MemoryKind::Thought)];
    let ids = manager.append(&mut fragments).await.unwrap();

    // 初始状态: is_pinned 应为 false
    assert!(!manager.is_pinned(ids[0]).await.unwrap());

    // pin 成功
    let result = manager
        .pin(ids[0], Some("Important context".to_string()))
        .await;
    assert!(result.is_ok());

    // pin 后: is_pinned 应为 true
    assert!(manager.is_pinned(ids[0]).await.unwrap());

    // 3. 重复 pin 应该报错
    let result = manager.pin(ids[0], Some("Second pin".to_string())).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, MemoryError::AlreadyPinned(id) if id == ids[0]));
}

#[tokio::test]
async fn test_unpin_operations() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    // 1. unpin 不存在的 pinned 应该报错
    let result = manager.unpin(99999).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, MemoryError::NotFound(id) if id == 99999));

    // 2. 创建、pin、然后 unpin
    let mut fragments = vec![create_test_fragment("To be unpinned", MemoryKind::Thought)];
    let ids = manager.append(&mut fragments).await.unwrap();

    manager
        .pin(ids[0], Some("Will unpin".to_string()))
        .await
        .unwrap();
    assert!(manager.is_pinned(ids[0]).await.unwrap());

    // unpin 成功
    let result = manager.unpin(ids[0]).await;
    assert!(result.is_ok());

    // unpin 后: is_pinned 应为 false
    assert!(!manager.is_pinned(ids[0]).await.unwrap());
}

#[tokio::test]
async fn test_pinned_queries_and_protection() {
    let (_container, db) = setup_test_db().await;
    let manager = MemoryManager::new(db, 0);

    // 1. 初始状态: get_pinned 应返回空
    let pinned = manager.get_pinned().await.unwrap();
    assert!(pinned.is_empty());

    // 2. 创建 memories 并 pin 部分内容
    let mut fragments = vec![
        create_test_fragment("Pinned memory 1", MemoryKind::Thought),
        create_test_fragment("Pinned memory 2", MemoryKind::Action),
        create_test_fragment("Unpinned memory", MemoryKind::Event),
    ];
    let ids = manager.append(&mut fragments).await.unwrap();

    manager
        .pin(ids[0], Some("Reason 1".to_string()))
        .await
        .unwrap();
    manager
        .pin(ids[1], Some("Reason 2".to_string()))
        .await
        .unwrap();
    // ids[2] 不 pin

    // 3. get_pinned 应返回正确的数据
    let pinned = manager.get_pinned().await.unwrap();
    assert_eq!(pinned.len(), 2);

    let contents: Vec<&str> = pinned.iter().map(|p| p.fragment.content.as_str()).collect();
    assert!(contents.contains(&"Pinned memory 1"));
    assert!(contents.contains(&"Pinned memory 2"));
    assert!(!contents.contains(&"Unpinned memory"));

    // 4. 验证 JOIN 数据完整性 (data integrity)
    let reasons: Vec<Option<&String>> = pinned.iter().map(|p| p.reason.as_ref()).collect();
    assert!(
        reasons
            .iter()
            .any(|r| r.map(|s| s.as_str()) == Some("Reason 1"))
    );
    assert!(
        reasons
            .iter()
            .any(|r| r.map(|s| s.as_str()) == Some("Reason 2"))
    );

    // 验证 kind 正确关联
    for p in &pinned {
        if p.fragment.content == "Pinned memory 1" {
            assert_eq!(p.fragment.kind, MemoryKind::Thought);
        } else if p.fragment.content == "Pinned memory 2" {
            assert_eq!(p.fragment.kind, MemoryKind::Action);
        }
    }

    // 5. 不能删除 pinned memory
    let result = manager.delete(&[ids[0]]).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, MemoryError::MemoryPinned(id) if id == ids[0]));

    // 验证 memory 仍然存在
    let memory = manager.get_one(ids[0]).await.unwrap();
    assert_eq!(memory.content, "Pinned memory 1");
}
