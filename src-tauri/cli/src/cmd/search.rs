use anyhow::Result;
use clap::Args;
use hearth_core::search;

#[derive(Args)]
pub struct SearchArgs {
    /// Full-text search query.
    pub query: String,
    /// Comma-separated scopes to include: project,memo,schedule (default: all).
    #[arg(long, value_delimiter = ',')]
    pub scope: Vec<String>,
    /// Maximum results per scope.
    #[arg(long, default_value_t = 20)]
    pub limit: i64,
}

pub fn dispatch(db_path_flag: Option<&str>, args: SearchArgs) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let conn = crate::db::open(&p)?;
    let hits = search::search_all(&conn, &args.query, args.limit)?;
    let filtered: Vec<_> = if args.scope.is_empty() {
        hits
    } else {
        hits.into_iter()
            .filter(|h| args.scope.contains(&h.kind))
            .collect()
    };
    crate::util::emit_ok(serde_json::to_value(&filtered).unwrap());
    Ok(())
}
