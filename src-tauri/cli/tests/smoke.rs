use assert_cmd::Command;
use predicates::prelude::*;
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
