use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

/// Size of partial hash in bytes (8KB)
const PARTIAL_HASH_SIZE: usize = 8 * 1024;

/// A group of files that share the same hash
pub type HashGroup = Vec<PathBuf>;

/// A collection of hash groups
pub type HashGroups = Vec<HashGroup>;

/// Compute Blake3 hash of the first 8KB of a file
fn partial_hash_file(path: &Path) -> Option<blake3::Hash> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; PARTIAL_HASH_SIZE];

    let bytes_read = reader.read(&mut buffer).ok()?;
    buffer.truncate(bytes_read);

    Some(blake3::hash(&buffer))
}

/// Compute Blake3 hash of entire file contents
fn full_hash_file(path: &Path) -> Option<blake3::Hash> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; 64 * 1024];

    let mut hasher = blake3::Hasher::new();

    // Read in chunks
    loop {
        let bytes_read = reader.read(&mut buffer).ok()?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Some(hasher.finalize())
}

/// Generic grouping by hash
/// Returns only groups with 2+ files.
fn group_by_hash<F>(files: Vec<PathBuf>, hash_fn: F) -> HashGroups
where
    F: Fn(&Path) -> Option<blake3::Hash> + Sync,
{
    let hashes: Vec<(PathBuf, blake3::Hash)> = files
        .into_par_iter()
        .filter_map(|path| {
            let hash = hash_fn(&path)?;
            Some((path, hash))
        })
        .collect();

    let mut hash_map: HashMap<blake3::Hash, Vec<PathBuf>> = HashMap::new();
    for (path, hash) in hashes {
        hash_map.entry(hash).or_insert_with(Vec::new).push(path);
    }

    hash_map
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .map(|(_, files)| files)
        .collect()
}

/// Group files by their partial hash (first 8KB)
/// Returns only groups with 2+ files (potential duplicates)
pub fn group_by_partial_hash(files: Vec<PathBuf>) -> HashGroups {
    group_by_hash(files, partial_hash_file)
}

/// Group files by their full content hash
/// Returns only groups with 2+ files (confirmed duplicates)
pub fn group_by_full_hash(files: Vec<PathBuf>) -> HashGroups {
    group_by_hash(files, full_hash_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    #[test]
    fn test_partial_hash_identical_files() {
        let temp = TempDir::new().unwrap();
        let content = b"hello world";

        let path1 = create_file(temp.path(), "file1.txt", content);
        let path2 = create_file(temp.path(), "file2.txt", content);

        let hash1 = partial_hash_file(&path1).unwrap();
        let hash2 = partial_hash_file(&path2).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_partial_hash_different_files() {
        let temp = TempDir::new().unwrap();

        let path1 = create_file(temp.path(), "file1.txt", b"hello");
        let path2 = create_file(temp.path(), "file2.txt", b"world");

        let hash1 = partial_hash_file(&path1).unwrap();
        let hash2 = partial_hash_file(&path2).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_partial_hash_empty_file() {
        let temp = TempDir::new().unwrap();
        let path = create_file(temp.path(), "empty.txt", b"");

        let hash = partial_hash_file(&path);
        assert!(hash.is_some());
    }

    #[test]
    fn test_partial_hash_large_file_only_reads_8kb() {
        let temp = TempDir::new().unwrap();

        // Create two files: same first 8KB, different after
        let mut content1 = vec![b'A'; PARTIAL_HASH_SIZE];
        content1.extend_from_slice(b"DIFFERENT_ENDING_1");

        let mut content2 = vec![b'A'; PARTIAL_HASH_SIZE];
        content2.extend_from_slice(b"DIFFERENT_ENDING_2");

        let path1 = create_file(temp.path(), "file1.bin", &content1);
        let path2 = create_file(temp.path(), "file2.bin", &content2);

        // Partial hashes should match (same first 8KB)
        let hash1 = partial_hash_file(&path1).unwrap();
        let hash2 = partial_hash_file(&path2).unwrap();
        assert_eq!(hash1, hash2);

        // Full hashes should differ
        let full1 = full_hash_file(&path1).unwrap();
        let full2 = full_hash_file(&path2).unwrap();
        assert_ne!(full1, full2);
    }

    #[test]
    fn test_partial_hash_nonexistent_file() {
        let hash = partial_hash_file(Path::new("/nonexistent/file.txt"));
        assert!(hash.is_none());
    }

    #[test]
    fn test_group_by_partial_hash_finds_duplicates() {
        let temp = TempDir::new().unwrap();
        let content = b"duplicate content";

        let path1 = create_file(temp.path(), "dup1.txt", content);
        let path2 = create_file(temp.path(), "dup2.txt", content);
        let _unique = create_file(temp.path(), "unique.txt", b"different");

        let files = vec![path1, path2];
        let groups = group_by_partial_hash(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn test_group_by_partial_hash_filters_unique() {
        let temp = TempDir::new().unwrap();

        let path1 = create_file(temp.path(), "a.txt", b"content a");
        let path2 = create_file(temp.path(), "b.txt", b"content b");
        let path3 = create_file(temp.path(), "c.txt", b"content c");

        let files = vec![path1, path2, path3];
        let groups = group_by_partial_hash(files);

        // All unique, no groups
        assert!(groups.is_empty());
    }

    #[test]
    fn test_full_hash_file() {
        let temp = TempDir::new().unwrap();
        let content = b"test content for full hash";

        let path1 = create_file(temp.path(), "file1.txt", content);
        let path2 = create_file(temp.path(), "file2.txt", content);

        let hash1 = full_hash_file(&path1).unwrap();
        let hash2 = full_hash_file(&path2).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_group_by_full_hash_large_files() {
        let temp = TempDir::new().unwrap();

        // Two identical 100KB files
        let content: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();

        let path1 = create_file(temp.path(), "dup1.bin", &content);
        let path2 = create_file(temp.path(), "dup2.bin", &content);

        let files = vec![path1, path2];
        let groups = group_by_full_hash(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn test_full_hash_detects_late_differences() {
        let temp = TempDir::new().unwrap();

        // Two files: identical first 8KB, different after
        let mut content1 = vec![b'X'; PARTIAL_HASH_SIZE + 1000];
        let mut content2 = vec![b'X'; PARTIAL_HASH_SIZE + 1000];
        content1[PARTIAL_HASH_SIZE + 500] = b'A';
        content2[PARTIAL_HASH_SIZE + 500] = b'B';

        let path1 = create_file(temp.path(), "file1.bin", &content1);
        let path2 = create_file(temp.path(), "file2.bin", &content2);

        // Partial hashes match (same first 8KB)
        let partial1 = partial_hash_file(&path1).unwrap();
        let partial2 = partial_hash_file(&path2).unwrap();
        assert_eq!(partial1, partial2);

        // Full hashes differ
        let full1 = full_hash_file(&path1).unwrap();
        let full2 = full_hash_file(&path2).unwrap();
        assert_ne!(full1, full2);

        // group_by_full_hash should NOT group them
        let files = vec![path1, path2];
        let groups = group_by_full_hash(files);
        assert!(groups.is_empty());
    }
}
