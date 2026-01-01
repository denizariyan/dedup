mod common;

use common::{create_file, dedup, file_inode};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_hardlink_dry_run_no_changes() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    let inode_a_before = file_inode(&dir.path().join("a.txt"));
    let inode_b_before = file_inode(&dir.path().join("b.txt"));

    assert_ne!(inode_a_before, inode_b_before);

    dedup()
        .arg(dir.path())
        .arg("--action")
        .arg("hardlink")
        .arg("--dry-run")
        .arg("--no-progress")
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));

    let inode_a_after = file_inode(&dir.path().join("a.txt"));
    assert_eq!(inode_a_before, inode_a_after);

    let inode_b_after = file_inode(&dir.path().join("b.txt"));
    assert_eq!(inode_b_before, inode_b_after);
}

#[test]
fn test_hardlink_creates_links() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "a.txt", b"duplicate content");
    create_file(dir.path(), "b.txt", b"duplicate content");

    let inode_a_before = file_inode(&dir.path().join("a.txt"));
    let inode_b_before = file_inode(&dir.path().join("b.txt"));
    assert_ne!(inode_a_before, inode_b_before);

    dedup()
        .arg(dir.path())
        .arg("--action")
        .arg("hardlink")
        .arg("--no-progress")
        .assert()
        .success();

    let inode_a_after = file_inode(&dir.path().join("a.txt"));
    let inode_b_after = file_inode(&dir.path().join("b.txt"));
    assert_eq!(inode_a_after, inode_b_after);
}

#[test]
fn test_hardlink_skips_already_linked() {
    let dir = TempDir::new().unwrap();
    let path_a = dir.path().join("a.txt");
    let path_b = dir.path().join("b.txt");

    fs::write(&path_a, b"duplicate content").unwrap();
    fs::hard_link(&path_a, &path_b).unwrap();

    let inode_a_before = file_inode(&path_a);
    assert_eq!(inode_a_before, file_inode(&path_b));

    dedup()
        .arg(dir.path())
        .arg("--action")
        .arg("hardlink")
        .arg("--no-progress")
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked 0 files"));

    let inode_a_after = file_inode(&path_a);
    assert_eq!(inode_a_after, file_inode(&path_b));

    // No hidden delete and re-link
    assert_eq!(inode_a_before, inode_a_after);
}

#[test]
fn test_hardlink_reports_savings() {
    let dir = TempDir::new().unwrap();
    let content = b"duplicate content here";
    create_file(dir.path(), "a.txt", content);
    create_file(dir.path(), "b.txt", content);

    dedup()
        .arg(dir.path())
        .arg("--action")
        .arg("hardlink")
        .arg("--no-progress")
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked 1 files"))
        .stdout(predicate::str::contains(format!(
            "saved {} bytes",
            content.len()
        )));
}

#[test]
fn test_hardlink_multiple_groups() {
    let dir = TempDir::new().unwrap();
    create_file(dir.path(), "g1_a.txt", b"group one content");
    create_file(dir.path(), "g1_b.txt", b"group one content");

    create_file(dir.path(), "g2_a.txt", b"group two content");
    create_file(dir.path(), "g2_b.txt", b"group two content");

    dedup()
        .arg(dir.path())
        .arg("--action")
        .arg("hardlink")
        .arg("--no-progress")
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked 2 files"));

    assert_eq!(
        file_inode(&dir.path().join("g1_a.txt")),
        file_inode(&dir.path().join("g1_b.txt"))
    );
    assert_eq!(
        file_inode(&dir.path().join("g2_a.txt")),
        file_inode(&dir.path().join("g2_b.txt"))
    );

    assert_ne!(
        file_inode(&dir.path().join("g1_a.txt")),
        file_inode(&dir.path().join("g2_a.txt"))
    );
}
