use std::fs;
use std::io;
use std::path::PathBuf;

use colored::Colorize;

use crate::output::DuplicateGroup;

/// Result of a hardlink operation
#[derive(Debug, Default)]
pub struct ActionResult {
    /// Number of files replaced with hardlinks
    pub files_linked: usize,
    /// Number of bytes saved
    pub bytes_saved: u64,
    /// Errors encountered (path, error message)
    pub errors: Vec<(PathBuf, String)>,
}

/// Select which file to keep as the "original" in a duplicate group.
fn select_original(files: &[PathBuf]) -> &PathBuf {
    files
        .iter()
        .min_by_key(|p| p.as_os_str().len())
        .expect("group must have at least one file")
}

/// Replace duplicate files with hardlinks to the original.
///
/// For each group:
/// 1. Select one file as the "original". (shortest path)
/// 2. For each duplicate: remove it and create a hardlink to original
///
/// If `dry_run` is true, only prints what would happen without modifying files.
pub fn hardlink_duplicates(
    groups: &[DuplicateGroup],
    dry_run: bool,
    print_verbose_logs: bool,
) -> ActionResult {
    let mut result = ActionResult::default();

    for group in groups {
        if group.files.len() < 2 {
            continue;
        }

        let original = select_original(&group.files);

        for path in &group.files {
            if path == original {
                continue;
            }

            if print_verbose_logs {
                println!(
                    "{} {} -> {}",
                    "[dry-run]".yellow(),
                    path.display(),
                    original.display()
                );
            }
            result.files_linked += 1;
            result.bytes_saved += group.size;

            if !dry_run {
                // TODO: better handling of files on different filesystems when hardlinking
                match replace_with_hardlink(path, original) {
                    Ok(()) => {
                        if print_verbose_logs {
                            println!(
                                "{} {} -> {}",
                                "[linked]".green(),
                                path.display(),
                                original.display()
                            );
                        }
                    }
                    Err(e) => {
                        result.errors.push((path.clone(), e.to_string()));
                    }
                }
            }
        }
    }

    result
}

/// Replace a file with a hardlink to another file.
/// Removes the original file first, then creates the hardlink.
fn replace_with_hardlink(path: &PathBuf, original: &PathBuf) -> io::Result<()> {
    fs::remove_file(path)?;
    fs::hard_link(original, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_file(dir: &std::path::Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    #[test]
    fn test_select_original_shortest_path() {
        let files = vec![
            PathBuf::from("/a/b/c/file.txt"),
            PathBuf::from("/a/file.txt"),
            PathBuf::from("/a/b/file.txt"),
        ];

        let original = select_original(&files);
        assert_eq!(original, &PathBuf::from("/a/file.txt"));
    }

    #[test]
    fn test_select_original_single_file() {
        let files = vec![PathBuf::from("/only/file.txt")];
        let original = select_original(&files);
        assert_eq!(original, &PathBuf::from("/only/file.txt"));
    }

    #[test]
    fn test_hardlink_dry_run() {
        use std::os::unix::fs::MetadataExt;

        let temp = TempDir::new().unwrap();
        let content = b"duplicate content";

        let path1 = create_file(temp.path(), "file1.txt", content);
        let path2 = create_file(temp.path(), "file2.txt", content);

        let groups = vec![DuplicateGroup {
            size: content.len() as u64,
            files: vec![path1.clone(), path2.clone()],
        }];

        let result = hardlink_duplicates(&groups, true, false);

        assert_eq!(result.files_linked, 1);
        assert_eq!(result.bytes_saved, content.len() as u64);
        assert!(result.errors.is_empty());

        // Files should still be separate (dry run)
        let meta1 = fs::metadata(&path1).unwrap();
        let meta2 = fs::metadata(&path2).unwrap();
        assert_ne!(meta1.ino(), meta2.ino());
    }

    #[test]
    fn test_hardlink_actual() {
        let temp = TempDir::new().unwrap();
        let content = b"duplicate content";

        let path1 = create_file(temp.path(), "file1.txt", content);
        let path2 = create_file(temp.path(), "file2.txt", content);

        let groups = vec![DuplicateGroup {
            size: content.len() as u64,
            files: vec![path1.clone(), path2.clone()],
        }];

        let result = hardlink_duplicates(&groups, false, false);

        assert_eq!(result.files_linked, 1);
        assert_eq!(result.bytes_saved, content.len() as u64);
        assert!(result.errors.is_empty());

        // Files should now be hardlinks (same inode)
        use std::os::unix::fs::MetadataExt;
        let meta1 = fs::metadata(&path1).unwrap();
        let meta2 = fs::metadata(&path2).unwrap();
        assert_eq!(meta1.ino(), meta2.ino());

        // Content should still be readable
        let content1 = fs::read(&path1).unwrap();
        let content2 = fs::read(&path2).unwrap();
        assert_eq!(content1, content2);
        assert_eq!(content1, content);
    }

    #[test]
    fn test_hardlink_multiple_duplicates() {
        let temp = TempDir::new().unwrap();
        let content = b"same content";

        let path1 = create_file(temp.path(), "a.txt", content);
        let path2 = create_file(temp.path(), "b.txt", content);
        let path3 = create_file(temp.path(), "c.txt", content);

        let groups = vec![DuplicateGroup {
            size: content.len() as u64,
            files: vec![path1.clone(), path2.clone(), path3.clone()],
        }];

        let result = hardlink_duplicates(&groups, false, false);

        assert_eq!(result.files_linked, 2); // 2 files linked to original
        assert_eq!(result.bytes_saved, (content.len() * 2) as u64);

        // All should share same inode
        use std::os::unix::fs::MetadataExt;
        let ino1 = fs::metadata(&path1).unwrap().ino();
        let ino2 = fs::metadata(&path2).unwrap().ino();
        let ino3 = fs::metadata(&path3).unwrap().ino();
        assert_eq!(ino1, ino2);
        assert_eq!(ino2, ino3);
    }
}
