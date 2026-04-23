use anyhow::Result;
use clap::Subcommand;
use hearth_core::audit::Source;
use hearth_core::memos::{self, NewMemo, UpdateMemo};

#[derive(Subcommand)]
pub enum MemoCmd {
    /// List all memos.
    List,
    /// Get a memo by id.
    Get { id: i64 },
    /// Create a new memo.
    Create {
        content: String,
        #[arg(long, default_value = "yellow")]
        color: String,
        /// Attach to a project id.
        #[arg(long)]
        project: Option<i64>,
    },
    /// Update a memo's fields.
    Update {
        id: i64,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        color: Option<String>,
        /// Attach to a project id (conflicts with --detach).
        #[arg(long, conflicts_with = "detach")]
        project: Option<i64>,
        /// Detach from any project (conflicts with --project).
        #[arg(long)]
        detach: bool,
    },
    /// Delete a memo by id.
    Delete { id: i64 },
}

pub fn dispatch(db_path_flag: Option<&str>, sub: MemoCmd, pretty: bool) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        MemoCmd::List => {
            let all = memos::list(&conn)?;
            let val = serde_json::to_value(&all).unwrap();
            if pretty {
                crate::util::emit_ok_pretty(&val, &["id", "color", "content", "project_id"]);
            } else {
                crate::util::emit_ok(val);
            }
        }
        MemoCmd::Get { id } => match memos::get(&conn, id)? {
            Some(m) => crate::util::emit_ok(serde_json::to_value(&m).unwrap()),
            None => {
                crate::util::emit_err(&format!("memo {id} not found"), Some("try 'hearth memo list'"));
                std::process::exit(1);
            }
        },
        MemoCmd::Create { content, color, project } => {
            let m = memos::create(
                &mut conn,
                Source::Cli,
                &NewMemo {
                    content: &content,
                    color: &color,
                    project_id: project,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&m).unwrap());
        }
        MemoCmd::Update { id, content, color, project, detach } => {
            // Tri-state: --project N => Some(Some(N)), --detach => Some(None), neither => None
            let project_id: Option<Option<i64>> = if detach {
                Some(None)
            } else if let Some(pid) = project {
                Some(Some(pid))
            } else {
                None
            };
            let m = memos::update(
                &mut conn,
                Source::Cli,
                id,
                &UpdateMemo {
                    content: content.as_deref(),
                    color: color.as_deref(),
                    project_id,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&m).unwrap());
        }
        MemoCmd::Delete { id } => {
            memos::delete(&mut conn, Source::Cli, id)?;
            crate::util::emit_ok(serde_json::json!({ "deleted": id }));
        }
    }
    Ok(())
}
