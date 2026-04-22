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
