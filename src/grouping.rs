use std::collections::HashMap;
use std::path::PathBuf;

use crate::scanner::FileEntry;

/// A collection of size groups, where each group contains files of the same size
pub type SizeGroups = Vec<Vec<PathBuf>>;

/// Groups files by size and returns size groups (each group contains files of the same size).
///
/// This is the first stage of duplicate detection - files can only be duplicates
/// if they have the same size, so we filter out unique-sized files early.
pub fn group_by_size(files: Vec<FileEntry>) -> SizeGroups {
    let mut size_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for file in files {
        size_map
            .entry(file.size)
            .or_insert_with(Vec::new)
            .push(file.path);
    }

    size_map
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .map(|(_, paths)| paths)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_entry(path: &str, size: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            size,
        }
    }

    #[test]
    fn test_empty_input() {
        let groups = group_by_size(vec![]);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_single_file_filtered_out() {
        let groups = group_by_size(vec![file_entry("/a.txt", 100)]);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_unique_sizes_filtered_out() {
        let files = vec![
            file_entry("/a.txt", 100),
            file_entry("/b.txt", 200),
            file_entry("/c.txt", 300),
        ];
        let groups = group_by_size(files);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_same_size_grouped() {
        let files = vec![file_entry("/a.txt", 100), file_entry("/b.txt", 100)];
        let groups = group_by_size(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn test_multiple_groups() {
        let files = vec![
            file_entry("/a.txt", 100),
            file_entry("/b.txt", 100),
            file_entry("/c.txt", 200),
            file_entry("/d.txt", 200),
            file_entry("/e.txt", 200),
        ];
        let groups = group_by_size(files);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_mixed_unique_and_duplicates() {
        let files = vec![
            file_entry("/unique.txt", 50),
            file_entry("/a.txt", 100),
            file_entry("/b.txt", 100),
            file_entry("/solo.txt", 999),
        ];
        let groups = group_by_size(files);
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn test_paths_preserved() {
        let files = vec![
            file_entry("/path/to/file1.txt", 100),
            file_entry("/another/path/file2.txt", 100),
        ];
        let groups = group_by_size(files);

        assert_eq!(groups.len(), 1);
        let paths: Vec<&str> = groups[0].iter().map(|p| p.to_str().unwrap()).collect();

        assert!(paths.contains(&"/path/to/file1.txt"));
        assert!(paths.contains(&"/another/path/file2.txt"));
    }
}
