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

pub fn get_all_filenames(json: &serde_json::Value) -> Vec<String> {
    json["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["files"].as_array().unwrap())
        .map(|f| f.as_str().unwrap().rsplit('/').next().unwrap().to_string())
        .collect()
}
