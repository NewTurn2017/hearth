use serde_json::Value;

pub fn emit_ok(data: Value) {
    let out = serde_json::json!({ "ok": true, "data": data });
    println!("{}", out);
}

pub fn emit_err(msg: &str, hint: Option<&str>) {
    let mut e = serde_json::Map::new();
    e.insert("ok".into(), Value::Bool(false));
    e.insert("error".into(), Value::String(msg.to_string()));
    if let Some(h) = hint {
        e.insert("hint".into(), Value::String(h.to_string()));
    }
    eprintln!("{}", Value::Object(e));
}

/// Print a pretty human-readable table for list commands when `--pretty` is set.
/// `rows` is a JSON array of objects; `headers` is the ordered column list.
/// Falls back to regular `emit_ok` if `rows` is not an array.
pub fn emit_ok_pretty(rows: &Value, headers: &[&str]) {
    use comfy_table::{presets::UTF8_BORDERS_ONLY, Table};

    let arr = match rows.as_array() {
        Some(a) => a,
        None => {
            emit_ok(rows.clone());
            return;
        }
    };

    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);
    table.set_header(headers.iter().map(|h| h.to_string()).collect::<Vec<_>>());

    for row in arr {
        let cells: Vec<String> = headers
            .iter()
            .map(|h| {
                match row.get(*h) {
                    None | Some(Value::Null) => String::new(),
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => v.to_string(),
                }
            })
            .collect();
        table.add_row(cells);
    }

    println!("{table}");
}
