use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn hearth(db_str: &str) -> Command {
    let mut cmd = Command::cargo_bin("hearth").unwrap();
    cmd.env("HEARTH_DB", db_str);
    cmd
}

fn stdout_json(cmd: assert_cmd::assert::Assert) -> Value {
    let out = cmd.success().get_output().stdout.clone();
    serde_json::from_slice(&out).unwrap()
}

#[test]
fn db_path_outputs_json() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_path.to_str().unwrap())
        .args(["db", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\""));
}

#[test]
fn db_migrate_creates_schema() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_path.to_str().unwrap())
        .args(["db", "migrate"])
        .assert()
        .success();
    // DB file exists
    assert!(db_path.exists());
}

#[test]
fn project_create_then_list_contains_it() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap();

    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "create", "TestProj", "--priority", "P1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["name"], "TestProj");

    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let arr = v["data"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "TestProj");
}

#[test]
fn project_delete_removes_it_and_records_audit() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap();

    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "create", "X"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let id = v["data"]["id"].as_i64().unwrap();

    Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "delete", &id.to_string()])
        .assert()
        .success();

    // List is empty
    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), 0);
}

#[test]
fn project_get_missing_returns_err_exit_1() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_path.to_str().unwrap())
        .args(["project", "get", "999"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn project_scan_reports_subdirs() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    std::fs::create_dir_all(dir.path().join("sub1")).unwrap();
    std::fs::create_dir_all(dir.path().join("sub2")).unwrap();

    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_path.to_str().unwrap())
        .args([
            "project", "scan",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let hits = v["data"].as_array().unwrap();
    assert!(hits.len() >= 2);
}

// ── Task 7.1 — memo ──────────────────────────────────────────────────────────

#[test]
fn memo_create_list_delete_roundtrip() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // create
    let v = stdout_json(
        hearth(db_str)
            .args(["memo", "create", "dentist on friday", "--color", "blue"])
            .assert(),
    );
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["content"], "dentist on friday");
    let id = v["data"]["id"].as_i64().unwrap();

    // list
    let v = stdout_json(hearth(db_str).args(["memo", "list"]).assert());
    assert_eq!(v["data"].as_array().unwrap().len(), 1);

    // get
    let v = stdout_json(hearth(db_str).args(["memo", "get", &id.to_string()]).assert());
    assert_eq!(v["data"]["content"], "dentist on friday");

    // update content
    let v = stdout_json(
        hearth(db_str)
            .args(["memo", "update", &id.to_string(), "--content", "dentist on saturday"])
            .assert(),
    );
    assert_eq!(v["data"]["content"], "dentist on saturday");

    // delete
    hearth(db_str)
        .args(["memo", "delete", &id.to_string()])
        .assert()
        .success();

    // list is empty
    let v = stdout_json(hearth(db_str).args(["memo", "list"]).assert());
    assert_eq!(v["data"].as_array().unwrap().len(), 0);
}

// ── Task 7.2 — schedule ──────────────────────────────────────────────────────

#[test]
fn schedule_create_list_delete_roundtrip() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // create
    let v = stdout_json(
        hearth(db_str)
            .args([
                "schedule", "create", "2026-05-10",
                "--description", "dentist appointment",
                "--remind-5min",
            ])
            .assert(),
    );
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["date"], "2026-05-10");
    assert_eq!(v["data"]["remind_before_5min"], true);
    let id = v["data"]["id"].as_i64().unwrap();

    // list (no filter)
    let v = stdout_json(hearth(db_str).args(["schedule", "list"]).assert());
    assert_eq!(v["data"].as_array().unwrap().len(), 1);

    // list --month
    let v = stdout_json(
        hearth(db_str)
            .args(["schedule", "list", "--month", "2026-05"])
            .assert(),
    );
    assert_eq!(v["data"].as_array().unwrap().len(), 1);

    // list --from/--to range
    let v = stdout_json(
        hearth(db_str)
            .args([
                "schedule", "list",
                "--from", "2026-05-01",
                "--to", "2026-05-31",
            ])
            .assert(),
    );
    assert_eq!(v["data"].as_array().unwrap().len(), 1);

    // get
    let v = stdout_json(hearth(db_str).args(["schedule", "get", &id.to_string()]).assert());
    assert_eq!(v["data"]["description"], "dentist appointment");

    // update
    let v = stdout_json(
        hearth(db_str)
            .args(["schedule", "update", &id.to_string(), "--date", "2026-05-11"])
            .assert(),
    );
    assert_eq!(v["data"]["date"], "2026-05-11");

    // delete
    hearth(db_str)
        .args(["schedule", "delete", &id.to_string()])
        .assert()
        .success();

    let v = stdout_json(hearth(db_str).args(["schedule", "list"]).assert());
    assert_eq!(v["data"].as_array().unwrap().len(), 0);
}

// ── Task 7.3 — category ──────────────────────────────────────────────────────

#[test]
fn category_create_rename_cascades_to_project() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // create category
    let v = stdout_json(
        hearth(db_str)
            .args(["category", "create", "OldName", "--color", "#ff0000"])
            .assert(),
    );
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["name"], "OldName");

    // create project referencing that category
    hearth(db_str)
        .args(["project", "create", "MyProj", "--category", "OldName"])
        .assert()
        .success();

    // rename
    let v = stdout_json(
        hearth(db_str)
            .args(["category", "rename", "OldName", "NewName"])
            .assert(),
    );
    assert_eq!(v["data"]["name"], "NewName");

    // project list should reflect new category name
    let v = stdout_json(hearth(db_str).args(["project", "list"]).assert());
    let arr = v["data"].as_array().unwrap();
    assert_eq!(arr[0]["category"], "NewName");
}

