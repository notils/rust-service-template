//! SeaORM entities, one module per table.
//!
//! Hand-write these rather than regenerating over them once you've added doc
//! comments explaining *why* a column is shaped the way it is — comments a
//! `sea-orm-cli generate entity` re-run would otherwise discard. Regenerate
//! into a scratch location and hand-port the shape back if you need to.
//!
//! Add one `pub mod example_table;` (+ `pub use example_table::Entity as
//! ExampleTable;`) per table you create in `{{project-name}}-migration`.
