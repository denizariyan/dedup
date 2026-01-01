mod common;

use common::{create_file, dedup};
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_human_output_shows_report() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    dedup()
        .arg(dir.path())
        .arg("--no-progress")
        .assert()
        .success()
        .stdout(predicate::str::contains("Duplicate Report"));
}

#[test]
fn test_json_output_valid() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    let output = dedup()
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("Invalid JSON output");
    assert!(json.is_object());
}

#[test]
fn test_json_output_structure() {
    let dir = TempDir::new().unwrap();
    let content = b"duplicate content";
    create_file(dir.path(), "a.txt", content);
    create_file(dir.path(), "b.txt", content);

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

    assert!(
        json.get("stats").is_some(),
        "JSON should have 'stats' field"
    );
    assert!(
        json.get("groups").is_some(),
        "JSON should have 'groups' field"
    );

    assert!(json["stats"]["total_files"].is_number());
    assert!(json["stats"]["duplicate_files"].is_number());
    assert!(json["stats"]["wasted_bytes"].is_number());

    let groups = json["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);

    let group = &groups[0];
    assert!(
        group.get("size").is_some(),
        "Group should have 'size' field"
    );
    assert!(
        group.get("files").is_some(),
        "Group should have 'files' field"
    );
    assert_eq!(group["size"], content.len() as u64);

    let files = group["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    for file in files {
        assert!(file.is_string(), "Each file should be a string path");
    }

    let filenames: Vec<&str> = files
        .iter()
        .map(|f| f.as_str().unwrap().rsplit('/').next().unwrap())
        .collect();
    assert!(filenames.contains(&"a.txt"));
    assert!(filenames.contains(&"b.txt"));
}

#[test]
fn test_quiet_output_empty_with_duplicates() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    dedup()
        .arg(dir.path())
        .arg("--format")
        .arg("quiet")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_quiet_output_empty_no_duplicates() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"unique content a");
    create_file(dir.path(), "b.txt", b"unique content b");

    dedup()
        .arg(dir.path())
        .arg("--format")
        .arg("quiet")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_verbose_shows_file_paths() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    dedup()
        .arg(dir.path())
        .arg("--verbose")
        .arg("--no-progress")
        .assert()
        .success()
        .stdout(predicate::str::contains("a.txt").or(predicate::str::contains("b.txt")));
}
