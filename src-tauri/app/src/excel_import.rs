use calamine::{open_workbook, Reader, Xlsx};
use rusqlite::Connection;

pub fn import_projects_from_xlsx(conn: &Connection, file_path: &str) -> Result<usize, String> {
    let mut workbook: Xlsx<_> =
        open_workbook(file_path).map_err(|e| format!("Failed to open xlsx: {}", e))?;

    let sheet_name = "Projects";
    let range = workbook
        .worksheet_range(sheet_name)
        .map_err(|e| format!("Sheet '{}' not found: {}", sheet_name, e))?;

    let mut count = 0;
    let mut sort_orders: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for (i, row) in range.rows().enumerate() {
        if i == 0 || row.len() < 3 {
            continue;
        }

        let priority_raw = row.get(0).map(|c| c.to_string()).unwrap_or_default();
        let priority = priority_raw.trim().to_string();

        if !matches!(priority.as_str(), "P0" | "P1" | "P2" | "P3" | "P4") {
            continue;
        }

        let number: Option<i64> = row.get(1).and_then(|c| {
            let s = c.to_string();
            s.trim().parse().ok()
        });
        let name = row
            .get(2)
            .map(|c| c.to_string())
            .unwrap_or_default()
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }

        let category = row.get(3).map(|c| {
            let s = c.to_string();
            s.trim()
                .split_whitespace()
                .last()
                .unwrap_or(&s)
                .to_string()
        });
        let path = row
            .get(4)
            .map(|c| c.to_string().trim().to_string())
            .filter(|s| !s.is_empty());
        let evaluation = row
            .get(5)
            .map(|c| c.to_string().trim().to_string())
            .filter(|s| !s.is_empty());

        let order = sort_orders.entry(priority.clone()).or_insert(0);
        *order += 1;

        conn.execute(
            "INSERT INTO projects (priority, number, name, category, path, evaluation, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![priority, number, name, category, path, evaluation, *order],
        )
        .map_err(|e| format!("Failed to insert project '{}': {}", name, e))?;

        count += 1;
    }

    // Import schedules from Priority sheet
    if let Ok(range) = workbook.worksheet_range("Priority") {
        let mut in_schedule_section = false;
        for row in range.rows() {
            let first = row.get(0).map(|c| c.to_string()).unwrap_or_default();
            if first.contains("출장 일정") {
                in_schedule_section = true;
                continue;
            }
            if first.contains("날짜") && in_schedule_section {
                continue;
            }
            if in_schedule_section && !first.trim().is_empty() {
                let date = first
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                if date.contains('-') && date.len() >= 10 {
                    let time = row
                        .get(1)
                        .map(|c| c.to_string().trim().to_string())
                        .filter(|s| !s.is_empty());
                    let location = row
                        .get(2)
                        .map(|c| c.to_string().trim().to_string())
                        .filter(|s| !s.is_empty());
                    let description = row
                        .get(3)
                        .map(|c| c.to_string().trim().to_string())
                        .filter(|s| !s.is_empty());
                    let notes = row
                        .get(4)
                        .map(|c| c.to_string().trim().to_string())
                        .filter(|s| !s.is_empty());

                    conn.execute(
                        "INSERT INTO schedules (date, time, location, description, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![date, time, location, description, notes],
                    )
                    .ok();
                }
            }
        }
    }

    // Import client from Priority sheet
    if let Ok(range) = workbook.worksheet_range("Priority") {
        let mut in_client_section = false;
        let mut client_data: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for row in range.rows() {
            let first = row.get(0).map(|c| c.to_string()).unwrap_or_default();
            if first.contains("의뢰 업체") {
                in_client_section = true;
                continue;
            }
            if first.contains("출장 일정") {
                in_client_section = false;
            }
            if in_client_section {
                let key = first.trim().to_string();
                let val = row
                    .get(1)
                    .map(|c| c.to_string().trim().to_string())
                    .unwrap_or_default();
                if !key.is_empty() && !val.is_empty() {
                    client_data.insert(key, val);
                }
            }
        }

        if !client_data.is_empty() {
            let offices_json = serde_json::json!({
                "대전": client_data.get("대전 사무실").cloned().unwrap_or_default(),
                "보령": client_data.get("보령 사무실").cloned().unwrap_or_default(),
            })
            .to_string();

            conn.execute(
                "INSERT INTO clients (company_name, ceo, phone, fax, email, offices, project_desc, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    client_data.get("회사명"),
                    client_data.get("대표"),
                    client_data.get("연락처"),
                    client_data.get("팩스"),
                    client_data.get("이메일"),
                    offices_json,
                    client_data.get("프로젝트"),
                    client_data.get("상태"),
                ],
            )
            .ok();
        }
    }

    Ok(count)
}
