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
        /// Memo font size for Focus board emphasis.
        #[arg(long, value_parser = ["small", "normal", "large"])]
        size: Option<String>,
        /// Render the memo content in bold.
        #[arg(long)]
        bold: bool,
        /// Attach a memo-specific tag by name. Repeat to attach multiple tags.
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// Focus board horizontal position. Core clamps values to the board range.
        #[arg(long)]
        focus_x: Option<f64>,
        /// Focus board vertical position. Core clamps values to the board range.
        #[arg(long)]
        focus_y: Option<f64>,
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
        /// Memo font size for Focus board emphasis.
        #[arg(long, value_parser = ["small", "normal", "large"])]
        size: Option<String>,
        /// Set whether memo content should render in bold.
        #[arg(long)]
        bold: Option<bool>,
        /// Replace memo-specific tags by name. Repeat to attach multiple tags.
        #[arg(long = "tag", conflicts_with = "clear_tags")]
        tags: Vec<String>,
        /// Remove all memo-specific tags.
        #[arg(long)]
        clear_tags: bool,
        /// Focus board horizontal position. Core clamps values to the board range.
        #[arg(long)]
        focus_x: Option<f64>,
        /// Focus board vertical position. Core clamps values to the board range.
        #[arg(long)]
        focus_y: Option<f64>,
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
                crate::util::emit_err(
                    &format!("memo {id} not found"),
                    Some("try 'hearth memo list'"),
                );
                std::process::exit(1);
            }
        },
        MemoCmd::Create {
            content,
            color,
            project,
            size,
            bold,
            tags,
            focus_x,
            focus_y,
        } => {
            let m = memos::create(
                &mut conn,
                Source::Cli,
                &NewMemo {
                    content: &content,
                    color: &color,
                    project_id: project,
                    font_size: size.as_deref(),
                    is_bold: Some(bold),
                    focus_x,
                    focus_y,
                    tag_names: tags,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&m).unwrap());
        }
        MemoCmd::Update {
            id,
            content,
            color,
            project,
            detach,
            size,
            bold,
            tags,
            clear_tags,
            focus_x,
            focus_y,
        } => {
            // Tri-state: --project N => Some(Some(N)), --detach => Some(None), neither => None
            let project_id: Option<Option<i64>> = if detach {
                Some(None)
            } else if let Some(pid) = project {
                Some(Some(pid))
            } else {
                None
            };
            let tag_names = if clear_tags {
                Some(Vec::new())
            } else if !tags.is_empty() {
                Some(tags)
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
                    font_size: size.as_deref(),
                    is_bold: bold,
                    focus_x: focus_x.map(Some),
                    focus_y: focus_y.map(Some),
                    tag_names,
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
