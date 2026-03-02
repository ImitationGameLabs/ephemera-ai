use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use testcontainers_modules::{mysql::Mysql, testcontainers::runners::AsyncRunner};

use loom::services::memory::manager::MemoryManager;

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

/// Create MemoryManager from a database connection
pub fn create_memory_manager(db: &sea_orm::DatabaseConnection) -> MemoryManager {
    MemoryManager::new(db.clone(), 0)
}
