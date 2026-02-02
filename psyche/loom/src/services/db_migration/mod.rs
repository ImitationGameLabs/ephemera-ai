pub use sea_orm_migration::prelude::*;

mod m20260202_01_create_memory_fragments_table;
mod m20260202_02_create_system_configs_table;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260202_01_create_memory_fragments_table::Migration),
            Box::new(m20260202_02_create_system_configs_table::Migration),
        ]
    }
}
