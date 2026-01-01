mod common;

use common::{create_file, dedup};
use tempfile::TempDir;

#[test]
fn test_finds_identical_files() {
    let dir = TempDir::new().unwrap();
    let content = b"identical content";
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
    assert_eq!(json["groups"].as_array().unwrap().len(), 1);
    assert_eq!(json["stats"]["duplicate_files"], 2);
    assert_eq!(json["stats"]["wasted_bytes"], content.len() as u64);
}

#[test]
fn test_handles_different_files() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"content a");
    create_file(dir.path(), "b.txt", b"content b");

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
fn test_large_files_detected() {
    let dir = TempDir::new().unwrap();
    // Create files larger than 8KB partial hash threshold
    let large_content: Vec<u8> = vec![b'x'; 16 * 1024];
    create_file(dir.path(), "large_a.bin", &large_content);
    create_file(dir.path(), "large_b.bin", &large_content);

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
}

#[test]
fn test_multiple_duplicate_groups() {
    let dir = TempDir::new().unwrap();
    let content1 = b"group one content";
    let content2 = b"group two content";
    create_file(dir.path(), "group1_a.txt", content1);
    create_file(dir.path(), "group1_b.txt", content1);

    create_file(dir.path(), "group2_a.txt", content2);
    create_file(dir.path(), "group2_b.txt", content2);

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
    let groups = json["groups"].as_array().unwrap();

    assert_eq!(groups.len(), 2);
    assert_eq!(json["stats"]["duplicate_files"], 4);

    let expected_wasted = content1.len() + content2.len();
    assert_eq!(json["stats"]["wasted_bytes"], expected_wasted as u64);

    for group in groups {
        assert_eq!(group["files"].as_array().unwrap().len(), 2);
    }

    let sizes: Vec<u64> = groups.iter().map(|g| g["size"].as_u64().unwrap()).collect();
    assert!(sizes.contains(&(content1.len() as u64)));
    assert!(sizes.contains(&(content2.len() as u64)));
}

#[test]
fn test_three_way_duplicates() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"triple duplicate");
    create_file(dir.path(), "b.txt", b"triple duplicate");
    create_file(dir.path(), "c.txt", b"triple duplicate");

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
    // Wasted space should be 2 * file_size (3 files, only 1 needed)
    let wasted = json["stats"]["wasted_bytes"].as_u64().unwrap();
    assert_eq!(wasted, 2 * b"triple duplicate".len() as u64);
}
