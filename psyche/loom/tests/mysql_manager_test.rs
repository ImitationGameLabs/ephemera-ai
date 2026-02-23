mod fixtures;

use fixtures::{create_mysql_manager, setup_test_db, test_source};
use loom::memory::builder::MemoryFragmentBuilder;
use loom::services::memory::manager::MysqlError;

#[tokio::test]
async fn test_save_operations() {
    let (_container, db) = setup_test_db().await;
    let manager = create_mysql_manager(&db);

    // === Test: save single memory ===
    let fragment =
        MemoryFragmentBuilder::new("test content".to_string(), test_source("dialogue", "alice"))
            .build();

    let ids = manager.save(&[fragment]).await.unwrap();
    assert_eq!(ids.len(), 1);
    assert!(ids[0] > 0);

    // === Test: save batch memories ===
    let fragments = vec![
        MemoryFragmentBuilder::new("first memory".to_string(), test_source("dialogue", "alice"))
            .build(),
        MemoryFragmentBuilder::new("second memory".to_string(), test_source("dialogue", "bob"))
            .build(),
        MemoryFragmentBuilder::new(
            "third memory".to_string(),
            test_source("information", "api"),
        )
        .build(),
    ];

    let ids = manager.save(&fragments).await.unwrap();
    assert_eq!(ids.len(), 3);
    // All IDs should be positive and unique
    assert!(ids.iter().all(|&id| id > 0));
    assert_eq!(
        ids.len(),
        ids.iter().collect::<std::collections::HashSet<_>>().len()
    );
}

#[tokio::test]
async fn test_get_operations() {
    let (_container, db) = setup_test_db().await;
    let manager = create_mysql_manager(&db);

    // === Test: get_one success ===
    let fragment = MemoryFragmentBuilder::new(
        "retrievable content".to_string(),
        test_source("dialogue", "test_user"),
    )
    .build();

    let ids = manager.save(&[fragment.clone()]).await.unwrap();
    let saved_id = ids[0];

    let retrieved = manager.get_one(saved_id).await.unwrap();

    assert_eq!(retrieved.id, saved_id);
    assert_eq!(retrieved.content, "retrievable content");
    assert_eq!(retrieved.source.channel, "dialogue");
    assert_eq!(retrieved.source.identifier, "test_user");

    // === Test: get_one not found ===
    let result = manager.get_one(999999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        MysqlError::NotFound(id) => assert_eq!(id, 999999),
        _ => panic!("Expected NotFound error"),
    }

    // === Test: get multiple by ids ===
    let fragments = vec![
        MemoryFragmentBuilder::new("memory one".to_string(), test_source("dialogue", "user1"))
            .build(),
        MemoryFragmentBuilder::new("memory two".to_string(), test_source("dialogue", "user2"))
            .build(),
        MemoryFragmentBuilder::new("memory three".to_string(), test_source("dialogue", "user3"))
            .build(),
    ];

    let ids = manager.save(&fragments).await.unwrap();

    // Get first and third memories
    let retrieved = manager.get(&[ids[0], ids[2]]).await.unwrap();

    assert_eq!(retrieved.len(), 2);
    let retrieved_ids: Vec<i64> = retrieved.iter().map(|f| f.id).collect();
    assert!(retrieved_ids.contains(&ids[0]));
    assert!(retrieved_ids.contains(&ids[2]));
    assert!(!retrieved_ids.contains(&ids[1]));
}

#[tokio::test]
async fn test_delete_operations() {
    let (_container, db) = setup_test_db().await;
    let manager = create_mysql_manager(&db);

    // === Test: delete success ===
    let fragment = MemoryFragmentBuilder::new(
        "to be deleted".to_string(),
        test_source("dialogue", "deleter"),
    )
    .build();

    let ids = manager.save(&[fragment]).await.unwrap();
    let saved_id = ids[0];

    // Verify it exists
    let retrieved = manager.get_one(saved_id).await.unwrap();
    assert_eq!(retrieved.content, "to be deleted");

    // Delete it
    manager.delete(&[saved_id]).await.unwrap();

    // Verify it's gone
    let result = manager.get_one(saved_id).await;
    assert!(result.is_err());

    // === Test: delete not found ===
    let result = manager.delete(&[999999]).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        MysqlError::NotFound(id) => assert_eq!(id, 0), // Batch delete uses 0 as indicator
        _ => panic!("Expected NotFound error"),
    }

    // === Test: delete batch ===
    let fragments = vec![
        MemoryFragmentBuilder::new("delete me 1".to_string(), test_source("dialogue", "user"))
            .build(),
        MemoryFragmentBuilder::new("delete me 2".to_string(), test_source("dialogue", "user"))
            .build(),
        MemoryFragmentBuilder::new("keep me".to_string(), test_source("dialogue", "user")).build(),
    ];

    let ids = manager.save(&fragments).await.unwrap();

    // Delete first two
    manager.delete(&[ids[0], ids[1]]).await.unwrap();

    // Verify they're gone
    assert!(manager.get_one(ids[0]).await.is_err());
    assert!(manager.get_one(ids[1]).await.is_err());

    // Verify third still exists
    let remaining = manager.get_one(ids[2]).await.unwrap();
    assert_eq!(remaining.content, "keep me");
}
