pub use sea_orm_migration::prelude::*;

mod m20251004_01_create_memory_fragments_table;
mod m20251019_01_redesign_memory_source;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251004_01_create_memory_fragments_table::Migration),
            Box::new(m20251019_01_redesign_memory_source::Migration),
        ]
    }
}