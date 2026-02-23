mod fixtures;

use fixtures::{create_mysql_manager, setup_test_db, test_source};
use loom::memory::builder::MemoryFragmentBuilder;
use loom::memory::models::MemoryQuery;
use loom::memory::types::MemoryFragment;
use loom::services::memory::manager::{HybridError, HybridMemoryManager, Manager};

fn create_hybrid_manager(db: &sea_orm::DatabaseConnection) -> HybridMemoryManager {
    HybridMemoryManager::new(create_mysql_manager(db))
}

#[tokio::test]
async fn test_append_operations() {
    let (_container, db) = setup_test_db().await;
    let manager = create_hybrid_manager(&db);

    // === Test: append empty returns empty ===
    let mut empty_fragments: Vec<MemoryFragment> = vec![];
    let ids = manager.append(&mut empty_fragments).await.unwrap();

    assert!(ids.is_empty());

    // === Test: append saves to mysql ===
    let mut fragments = vec![
        MemoryFragmentBuilder::new(
            "hybrid test content".to_string(),
            test_source("dialogue", "test_user"),
        )
        .build(),
    ];

    let ids = manager.append(&mut fragments).await.unwrap();

    assert_eq!(ids.len(), 1);
    assert!(ids[0] > 0);

    // Verify the fragment was saved by retrieving it
    let retrieved = manager.get(ids[0]).await.unwrap();
    assert_eq!(retrieved.content, "hybrid test content");

    // === Test: append batch saves all ===
    let mut fragments = vec![
        MemoryFragmentBuilder::new("first".to_string(), test_source("dialogue", "a")).build(),
        MemoryFragmentBuilder::new("second".to_string(), test_source("dialogue", "b")).build(),
        MemoryFragmentBuilder::new("third".to_string(), test_source("dialogue", "c")).build(),
    ];

    let ids = manager.append(&mut fragments).await.unwrap();

    assert_eq!(ids.len(), 3);
    assert!(ids.iter().all(|&id| id > 0));
}

#[tokio::test]
async fn test_get_and_delete_delegation() {
    let (_container, db) = setup_test_db().await;
    let manager = create_hybrid_manager(&db);

    // === Test: get delegates to mysql ===
    // First save a fragment
    let mut fragments = vec![
        MemoryFragmentBuilder::new(
            "get test content".to_string(),
            test_source("information", "api"),
        )
        .build(),
    ];
    let ids = manager.append(&mut fragments).await.unwrap();
    let saved_id = ids[0];

    // Then retrieve it through hybrid manager
    let retrieved = manager.get(saved_id).await.unwrap();

    assert_eq!(retrieved.id, saved_id);
    assert_eq!(retrieved.content, "get test content");
    assert_eq!(retrieved.source.channel, "information");
    assert_eq!(retrieved.source.identifier, "api");

    // === Test: get not found returns error ===
    let result = manager.get(999999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        HybridError::Mysql(_) => {} // Expected
    }

    // === Test: delete delegates to mysql ===
    // First save a fragment
    let mut fragments = vec![
        MemoryFragmentBuilder::new("to be deleted".to_string(), test_source("dialogue", "user"))
            .build(),
    ];
    let ids = manager.append(&mut fragments).await.unwrap();
    let saved_id = ids[0];

    // Verify it exists
    let retrieved = manager.get(saved_id).await.unwrap();
    assert_eq!(retrieved.content, "to be deleted");

    // Delete it
    manager.delete(saved_id).await.unwrap();

    // Verify it's gone
    let result = manager.get(saved_id).await;
    assert!(result.is_err());

    // === Test: delete not found returns error ===
    let result = manager.delete(999999).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_recall_operations() {
    let (_container, db) = setup_test_db().await;
    let manager = create_hybrid_manager(&db);

    // === Test: recall returns empty for now ===
    // Save some fragments
    let mut fragments = vec![
        MemoryFragmentBuilder::new("test memory".to_string(), test_source("dialogue", "user"))
            .build(),
    ];
    manager.append(&mut fragments).await.unwrap();

    // Recall should return empty results (not yet implemented)
    let query = MemoryQuery {
        keywords: "test".to_string(),
        time_range: None,
    };
    let result = manager.recall(&query).await.unwrap();

    assert!(result.memories.is_empty());
}
