mod cmd;
mod db;
mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hearth", version, about = "Hearth CLI — agent-friendly workspace control")]
struct Cli {
    /// SQLite DB path override. Falls back to $HEARTH_DB then the default app data path.
    #[arg(long, global = true)]
    db: Option<String>,

    /// Verbose tracing (equivalent to RUST_LOG=debug).
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Human-readable table output for list commands.
    #[arg(long, global = true)]
    pretty: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// DB-level utilities.
    Db {
        #[command(subcommand)]
        sub: DbCmd,
    },
    /// Project management.
    Project {
        #[command(subcommand)]
        sub: crate::cmd::project::ProjectCmd,
    },
    /// Memo management.
    Memo {
        #[command(subcommand)]
        sub: crate::cmd::memo::MemoCmd,
    },
    /// Schedule management.
    Schedule {
        #[command(subcommand)]
        sub: crate::cmd::schedule::ScheduleCmd,
    },
    /// Category management.
    Category {
        #[command(subcommand)]
        sub: crate::cmd::category::CategoryCmd,
    },
    /// Full-text search across projects, memos, and schedules.
    Search(crate::cmd::search::SearchArgs),
    /// Show today's schedules, P0 projects, and recent memos.
    Today,
    /// Show overdue schedules and stale projects.
    Overdue,
    /// Show aggregate statistics.
    Stats,
    /// Audit log: show, undo, redo.
    Log {
        #[command(subcommand)]
        sub: crate::cmd::log::LogCmd,
    },
    /// Undo the most recent N mutations (shortcut for `log undo`).
    Undo {
        /// Number of entries to undo.
        #[arg(default_value_t = 1)]
        count: i64,
    },
    /// Redo the most recently undone N mutations (shortcut for `log redo`).
    Redo {
        /// Number of entries to redo.
        #[arg(default_value_t = 1)]
        count: i64,
    },
    /// Export workspace data (JSON or SQLite copy).
    Export(crate::cmd::export::ExportArgs),
    /// Import workspace data from a JSON export.
    Import(crate::cmd::import::ImportArgs),
}

#[derive(Subcommand)]
enum DbCmd {
    /// Print the resolved DB path.
    Path,
    /// VACUUM + integrity_check.
    Vacuum,
    /// (Re)run migrations.
    Migrate,
}

fn main() {
    if let Err(e) = run() {
        crate::util::emit_err(&format!("{e:#}"), None);
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new("debug"))
            .with_writer(std::io::stderr)
            .init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
            )
            .with_writer(std::io::stderr)
            .try_init();
    }

    match cli.command {
        Commands::Db { sub } => cmd_db(cli.db.as_deref(), sub),
        Commands::Project { sub } => {
            crate::cmd::project::dispatch(cli.db.as_deref(), sub, cli.pretty)
        }
        Commands::Memo { sub } => crate::cmd::memo::dispatch(cli.db.as_deref(), sub, cli.pretty),
        Commands::Schedule { sub } => {
            crate::cmd::schedule::dispatch(cli.db.as_deref(), sub, cli.pretty)
        }
        Commands::Category { sub } => crate::cmd::category::dispatch(cli.db.as_deref(), sub),
        Commands::Search(args) => crate::cmd::search::dispatch(cli.db.as_deref(), args),
        Commands::Today => crate::cmd::views::today(cli.db.as_deref()),
        Commands::Overdue => crate::cmd::views::overdue(cli.db.as_deref()),
        Commands::Stats => crate::cmd::views::stats(cli.db.as_deref()),
        Commands::Log { sub } => crate::cmd::log::dispatch(cli.db.as_deref(), sub),
        Commands::Undo { count } => {
            crate::cmd::log::dispatch(cli.db.as_deref(), crate::cmd::log::LogCmd::Undo { count })
        }
        Commands::Redo { count } => {
            crate::cmd::log::dispatch(cli.db.as_deref(), crate::cmd::log::LogCmd::Redo { count })
        }
        Commands::Export(args) => crate::cmd::export::dispatch(cli.db.as_deref(), args),
        Commands::Import(args) => crate::cmd::import::dispatch(cli.db.as_deref(), args),
    }
}

fn cmd_db(db_flag: Option<&str>, sub: DbCmd) -> Result<()> {
    match sub {
        DbCmd::Path => {
            let p = crate::db::resolve_db_path(db_flag)?;
            crate::util::emit_ok(serde_json::json!({ "path": p.to_string_lossy() }));
            Ok(())
        }
        DbCmd::Vacuum => {
            let p = crate::db::resolve_db_path(db_flag)?;
            let conn = crate::db::open(&p)?;
            conn.execute("VACUUM", [])?;
            let integrity: String =
                conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
            crate::util::emit_ok(serde_json::json!({
                "path": p.to_string_lossy(),
                "integrity_check": integrity,
            }));
            Ok(())
        }
        DbCmd::Migrate => {
            let p = crate::db::resolve_db_path(db_flag)?;
            let _ = crate::db::open(&p)?; // init_db runs migrations
            crate::util::emit_ok(serde_json::json!({ "migrated": true }));
            Ok(())
        }
    }
}
