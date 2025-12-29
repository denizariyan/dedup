use std::collections::HashMap;
use std::path::PathBuf;

use crate::scanner::FileEntry;

/// A group of files that share the same size
#[derive(Debug, Clone)]
pub struct SizeGroup {
    pub size: u64,
    pub files: Vec<PathBuf>,
}

/// Groups files by their size and returns only groups with potential duplicates (2+ files)
///
/// This is the first stage of duplicate detection - files can only be duplicates
/// if they have the same size, so we filter out unique-sized files early.
fn group_by_size(files: Vec<FileEntry>) -> Vec<SizeGroup> {
    let mut size_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();

    for file in files {
        size_map
            .entry(file.size)
            .or_insert_with(Vec::new)
            .push(file.path);
    }

    // Convert to SizeGroup and filter to only groups with 2+ files
    size_map
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .map(|(size, files)| SizeGroup { size, files })
        .collect()
}

/// Statistics about the grouping operation
#[derive(Debug, Clone)]
pub struct GroupingStats {
    /// Total number of files before grouping
    pub total_files: usize,
    /// Number of files that share a size with at least one other file (need hashing)
    pub n_candidate_files: usize,
    /// Number of size groups containing 2+ files
    pub n_candidate_groups: usize,
}

/// Group files by size and return both groups and statistics
pub fn group_by_size_with_stats(files: Vec<FileEntry>) -> (Vec<SizeGroup>, GroupingStats) {
    let total_files = files.len();
    let groups = group_by_size(files);

    let n_candidate_files: usize = groups.iter().map(|g| g.files.len()).sum();
    let n_candidate_groups = groups.len();

    let stats = GroupingStats {
        total_files,
        n_candidate_files,
        n_candidate_groups,
    };

    (groups, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a FileEntry for testing
    fn file_entry(path: &str, size: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            size,
        }
    }

    #[test]
    fn test_empty_input() {
        let files: Vec<FileEntry> = vec![];
        let groups = group_by_size(files);

        assert!(groups.is_empty());
    }

    #[test]
    fn test_single_file_filtered_out() {
        let files = vec![file_entry("/a.txt", 100)];
        let groups = group_by_size(files);

        // Single file has no duplicates, should be filtered out
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

        // All files have unique sizes, no potential duplicates
        assert!(groups.is_empty());
    }

    #[test]
    fn test_same_size_grouped() {
        let files = vec![
            file_entry("/a.txt", 100),
            file_entry("/b.txt", 100),
        ];
        let groups = group_by_size(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].size, 100);
        assert_eq!(groups[0].files.len(), 2);
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

        // Find each group by size
        let group_100 = groups.iter().find(|g| g.size == 100).unwrap();
        let group_200 = groups.iter().find(|g| g.size == 200).unwrap();

        assert_eq!(group_100.files.len(), 2);
        assert_eq!(group_200.files.len(), 3);
    }

    #[test]
    fn test_mixed_unique_and_duplicates() {
        let files = vec![
            file_entry("/unique.txt", 50),   // unique size - filtered out
            file_entry("/a.txt", 100),       // duplicate size
            file_entry("/b.txt", 100),       // duplicate size
            file_entry("/solo.txt", 999),    // unique size - filtered out
        ];
        let groups = group_by_size(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].size, 100);
        assert_eq!(groups[0].files.len(), 2);
    }

    #[test]
    fn test_zero_size_files() {
        let files = vec![
            file_entry("/empty1.txt", 0),
            file_entry("/empty2.txt", 0),
        ];
        let groups = group_by_size(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].size, 0);
        assert_eq!(groups[0].files.len(), 2);
    }

    #[test]
    fn test_stats_calculation() {
        let files = vec![
            file_entry("/unique.txt", 50),   // unique
            file_entry("/a.txt", 100),       // duplicate
            file_entry("/b.txt", 100),       // duplicate
            file_entry("/c.txt", 200),       // duplicate
            file_entry("/d.txt", 200),       // duplicate
            file_entry("/solo.txt", 999),    // unique
        ];
        let (groups, stats) = group_by_size_with_stats(files);

        assert_eq!(stats.total_files, 6);
        assert_eq!(stats.n_candidate_groups, 2);  // 100 and 200
        assert_eq!(stats.n_candidate_files, 4);   // 2 + 2 files
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_paths_preserved() {
        let files = vec![
            file_entry("/path/to/file1.txt", 100),
            file_entry("/another/path/file2.txt", 100),
        ];
        let groups = group_by_size(files);

        assert_eq!(groups.len(), 1);
        let paths: Vec<&str> = groups[0]
            .files
            .iter()
            .map(|p| p.to_str().unwrap())
            .collect();

        assert!(paths.contains(&"/path/to/file1.txt"));
        assert!(paths.contains(&"/another/path/file2.txt"));
    }
}
