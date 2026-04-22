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
