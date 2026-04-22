use anyhow::Result;
use clap::Subcommand;
use hearth_core::categories::{self, CategoryError, UpdateCategory};

#[derive(Subcommand)]
pub enum CategoryCmd {
    /// List all categories.
    List,
    /// Create a new category.
    Create {
        name: String,
        #[arg(long)]
        color: Option<String>,
    },
    /// Rename a category (cascades to all projects referencing it).
    Rename {
        /// Current category name.
        old_name: String,
        /// New category name.
        new_name: String,
    },
    /// Update category color or sort_order.
    Update {
        id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        sort_order: Option<i64>,
    },
    /// Delete a category by id. Fails if any project references it.
    Delete { id: i64 },
}

pub fn dispatch(db_path_flag: Option<&str>, sub: CategoryCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        CategoryCmd::List => {
            let all = categories::list(&conn)?;
            crate::util::emit_ok(serde_json::to_value(&all).unwrap());
        }
        CategoryCmd::Create { name, color } => {
            match categories::create(&conn, &name, color.as_deref()) {
                Ok(c) => crate::util::emit_ok(serde_json::to_value(&c).unwrap()),
                Err(e) => {
                    crate::util::emit_err(&e.to_string(), None);
                    std::process::exit(1);
                }
            }
        }
        CategoryCmd::Rename { old_name, new_name } => {
            // Resolve old_name -> id
            let all = categories::list(&conn)?;
            let cat = all.iter().find(|c| c.name == old_name);
            match cat {
                None => {
                    crate::util::emit_err(
                        &format!("카테고리를 찾을 수 없음: '{old_name}'"),
                        Some("try 'hearth category list'"),
                    );
                    std::process::exit(1);
                }
                Some(c) => {
                    let id = c.id;
                    match categories::update(
                        &mut conn,
                        id,
                        &UpdateCategory {
                            name: Some(&new_name),
                            color: None,
                            sort_order: None,
                        },
                    ) {
                        Ok(updated) => crate::util::emit_ok(serde_json::to_value(&updated).unwrap()),
                        Err(e) => {
                            crate::util::emit_err(&e.to_string(), None);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        CategoryCmd::Update { id, name, color, sort_order } => {
            match categories::update(
                &mut conn,
                id,
                &UpdateCategory {
                    name: name.as_deref(),
                    color: color.as_deref(),
                    sort_order,
                },
            ) {
                Ok(c) => crate::util::emit_ok(serde_json::to_value(&c).unwrap()),
                Err(e) => {
                    crate::util::emit_err(&e.to_string(), None);
                    std::process::exit(1);
                }
            }
        }
        CategoryCmd::Delete { id } => {
            match categories::delete(&conn, id) {
                Ok(()) => crate::util::emit_ok(serde_json::json!({ "deleted": id })),
                Err(CategoryError::InUse { name, count }) => {
                    crate::util::emit_err(
                        &format!("카테고리 사용 중 ({count}개 프로젝트): {name}"),
                        Some("먼저 프로젝트의 카테고리를 변경하세요"),
                    );
                    std::process::exit(1);
                }
                Err(e) => {
                    crate::util::emit_err(&e.to_string(), None);
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}
