use crate::models::Client;
use rusqlite::Connection;

const COLS: &str =
    "id, company_name, ceo, phone, fax, email, offices, project_desc, status, created_at, updated_at";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Client>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM clients ORDER BY id DESC"
    ))?;
    let rows = stmt.query_map([], |r| {
        Ok(Client {
            id: r.get(0)?,
            company_name: r.get(1)?,
            ceo: r.get(2)?,
            phone: r.get(3)?,
            fax: r.get(4)?,
            email: r.get(5)?,
            offices: r.get(6)?,
            project_desc: r.get(7)?,
            status: r.get(8)?,
            created_at: r.get(9)?,
            updated_at: r.get(10)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    #[test]
    fn list_returns_empty_on_fresh_db() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("t.db");
        let conn = init_db(&p).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 0);
    }
}
