//! Repositories: the only place SQL is written.
//!
//! Each takes a `Database` handle (cheap to clone) and exposes intent-named
//! methods rather than query builders, so callers cannot accidentally
//! construct a query that bypasses a soft-delete or tenancy filter.
//!
//! Add one `pub mod` per aggregate/table group as you build them out, and
//! re-export its public repository type(s) here.
