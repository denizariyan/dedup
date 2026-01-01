mod common;

use common::{create_file, dedup};
use tempfile::TempDir;

#[test]
fn test_empty_directory() {
    let dir = TempDir::new().unwrap();

    let output = dedup()
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["groups"].as_array().unwrap().len(), 0);
    assert_eq!(json["stats"]["duplicate_files"], 0);
}

#[test]
fn test_nested_directories() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "top.txt", b"nested duplicate");
    create_file(dir.path(), "sub/nested.txt", b"nested duplicate");
    create_file(dir.path(), "sub/deep/deeper.txt", b"nested duplicate");

    let output = dedup()
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["groups"].as_array().unwrap().len(), 1);
    assert_eq!(json["stats"]["duplicate_files"], 3);
}

#[test]
fn test_single_file() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "only.txt", b"only file here");

    let output = dedup()
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["groups"].as_array().unwrap().len(), 0);
    assert_eq!(json["stats"]["duplicate_files"], 0);
}
