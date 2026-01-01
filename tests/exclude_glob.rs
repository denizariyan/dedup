mod common;

use common::{create_file, dedup, get_all_filenames};
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_exclude_extension_finds_duplicates_in_remaining() {
    let dir = TempDir::new().unwrap();

    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    create_file(dir.path(), "a.log", b"log duplicate");
    create_file(dir.path(), "b.log", b"log duplicate");

    let output = dedup()
        .arg(dir.path())
        .arg("--exclude")
        .arg("*.log")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["stats"]["total_files"], 2);
    assert_eq!(json["groups"].as_array().unwrap().len(), 1);

    let filenames = get_all_filenames(&json);
    assert!(filenames.contains(&"a.txt".to_string()));
    assert!(filenames.contains(&"b.txt".to_string()));
    assert!(!filenames.iter().any(|f| f.ends_with(".log")));
}

#[test]
fn test_exclude_directory_skips_entire_tree() {
    let dir = TempDir::new().unwrap();

    create_file(dir.path(), "root.txt", b"unique root");

    create_file(dir.path(), "node_modules/pkg/a.js", b"module dup");
    create_file(dir.path(), "node_modules/pkg/b.js", b"module dup");

    let output = dedup()
        .arg(dir.path())
        .arg("-e")
        .arg("node_modules")
        .arg("-f")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["stats"]["total_files"], 1);
    assert!(json["groups"].as_array().unwrap().is_empty());
}

#[test]
fn test_multiple_exclude_patterns() {
    let dir = TempDir::new().unwrap();

    create_file(dir.path(), "keep1.txt", b"keep this");
    create_file(dir.path(), "keep2.txt", b"keep this");

    create_file(dir.path(), "skip.log", b"skip log");
    create_file(dir.path(), "skip.bak", b"skip bak");
    create_file(dir.path(), "build/output.js", b"build output");

    let output = dedup()
        .arg(dir.path())
        .arg("-e")
        .arg("*.log")
        .arg("-e")
        .arg("*.bak")
        .arg("-e")
        .arg("build")
        .arg("-f")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["stats"]["total_files"], 2);
    assert_eq!(json["groups"].as_array().unwrap().len(), 1);

    let filenames = get_all_filenames(&json);
    assert_eq!(filenames.len(), 2);
    assert!(filenames.contains(&"keep1.txt".to_string()));
    assert!(filenames.contains(&"keep2.txt".to_string()));
}

#[test]
fn test_exclude_file_reads_patterns() {
    let dir = TempDir::new().unwrap();
    let exclude_dir = TempDir::new().unwrap();

    let exclude_file = exclude_dir.path().join(".dedupignore");
    let mut f = std::fs::File::create(&exclude_file).unwrap();
    writeln!(f, "# Ignore logs").unwrap();
    writeln!(f, "*.log").unwrap();
    #[allow(clippy::writeln_empty_string)] // Testing empty lines
    writeln!(f, "").unwrap();
    writeln!(f, " ").unwrap();
    writeln!(f, "node_modules").unwrap();

    create_file(dir.path(), "keep1.txt", b"keep this");
    create_file(dir.path(), "keep2.txt", b"keep this");
    create_file(dir.path(), "skip.log", b"skip log");
    create_file(dir.path(), "node_modules/lib.js", b"module");

    let output = dedup()
        .arg(dir.path())
        .arg("--exclude-file")
        .arg(&exclude_file)
        .arg("-f")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["stats"]["total_files"], 2);

    let filenames = get_all_filenames(&json);
    assert!(filenames.contains(&"keep1.txt".to_string()));
    assert!(filenames.contains(&"keep2.txt".to_string()));
}

#[test]
fn test_exclude_file_combined_with_exclude_flag() {
    let dir = TempDir::new().unwrap();
    let exclude_dir = TempDir::new().unwrap();

    let exclude_file = exclude_dir.path().join(".dedupignore");
    let mut f = std::fs::File::create(&exclude_file).unwrap();
    writeln!(f, "# From exclude file:").unwrap();
    writeln!(f, "*.log").unwrap();
    writeln!(f, "cache").unwrap();

    create_file(dir.path(), "keep1.txt", b"keep this");
    create_file(dir.path(), "keep2.txt", b"keep this");
    create_file(dir.path(), "skip.log", b"excluded by file"); // excluded by --exclude-file
    create_file(dir.path(), "skip.bak", b"excluded by flag"); // excluded by -e flag
    create_file(dir.path(), "skip.tmp", b"excluded by flag"); // excluded by -e flag
    create_file(dir.path(), "cache/data.bin", b"excluded by file"); // excluded by --exclude-file

    // Combine --exclude-file with multiple -e flags
    let output = dedup()
        .arg(dir.path())
        .arg("--exclude-file")
        .arg(&exclude_file)
        .arg("-e")
        .arg("*.bak")
        .arg("-e")
        .arg("*.tmp")
        .arg("-f")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    // Only keep1.txt and keep2.txt should remain (2 files)
    // - skip.log excluded by *.log from file
    // - skip.bak excluded by *.bak from -e flag
    // - skip.tmp excluded by *.tmp from -e flag
    // - cache/data.bin excluded by cache from file
    assert_eq!(json["stats"]["total_files"], 2);

    let filenames = get_all_filenames(&json);
    assert_eq!(filenames.len(), 2);
    assert!(filenames.contains(&"keep1.txt".to_string()));
    assert!(filenames.contains(&"keep2.txt".to_string()));
}

#[test]
fn test_exclude_file_with_directories_and_globs() {
    let dir = TempDir::new().unwrap();
    let exclude_dir = TempDir::new().unwrap();

    // Create exclude file with various pattern types
    let exclude_file = exclude_dir.path().join("excludes.txt");
    let mut f = std::fs::File::create(&exclude_file).unwrap();
    writeln!(f, "# Directories").unwrap();
    writeln!(f, "node_modules").unwrap();
    writeln!(f, ".git").unwrap();
    #[allow(clippy::writeln_empty_string)] // Testing empty lines
    writeln!(f, "").unwrap();
    writeln!(f, "# File patterns").unwrap();
    writeln!(f, "*.pyc").unwrap();
    writeln!(f, "**/*.o").unwrap();

    // Create directory structure
    create_file(dir.path(), "src/main.rs", b"keep");
    create_file(dir.path(), "src/lib.rs", b"keep");
    create_file(dir.path(), "node_modules/pkg/index.js", b"skip");
    create_file(dir.path(), ".git/objects/abc", b"skip");
    create_file(dir.path(), "cache.pyc", b"skip");
    create_file(dir.path(), "build/main.o", b"skip");

    let output = dedup()
        .arg(dir.path())
        .arg("--exclude-file")
        .arg(&exclude_file)
        .arg("-f")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["stats"]["total_files"], 2);
    let filenames = get_all_filenames(&json);
    assert!(filenames.contains(&"main.rs".to_string()));
    assert!(filenames.contains(&"lib.rs".to_string()));
}
