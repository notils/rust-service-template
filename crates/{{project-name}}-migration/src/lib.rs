//! Schema migrations.
//!
//! Rules worth keeping from day one:
//! - Forward-only in production — never edit an applied migration
//! - Never ship a destructive migration in the same release as the code needing it
//! - Run on staging before production, always
//!
//! `m20260101_000001_create_example_table` is a real, working example — copy
//! its shape for your first real migration, then delete it once you have one.

pub use sea_orm_migration::prelude::*;

mod m20260101_000001_create_example_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260101_000001_create_example_table::Migration)]
    }
}
