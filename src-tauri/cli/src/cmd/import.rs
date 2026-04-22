use anyhow::{bail, Context, Result};
use clap::Args;

#[derive(Args)]
pub struct ImportArgs {
    /// Path to the JSON export file to import.
    file: String,
    /// Replace all data (after creating a backup). Requires --yes.
    #[arg(long)]
    replace: bool,
    /// Merge imported data with existing data, skipping duplicates.
    #[arg(long, conflicts_with = "replace")]
    merge: bool,
    /// Preview the import without writing to the database.
    #[arg(long)]
    dry_run: bool,
    /// Confirm destructive replace operation (required with --replace).
    #[arg(long)]
    yes: bool,
}

pub fn dispatch(db_flag: Option<&str>, args: ImportArgs) -> Result<()> {
    if !args.replace && !args.merge && !args.dry_run {
        bail!("specify --merge, --replace, or --dry-run");
    }
    if args.replace && !args.yes && !args.dry_run {
        bail!("--replace requires --yes to confirm the destructive operation");
    }

    // Read the dump file
    let json_bytes = std::fs::read(&args.file)
        .with_context(|| format!("reading import file: {}", args.file))?;
    let dump: hearth_core::export::Dump =
        serde_json::from_slice(&json_bytes).context("parsing import JSON")?;

    let p = crate::db::resolve_db_path(db_flag)?;
    let mut conn = crate::db::open(&p)?;

    if args.replace && !args.dry_run {
        // Backup the current DB
        let ts = chrono::Local::now().format("%Y%m%dT%H%M%S");
        let backup_path = p
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(format!("pre-import-{ts}.db"));
        std::fs::copy(&p, &backup_path)
            .with_context(|| format!("creating backup at {}", backup_path.display()))?;

        // Truncate tables in dependency order
        conn.execute_batch(
            "DELETE FROM audit_log;
             DELETE FROM memos;
             DELETE FROM schedules;
             DELETE FROM projects;
             DELETE FROM categories;",
        )
        .context("truncating tables")?;

        eprintln!(
            "backup created: {}",
            backup_path.display()
        );
    }

    let report =
        hearth_core::export::import_json_merge(&mut conn, &dump, args.dry_run)
            .context("import_json_merge failed")?;

    crate::util::emit_ok(serde_json::to_value(&report).unwrap());
    Ok(())
}
