#![allow(dead_code)]

use assert_cmd::cargo;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub fn dedup() -> assert_cmd::Command {
    assert_cmd::Command::new(cargo::cargo_bin!("dedup"))
}

pub fn create_file(dir: &Path, name: &str, content: &[u8]) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

pub fn file_inode(path: &Path) -> u64 {
    fs::metadata(path).unwrap().ino()
}
