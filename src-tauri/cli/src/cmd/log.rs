use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum LogCmd {
    /// Show recent audit log entries.
    Show {
        /// Maximum number of entries to return.
        #[arg(long, default_value_t = 20)]
        limit: i64,
        /// Filter by source (app, cli, ai).
        #[arg(long)]
        source: Option<String>,
        /// Filter by table (projects, memos, schedules, ...).
        #[arg(long)]
        table: Option<String>,
        /// Include entries that have been undone.
        #[arg(long)]
        include_undone: bool,
    },
    /// Undo the most recent N mutations.
    Undo {
        /// Number of entries to undo.
        #[arg(default_value_t = 1)]
        count: i64,
    },
    /// Redo the most recently undone N mutations.
    Redo {
        /// Number of entries to redo.
        #[arg(default_value_t = 1)]
        count: i64,
    },
}

pub fn dispatch(db_flag: Option<&str>, sub: LogCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        LogCmd::Show { limit, source, table, include_undone } => {
            let entries = hearth_core::audit::list(
                &conn,
                limit,
                source.as_deref(),
                table.as_deref(),
                include_undone,
            )?;
            crate::util::emit_ok(serde_json::to_value(&entries).unwrap());
        }
        LogCmd::Undo { count } => {
            let done = hearth_core::audit::undo(&mut conn, count)?;
            crate::util::emit_ok(serde_json::json!({
                "undone": done.len(),
                "entries": serde_json::to_value(&done).unwrap(),
            }));
        }
        LogCmd::Redo { count } => {
            let done = hearth_core::audit::redo(&mut conn, count)?;
            crate::util::emit_ok(serde_json::json!({
                "redone": done.len(),
                "entries": serde_json::to_value(&done).unwrap(),
            }));
        }
    }
    Ok(())
}