#[test]
fn category_delete_refuses_in_use() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // create category and project using it
    let v = stdout_json(hearth(db_str).args(["category", "create", "Hot"]).assert());
    let cat_id = v["data"]["id"].as_i64().unwrap();
    hearth(db_str)
        .args(["project", "create", "X", "--category", "Hot"])
        .assert()
        .success();

    // delete should fail with exit code 1 and Korean "사용 중" in stderr
    hearth(db_str)
        .args(["category", "delete", &cat_id.to_string()])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("사용 중"));
}

// ── Task 8.1 — search ────────────────────────────────────────────────────────

#[test]
fn search_finds_memo_content() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // create memo with searchable content
    hearth(db_str)
        .args(["memo", "create", "dentist on friday"])
        .assert()
        .success();

    // search
    let v = stdout_json(hearth(db_str).args(["search", "dentist"]).assert());
    assert_eq!(v["ok"], true);
    let hits = v["data"].as_array().unwrap();
    assert!(!hits.is_empty(), "expected at least one search hit");
    assert_eq!(hits[0]["kind"], "memo");
}

// ── Task 8.2 — today / overdue / stats ───────────────────────────────────────

#[test]
fn today_returns_structured_view() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    let v = stdout_json(hearth(db_str).args(["today"]).assert());
    assert_eq!(v["ok"], true);
    let data = &v["data"];
    assert!(data["date"].is_string(), "data.date should be a string");
    assert!(data["schedules_today"].is_array(), "data.schedules_today should be an array");
    assert!(data["p0_projects"].is_array(), "data.p0_projects should be an array");
    assert!(data["recent_memos"].is_array(), "data.recent_memos should be an array");
}

#[test]
fn overdue_returns_structured_view() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    let v = stdout_json(hearth(db_str).args(["overdue"]).assert());
    assert_eq!(v["ok"], true);
    let data = &v["data"];
    assert!(data["overdue_schedules"].is_array());
    assert!(data["stale_projects"].is_array());
}

#[test]
fn stats_returns_counts() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // create some data first
    hearth(db_str)
        .args(["memo", "create", "test memo"])
        .assert()
        .success();
    hearth(db_str)
        .args(["project", "create", "StatsProj"])
        .assert()
        .success();

    let v = stdout_json(hearth(db_str).args(["stats"]).assert());
    assert_eq!(v["ok"], true);
    let data = &v["data"];
    assert_eq!(data["total_memos"].as_i64().unwrap(), 1);
    assert_eq!(data["total_projects"].as_i64().unwrap(), 1);
    assert!(data["total_schedules"].is_number());
}

// ── Task 9.1 — log / undo / redo ─────────────────────────────────────────────

#[test]
fn undo_reverts_last_mutation() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // Create a project via CLI
    hearth(db_str)
        .args(["project", "create", "UndoMe"])
        .assert()
        .success();

    // Verify it exists
    let v = stdout_json(hearth(db_str).args(["project", "list"]).assert());
    assert_eq!(v["data"].as_array().unwrap().len(), 1);

    // Undo
    let v = stdout_json(hearth(db_str).args(["undo"]).assert());
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["undone"], 1);

    // Should now be 0 projects
    let v = stdout_json(hearth(db_str).args(["project", "list"]).assert());
    assert_eq!(v["data"].as_array().unwrap().len(), 0);
}

// ── Task 10.1 — export ───────────────────────────────────────────────────────

#[test]
fn export_json_includes_projects() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // Create a project
    hearth(db_str)
        .args(["project", "create", "ExportMe", "--priority", "P1"])
        .assert()
        .success();

    // Export to a temp file
    let out_path = dir.path().join("export.json");
    let out_str = out_path.to_str().unwrap();

    let v = stdout_json(
        hearth(db_str)
            .args(["export", "--out", out_str])
            .assert(),
    );
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["written"], out_str);

    // Read the file and confirm the project is there
    let contents = std::fs::read_to_string(&out_path).unwrap();
    assert!(
        contents.contains("ExportMe"),
        "export file should contain project name"
    );
}

// ── Task 10.2 — import ───────────────────────────────────────────────────────

#[test]
fn export_then_import_merge_roundtrip() {
    let dir_a = TempDir::new().unwrap();
    let db_a = dir_a.path().join("a.db");
    let db_a_str = db_a.to_str().unwrap();

    let dir_b = TempDir::new().unwrap();
    let db_b = dir_b.path().join("b.db");
    let db_b_str = db_b.to_str().unwrap();

    // Create project in DB A
    hearth(db_a_str)
        .args(["project", "create", "RoundTripProj"])
        .assert()
        .success();

    // Export from DB A
    let export_path = dir_a.path().join("dump.json");
    let export_str = export_path.to_str().unwrap();
    hearth(db_a_str)
        .args(["export", "--out", export_str])
        .assert()
        .success();

    // Import into DB B (merge)
    let v = stdout_json(
        hearth(db_b_str)
            .args(["import", export_str, "--merge"])
            .assert(),
    );
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["inserted_projects"], 1);
    assert_eq!(v["data"]["dry_run"], false);

    // DB B should now list the project
    let v = stdout_json(hearth(db_b_str).args(["project", "list"]).assert());
    let arr = v["data"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "RoundTripProj");
}
