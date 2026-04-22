use anyhow::Result;
use hearth_core::views;

pub fn today(db_path_flag: Option<&str>) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let conn = crate::db::open(&p)?;
    let view = views::today(&conn)?;
    crate::util::emit_ok(serde_json::to_value(&view).unwrap());
    Ok(())
}

pub fn overdue(db_path_flag: Option<&str>) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let conn = crate::db::open(&p)?;
    let view = views::overdue(&conn)?;
    crate::util::emit_ok(serde_json::to_value(&view).unwrap());
    Ok(())
}

pub fn stats(db_path_flag: Option<&str>) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let conn = crate::db::open(&p)?;
    let view = views::stats(&conn)?;
    crate::util::emit_ok(serde_json::to_value(&view).unwrap());
    Ok(())
}
