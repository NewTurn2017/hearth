use anyhow::{Context, Result};
use clap::{Args, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
pub enum ExportFormat {
    Json,
    Sqlite,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Output format.
    #[arg(long, value_enum, default_value = "json")]
    format: ExportFormat,
    /// Output file path. If omitted (json only), writes raw JSON to stdout.
    #[arg(long, short)]
    out: Option<String>,
    /// Include the audit_log in the JSON dump.
    #[arg(long)]
    include_audit: bool,
}

pub fn dispatch(db_flag: Option<&str>, args: ExportArgs) -> Result<()> {
    let p = crate::db::resolve_db_path(db_flag)?;
    match args.format {
        ExportFormat::Json => {
            let conn = crate::db::open(&p)?;
            let dump = hearth_core::export::export_json(&conn, args.include_audit)
                .context("export_json failed")?;
            let json = serde_json::to_string_pretty(&dump).context("serialization failed")?;
            if let Some(out_path) = args.out {
                std::fs::write(&out_path, json.as_bytes()).context("writing export file")?;
                crate::util::emit_ok(serde_json::json!({ "written": out_path }));
            } else {
                // Raw JSON to stdout — no envelope — for piping (hearth export | jq .)
                println!("{json}");
            }
        }
        ExportFormat::Sqlite => {
            let out_path = args
                .out
                .context("--out <PATH> is required for sqlite format")?;
            std::fs::copy(&p, &out_path)
                .with_context(|| format!("copying db {} -> {}", p.display(), out_path))?;
            crate::util::emit_ok(serde_json::json!({ "written": out_path }));
        }
    }
    Ok(())
}
