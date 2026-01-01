mod common;

use common::{create_file, dedup};
use tempfile::TempDir;

#[test]
fn test_exit_zero_no_duplicates_default_settings() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"unique content a");
    create_file(dir.path(), "b.txt", b"unique content b");

    dedup()
        .arg(dir.path())
        .arg("--no-progress")
        .assert()
        .success();
}

#[test]
fn test_exit_zero_with_duplicates_default_settings() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    dedup()
        .arg(dir.path())
        .arg("--no-progress")
        .assert()
        .success();
}

#[test]
fn test_report_exit_code_zero_no_dups() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"unique content a");
    create_file(dir.path(), "b.txt", b"unique content b");

    dedup()
        .arg(dir.path())
        .arg("--action")
        .arg("report-exit-code")
        .arg("--no-progress")
        .assert()
        .success();
}

#[test]
fn test_report_exit_code_one_with_dups() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    dedup()
        .arg(dir.path())
        .arg("--action")
        .arg("report-exit-code")
        .arg("--no-progress")
        .assert()
        .code(1);
}
