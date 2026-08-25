//! Shared application state.

use std::sync::Arc;

use {{crate_name}}_db::Database;

use crate::config::Config;

/// State handed to every handler.
///
/// One `Arc` wraps the whole struct rather than each field: a single refcount
/// bump per request, and adding a field later does not change any handler
/// signature. `Database` is itself a cheap-clone pool handle, so cloning this
/// opens nothing.
#[derive(Debug, Clone)]
pub struct AppState(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    db: Database,
    config: Config,
}

impl AppState {
    pub fn new(db: Database, config: Config) -> Self {
        Self(Arc::new(Inner { db, config }))
    }

    pub fn db(&self) -> &Database {
        &self.0.db
    }

    pub fn config(&self) -> &Config {
        &self.0.config
    }
}
