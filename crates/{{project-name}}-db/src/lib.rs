//! Postgres access: connection pool, SeaORM entities, and repositories.
//!
//! This crate owns every detail of *how* data is stored.
//! `{{project-name}}-core` decides what should happen; this crate makes it
//! durable.

pub mod connection;
pub mod entities;
pub mod repository;

pub use connection::{Database, DbConfig};
