//! Migration CLI: `cargo run -p {{project-name}}-migration -- {up,down,status,fresh}`.
//!
//! Exists so migrations can run without a global `sea-orm-cli` install — CI and
//! a fresh checkout need only `cargo`.

use {{crate_name}}_migration::Migrator;
use sea_orm_migration::cli;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    // Reads DATABASE_URL and dispatches the subcommand.
    cli::run_cli(Migrator).await;
}
