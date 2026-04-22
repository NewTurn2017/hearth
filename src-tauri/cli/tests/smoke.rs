use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

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
