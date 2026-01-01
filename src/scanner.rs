use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use jwalk::WalkDirGeneric;
use std::path::{Path, PathBuf};

/// Information about a file found during scanning
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
}

/// Build a GlobSet from a list of glob patterns
fn build_glob_set(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match GlobBuilder::new(pattern).literal_separator(true).build() {
            Ok(g) => {
                builder.add(g);
            }
            Err(e) => eprintln!("Warning: invalid exclude pattern '{}': {}", pattern, e),
        }
    }
    builder.build().ok()
}

/// Check if a path matches any patterns in the glob set
fn matches_glob(path: &Path, glob_set: &GlobSet) -> bool {
    // Try matching the full path first (for patterns like **/node_modules)
    // Then try matching just the file/directory name (for patterns like *.log)
    if glob_set.is_match(path) {
        return true;
    }

    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| glob_set.is_match(name))
}

/// Scan a directory and return all regular files with their sizes
pub fn scan_directory(
    root: &Path,
    min_size: Option<u64>,
    max_size: Option<u64>,
    exclude_patterns: &[String],
    include_patterns: &[String],
) -> Vec<FileEntry> {
    let min = min_size.unwrap_or(0);
    let max = max_size.unwrap_or(u64::MAX);
    let exclude_set = build_glob_set(exclude_patterns);
    let include_set = build_glob_set(include_patterns);

    WalkDirGeneric::<((), ())>::new(root)
        .skip_hidden(false)
        .follow_links(false)
        .process_read_dir(move |_depth, _path, _state, children| {
            children.retain(|entry| {
                let Ok(e) = entry.as_ref() else {
                    return true; // keep errors to handle later
                };

                let path = e.path();

                if let Some(ref glob_set) = exclude_set
                    && matches_glob(&path, glob_set)
                {
                    return false;
                }

                let is_file = e.file_type().is_file();
                // Include patterns only apply to files, we still have to traverse directories
                if let Some(ref glob_set) = include_set
                    && is_file
                    && !matches_glob(&path, glob_set)
                {
                    return false;
                }

                true
            });
        })
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let metadata = entry.metadata().ok()?;

            if !metadata.is_file() {
                return None;
            }

            let size = metadata.len();

            // Empty files are commonly used as placeholders, they are all "duplicates" but not interesting
            if size == 0 {
                return None;
            }

            if size < min || size > max {
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

        let files = scan_directory(temp.path(), None, None, &[], &[]);

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_returns_correct_sizes() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "small.txt", b"hi");
        create_file(temp.path(), "large.txt", b"hello world!");

        let files = scan_directory(temp.path(), None, None, &[], &[]);

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

        let files = scan_directory(temp.path(), None, None, &[], &[]);

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

        let files = scan_directory(temp.path(), None, None, &[], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("file.txt"));
    }

    #[test]
    fn test_min_size_filter() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "tiny.txt", b"hi"); // 2 bytes
        create_file(temp.path(), "small.txt", b"hello"); // 5 bytes
        create_file(temp.path(), "large.txt", b"hello world!"); // 12 bytes

        let files = scan_directory(temp.path(), Some(5), None, &[], &[]);

        assert_eq!(files.len(), 2);
        assert!(!files.iter().any(|f| f.path.ends_with("tiny.txt")));
    }

    #[test]
    fn test_max_size_filter() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "tiny.txt", b"hi"); // 2 bytes
        create_file(temp.path(), "small.txt", b"hello"); // 5 bytes
        create_file(temp.path(), "large.txt", b"hello world!"); // 12 bytes

        let files = scan_directory(temp.path(), None, Some(5), &[], &[]);

        assert_eq!(files.len(), 2);
        assert!(!files.iter().any(|f| f.path.ends_with("large.txt")));
    }

    #[test]
    fn test_min_max_size_filter() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "tiny.txt", b"hi"); // 2 bytes
        create_file(temp.path(), "small.txt", b"hello"); // 5 bytes
        create_file(temp.path(), "large.txt", b"hello world!"); // 12 bytes

        let files = scan_directory(temp.path(), Some(3), Some(10), &[], &[]);

        assert_eq!(files.len(), 1);
        assert!(files.iter().any(|f| f.path.ends_with("small.txt")));
    }

    #[test]
    fn test_empty_directory() {
        let temp = TempDir::new().unwrap();

        let files = scan_directory(temp.path(), None, None, &[], &[]);

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

        let files = scan_directory(temp.path(), None, None, &[], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("real.txt"));
    }

    #[test]
    fn test_deeply_nested() {
        let temp = TempDir::new().unwrap();
        let deep = temp.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        create_file(&deep, "deep.txt", b"deep content");

        let files = scan_directory(temp.path(), None, None, &[], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("deep.txt"));
    }

    #[test]
    fn test_skips_empty_files() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "empty.txt", b"");
        create_file(temp.path(), "nonempty.txt", b"content");

        let files = scan_directory(temp.path(), None, None, &[], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("nonempty.txt"));
    }

    #[test]
    fn test_exclude_by_extension() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "keep.txt", b"keep");
        create_file(temp.path(), "skip.log", b"skip");

        let files = scan_directory(temp.path(), None, None, &["*.log".to_string()], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("keep.txt"));
    }

    #[test]
    fn test_exclude_directory() {
        let temp = TempDir::new().unwrap();
        let skip_dir = temp.path().join("node_modules");
        fs::create_dir(&skip_dir).unwrap();

        create_file(temp.path(), "root.txt", b"root");
        create_file(&skip_dir, "nested.txt", b"nested");

        let files = scan_directory(
            temp.path(),
            None,
            None,
            &["**/node_modules".to_string()],
            &[],
        );

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("root.txt"));
    }

    #[test]
    fn test_exclude_multiple_patterns() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "keep.txt", b"keep");
        create_file(temp.path(), "skip.log", b"skip1");
        create_file(temp.path(), "skip.tmp", b"skip2");

        let files = scan_directory(
            temp.path(),
            None,
            None,
            &["*.log".to_string(), "*.tmp".to_string()],
            &[],
        );

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("keep.txt"));
    }

    #[test]
    fn test_exclude_nested_files() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        create_file(temp.path(), "root.log", b"root");
        create_file(&subdir, "nested.log", b"nested");
        create_file(&subdir, "keep.txt", b"keep");

        let files = scan_directory(temp.path(), None, None, &["**/*.log".to_string()], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("keep.txt"));
    }

    #[test]
    fn test_exclude_exact_filename() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "keep.txt", b"keep");
        create_file(temp.path(), "secret.env", b"skip");

        let files = scan_directory(temp.path(), None, None, &["secret.env".to_string()], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("keep.txt"));
    }

    #[test]
    fn test_exclude_deeply_nested_directory() {
        let temp = TempDir::new().unwrap();
        let deep_build = temp.path().join("src").join("app").join("build");
        fs::create_dir_all(&deep_build).unwrap();

        create_file(temp.path(), "root.txt", b"root");
        create_file(&deep_build, "output.js", b"built");

        let files = scan_directory(temp.path(), None, None, &["**/build".to_string()], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("root.txt"));
    }

    #[test]
    fn test_exclude_directory_by_name_only() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join(".cache");
        fs::create_dir(&cache_dir).unwrap();

        create_file(temp.path(), "keep.txt", b"keep");
        create_file(&cache_dir, "cached.txt", b"cached");

        // Using just the directory name without **/ prefix
        let files = scan_directory(temp.path(), None, None, &[".cache".to_string()], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("keep.txt"));
    }

    #[test]
    fn test_exclude_with_no_matches() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "file1.txt", b"one");
        create_file(temp.path(), "file2.txt", b"two");

        let files = scan_directory(temp.path(), None, None, &["*.log".to_string()], &[]);

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_exclude_all_files() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "file1.log", b"one");
        create_file(temp.path(), "file2.log", b"two");

        let files = scan_directory(temp.path(), None, None, &["*.log".to_string()], &[]);

        assert!(files.is_empty());
    }

    #[test]
    fn test_exclude_match_all() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "123.txt", b"one");
        create_file(temp.path(), "321.bin", b"two");

        let files = scan_directory(temp.path(), None, None, &["*".to_string()], &[]);

        assert!(files.is_empty());
    }

    #[test]
    fn test_exclude_preserves_non_matching_in_excluded_sibling() {
        let temp = TempDir::new().unwrap();
        let keep_dir = temp.path().join("src");
        let skip_dir = temp.path().join("node_modules");
        fs::create_dir(&keep_dir).unwrap();
        fs::create_dir(&skip_dir).unwrap();

        create_file(&keep_dir, "app.js", b"app");
        create_file(&skip_dir, "lib.js", b"lib");

        let files = scan_directory(temp.path(), None, None, &["node_modules".to_string()], &[]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("app.js"));
    }

    #[test]
    fn test_include_by_extension() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "keep.txt", b"keep");
        create_file(temp.path(), "skip.log", b"skip");

        let files = scan_directory(temp.path(), None, None, &[], &["*.txt".to_string()]);

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("keep.txt"));
    }

    #[test]
    fn test_include_multiple_patterns() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "file.txt", b"txt");
        create_file(temp.path(), "file.rs", b"rust");
        create_file(temp.path(), "file.log", b"log");

        let files = scan_directory(
            temp.path(),
            None,
            None,
            &[],
            &["*.txt".to_string(), "*.rs".to_string()],
        );

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with("file.txt")));
        assert!(files.iter().any(|f| f.path.ends_with("file.rs")));
    }

    #[test]
    fn test_include_nested_files() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        create_file(temp.path(), "root.txt", b"root");
        create_file(&subdir, "nested.txt", b"nested");
        create_file(&subdir, "other.log", b"other");

        let files = scan_directory(temp.path(), None, None, &[], &["*.txt".to_string()]);

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with("root.txt")));
        assert!(files.iter().any(|f| f.path.ends_with("nested.txt")));
    }

    #[test]
    fn test_include_with_glob_pattern() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("src");
        fs::create_dir(&src).unwrap();

        create_file(&src, "main.rs", b"main");
        create_file(temp.path(), "lib.rs", b"lib");
        create_file(temp.path(), "README.md", b"readme");

        let files = scan_directory(temp.path(), None, None, &[], &["**/*.rs".to_string()]);

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with("main.rs")));
        assert!(files.iter().any(|f| f.path.ends_with("lib.rs")));
    }

    #[test]
    fn test_include_and_exclude_combined() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "keep.txt", b"keep");
        create_file(temp.path(), "skip.txt", b"skip"); // matches include but also exclude
        create_file(temp.path(), "other.log", b"other");

        // Include *.txt but exclude skip.txt
        let files = scan_directory(
            temp.path(),
            None,
            None,
            &["skip.txt".to_string()],
            &["*.txt".to_string()],
        );

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("keep.txt"));
    }

    #[test]
    fn test_include_with_exclude_directory() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("src");
        let vendor = temp.path().join("vendor");
        fs::create_dir(&src).unwrap();
        fs::create_dir(&vendor).unwrap();

        create_file(&src, "main.rs", b"main");
        create_file(&src, "lib.rs", b"lib");
        create_file(&vendor, "dep.rs", b"vendor dep"); // matches include but in excluded dir

        // Include *.rs but exclude vendor directory
        let files = scan_directory(
            temp.path(),
            None,
            None,
            &["vendor".to_string()],
            &["*.rs".to_string()],
        );

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with("main.rs")));
        assert!(files.iter().any(|f| f.path.ends_with("lib.rs")));
        assert!(
            !files
                .iter()
                .any(|f| f.path.to_string_lossy().contains("vendor"))
        );
    }

    #[test]
    fn test_include_and_exclude_multiple_patterns() {
        let temp = TempDir::new().unwrap();
        let build = temp.path().join("build");
        fs::create_dir(&build).unwrap();

        create_file(temp.path(), "app.rs", b"app");
        create_file(temp.path(), "lib.rs", b"lib");
        create_file(temp.path(), "test.rs", b"test"); // excluded by pattern
        create_file(temp.path(), "notes.txt", b"notes");
        create_file(&build, "output.rs", b"built"); // excluded by directory

        // Include *.rs and *.txt, but exclude test.rs and build directory
        let files = scan_directory(
            temp.path(),
            None,
            None,
            &["test.rs".to_string(), "build".to_string()],
            &["*.rs".to_string(), "*.txt".to_string()],
        );

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.path.ends_with("app.rs")));
        assert!(files.iter().any(|f| f.path.ends_with("lib.rs")));
        assert!(files.iter().any(|f| f.path.ends_with("notes.txt")));
    }

    #[test]
    fn test_empty_include_means_all() {
        let temp = TempDir::new().unwrap();
        create_file(temp.path(), "file.txt", b"txt");
        create_file(temp.path(), "file.rs", b"rs");

        let files = scan_directory(temp.path(), None, None, &[], &[]);

        assert_eq!(files.len(), 2);
    }
}
