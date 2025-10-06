pub use sea_orm_migration::prelude::*;

mod m20251004_01_create_memory_fragments_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251004_01_create_memory_fragments_table::Migration),
        ]
    }
}