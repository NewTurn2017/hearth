use anyhow::Result;
use clap::Subcommand;
use hearth_core::audit::Source;
use hearth_core::memos::{
    create_memo_tag, delete_memo_tag, list_memo_tags, update_memo_tag, UpdateMemoTag,
};

#[derive(Subcommand)]
pub enum MemoTagCmd {
    /// List all memo tags.
    List,
    /// Create a new memo tag.
    Create {
        name: String,
        #[arg(long)]
        color: Option<String>,
    },
    /// Update memo tag fields.
    Update {
        id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        sort_order: Option<i64>,
    },
    /// Delete a memo tag by id.
    Delete { id: i64 },
}

pub fn dispatch(db_path_flag: Option<&str>, sub: MemoTagCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        MemoTagCmd::List => {
            let all = list_memo_tags(&conn)?;
            crate::util::emit_ok(serde_json::to_value(&all).unwrap());
        }
        MemoTagCmd::Create { name, color } => {
            let tag = create_memo_tag(&mut conn, Source::Cli, &name, color.as_deref())?;
            crate::util::emit_ok(serde_json::to_value(&tag).unwrap());
        }
        MemoTagCmd::Update {
            id,
            name,
            color,
            sort_order,
        } => {
            let tag = update_memo_tag(
                &mut conn,
                Source::Cli,
                id,
                &UpdateMemoTag {
                    name: name.as_deref(),
                    color: color.as_deref(),
                    sort_order,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&tag).unwrap());
        }
        MemoTagCmd::Delete { id } => {
            delete_memo_tag(&mut conn, Source::Cli, id)?;
            crate::util::emit_ok(serde_json::json!({ "deleted": id }));
        }
    }
    Ok(())
}
