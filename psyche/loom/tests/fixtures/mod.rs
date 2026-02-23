use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use std::collections::HashMap;
use testcontainers_modules::{mysql::Mysql, testcontainers::runners::AsyncRunner};

use loom::memory::types::MemorySource;
use loom::services::memory::manager::MysqlMemoryManager;

/// Setup a test database using testcontainers.
/// Returns a tuple of (container, database connection).
/// The container will be automatically cleaned up when dropped.
pub async fn setup_test_db() -> (
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
    loom::services::db_migration::Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    (container, db)
}

/// Create a test memory source with the given channel and identifier
pub fn test_source(channel: &str, identifier: &str) -> MemorySource {
    MemorySource {
        channel: channel.to_string(),
        identifier: identifier.to_string(),
        metadata: HashMap::new(),
    }
}

/// Create a test memory source with metadata
pub fn test_source_with_metadata(
    channel: &str,
    identifier: &str,
    metadata: HashMap<String, String>,
) -> MemorySource {
    MemorySource {
        channel: channel.to_string(),
        identifier: identifier.to_string(),
        metadata,
    }
}

/// Create MysqlMemoryManager from a database connection
pub fn create_mysql_manager(db: &sea_orm::DatabaseConnection) -> MysqlMemoryManager {
    MysqlMemoryManager::new(db.clone())
}
