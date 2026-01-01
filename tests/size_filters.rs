mod common;

use common::{create_file, dedup};
use tempfile::TempDir;

fn get_all_filenames(json: &serde_json::Value) -> Vec<String> {
    json["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["files"].as_array().unwrap())
        .map(|f| f.as_str().unwrap().rsplit('/').next().unwrap().to_string())
        .collect()
}

#[test]
fn test_min_size_filter() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "small_a.txt", b"small"); // 5 bytes
    create_file(dir.path(), "small_b.txt", b"small");
    create_file(dir.path(), "large_a.txt", b"larger content here"); // 19 bytes
    create_file(dir.path(), "large_b.txt", b"larger content here");

    let output = dedup()
        .arg(dir.path())
        .arg("--min-size")
        .arg("10")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["groups"].as_array().unwrap().len(), 1);
    assert_eq!(json["stats"]["duplicate_files"], 2);

    // Verify only large files are found
    let filenames = get_all_filenames(&json);
    assert_eq!(filenames.len(), 2);
    assert!(filenames.contains(&"large_a.txt".to_string()));
    assert!(filenames.contains(&"large_b.txt".to_string()));
}

#[test]
fn test_max_size_filter() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "small_a.txt", b"small"); // 5 bytes
    create_file(dir.path(), "small_b.txt", b"small");
    create_file(dir.path(), "large_a.txt", b"larger content here"); // 19 bytes
    create_file(dir.path(), "large_b.txt", b"larger content here");

    let output = dedup()
        .arg(dir.path())
        .arg("--max-size")
        .arg("10")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["groups"].as_array().unwrap().len(), 1);
    assert_eq!(json["stats"]["duplicate_files"], 2);

    let filenames = get_all_filenames(&json);
    assert_eq!(filenames.len(), 2);
    assert!(filenames.contains(&"small_a.txt".to_string()));
    assert!(filenames.contains(&"small_b.txt".to_string()));
}

#[test]
fn test_size_filter_combined() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "tiny_a.txt", b"tiny"); // 4 bytes
    create_file(dir.path(), "tiny_b.txt", b"tiny");
    create_file(dir.path(), "medium_a.txt", b"medium content"); // 14 bytes
    create_file(dir.path(), "medium_b.txt", b"medium content");
    create_file(dir.path(), "huge_a.txt", b"this is a huge file content"); // 27 bytes
    create_file(dir.path(), "huge_b.txt", b"this is a huge file content");

    let output = dedup()
        .arg(dir.path())
        .arg("--min-size")
        .arg("5")
        .arg("--max-size")
        .arg("20")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["groups"].as_array().unwrap().len(), 1);
    assert_eq!(json["stats"]["duplicate_files"], 2);

    let filenames = get_all_filenames(&json);
    assert_eq!(filenames.len(), 2);
    assert!(filenames.contains(&"medium_a.txt".to_string()));
    assert!(filenames.contains(&"medium_b.txt".to_string()));
}
