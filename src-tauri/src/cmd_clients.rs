use crate::models::Client;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_clients(state: State<'_, AppState>) -> Result<Vec<Client>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, company_name, ceo, phone, fax, email, offices, project_desc, status, created_at, updated_at FROM clients ORDER BY id",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Client {
                id: row.get(0)?,
                company_name: row.get(1)?,
                ceo: row.get(2)?,
                phone: row.get(3)?,
                fax: row.get(4)?,
                email: row.get(5)?,
                offices: row.get(6)?,
                project_desc: row.get(7)?,
                status: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}
