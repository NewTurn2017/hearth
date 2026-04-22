use anyhow::{Context, Result};
use clap::Subcommand;
use hearth_core::audit::Source;
use hearth_core::projects::{self, NewProject, UpdateProject};

#[derive(Subcommand)]
pub enum ProjectCmd {
    /// List projects.
    List {
        #[arg(long, value_delimiter = ',')]
        priority: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        category: Vec<String>,
    },
    /// Get one project by id.
    Get { id: i64 },
    /// Create a new project.
    Create {
        name: String,
        #[arg(long, default_value = "P2")]
        priority: String,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        evaluation: Option<String>,
    },
    /// Update fields on a project.
    Update {
        id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        evaluation: Option<String>,
    },
    /// Delete a project.
    Delete { id: i64 },
}

pub fn dispatch(db_path_flag: Option<&str>, sub: ProjectCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        ProjectCmd::List { priority, category } => {
            let all = projects::list(&conn)?;
            let filtered: Vec<_> = all
                .into_iter()
                .filter(|p| {
                    let pri_ok = priority.is_empty() || priority.contains(&p.priority);
                    let cat_ok = category.is_empty()
                        || p.category.as_ref().map_or(false, |c| category.contains(c));
                    pri_ok && cat_ok
                })
                .collect();
            crate::util::emit_ok(serde_json::to_value(&filtered).unwrap());
        }
        ProjectCmd::Get { id } => match projects::get(&conn, id)? {
            Some(p) => crate::util::emit_ok(serde_json::to_value(&p).unwrap()),
            None => {
                crate::util::emit_err(
                    &format!("project {id} not found"),
                    Some("try 'hearth project list'"),
                );
                std::process::exit(1);
            }
        },
        ProjectCmd::Create {
            name,
            priority,
            category,
            path,
            evaluation,
        } => {
            let p = projects::create(
                &mut conn,
                Source::Cli,
                &NewProject {
                    name: &name,
                    priority: &priority,
                    category: category.as_deref(),
                    path: path.as_deref(),
                    evaluation: evaluation.as_deref(),
                },
            )
            .context("create failed")?;
            crate::util::emit_ok(serde_json::to_value(&p).unwrap());
        }
        ProjectCmd::Update {
            id,
            name,
            priority,
            category,
            path,
            evaluation,
        } => {
            let p = projects::update(
                &mut conn,
                Source::Cli,
                id,
                &UpdateProject {
                    name: name.as_deref(),
                    priority: priority.as_deref(),
                    category: category.as_deref(),
                    path: path.as_deref(),
                    evaluation: evaluation.as_deref(),
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&p).unwrap());
        }
        ProjectCmd::Delete { id } => {
            projects::delete(&mut conn, Source::Cli, id)?;
            crate::util::emit_ok(serde_json::json!({ "deleted": id }));
        }
    }
    Ok(())
}
