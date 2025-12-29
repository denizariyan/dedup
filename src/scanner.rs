use jwalk::WalkDir;
use std::path::{Path, PathBuf};

/// Information about a file found during scanning
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
}

/// Scan a directory and return all regular files with their sizes
pub fn scan_directory(root: &Path, min_size: Option<u64>) -> Vec<FileEntry> {
    let min = min_size.unwrap_or(0);

    WalkDir::new(root)
        .skip_hidden(false)
        .follow_links(false) // Don't follow symlinks to avoid infinite loops
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let metadata = entry.metadata().ok()?;

            if !metadata.is_file() {
                return None;
            }

            let size = metadata.len();

            if size < min {
                return None;
            }

            Some(FileEntry {
                path: entry.path(),
                size,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper to create a test file with specific content
    fn create_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    #[test]
    fn test_finds_files() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "file1.txt", b"hello");
        create_file(temp.path(), "file2.txt", b"world");

        let files = scan_directory(temp.path(), None);

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_returns_correct_sizes() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "small.txt", b"hi");
        create_file(temp.path(), "large.txt", b"hello world!");

        let files = scan_directory(temp.path(), None);

        let small = files
            .iter()
            .find(|f| f.path.ends_with("small.txt"))
            .unwrap();
        let large = files
            .iter()
            .find(|f| f.path.ends_with("large.txt"))
            .unwrap();

        assert_eq!(small.size, 2);
        assert_eq!(large.size, 12);
    }

    #[test]
    fn test_scans_subdirectories() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        create_file(temp.path(), "root.txt", b"root");
        create_file(&subdir, "nested.txt", b"nested");

        let files = scan_directory(temp.path(), None);

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with("root.txt")));
        assert!(files.iter().any(|f| f.path.ends_with("nested.txt")));
    }

    #[test]
    fn test_skips_directories() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        create_file(temp.path(), "file.txt", b"content");

        let files = scan_directory(temp.path(), None);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("file.txt"));
    }

    #[test]
    fn test_min_size_filter() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "tiny.txt", b"hi"); // 2 bytes
        create_file(temp.path(), "small.txt", b"hello"); // 5 bytes
        create_file(temp.path(), "large.txt", b"hello world!"); // 12 bytes

        let files = scan_directory(temp.path(), Some(5));

        assert_eq!(files.len(), 2);
        assert!(!files.iter().any(|f| f.path.ends_with("tiny.txt")));
    }

    #[test]
    fn test_empty_directory() {
        let temp = TempDir::new().unwrap();

        let files = scan_directory(temp.path(), None);

        assert!(files.is_empty());
    }

    #[test]
    fn test_handles_symlinks() {
        let temp = TempDir::new().unwrap();
        let file_path = create_file(temp.path(), "real.txt", b"content");

        #[cfg(unix)]
        {
            let link_path = temp.path().join("link.txt");
            std::os::unix::fs::symlink(&file_path, &link_path).unwrap();
        }

        let files = scan_directory(temp.path(), None);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("real.txt"));
    }

    #[test]
    fn test_deeply_nested() {
        let temp = TempDir::new().unwrap();
        let deep = temp.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        create_file(&deep, "deep.txt", b"deep content");

        let files = scan_directory(temp.path(), None);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("deep.txt"));
    }
}
