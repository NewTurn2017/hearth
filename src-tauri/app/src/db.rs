//! Schema lives in hearth-core. Re-export so the app uses the same migrations.
#[allow(unused_imports)]
pub use hearth_core::db::{init_db, init_db_with_recovery, DbInitOutcome};
