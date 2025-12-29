use std::collections::HashMap;
use std::path::PathBuf;

use crate::scanner::FileEntry;

/// A collection of size groups, where each group contains files of the same size
pub type SizeGroups = Vec<Vec<PathBuf>>;

/// Statistics about the grouping operation
#[derive(Debug, Clone)]
pub struct GroupingStats {
    /// Total number of files before grouping
    pub total_files: usize,
    /// Number of size groups containing 2+ files
    pub n_candidate_groups: usize,
}

/// Groups files by size and returns size groups (each group contains files of the same size).
///
/// This is the first stage of duplicate detection - files can only be duplicates
/// if they have the same size, so we filter out unique-sized files early.
/// Returns groups separately so each can be processed through the pipeline independently.
pub fn group_by_size(files: Vec<FileEntry>) -> (SizeGroups, GroupingStats) {
    let total_files = files.len();
    let mut size_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for file in files {
        size_map
            .entry(file.size)
            .or_insert_with(Vec::new)
            .push(file.path);
    }

    let groups: SizeGroups = size_map
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .map(|(_, paths)| paths)
        .collect();

    let stats = GroupingStats {
        total_files,
        n_candidate_groups: groups.len(),
    };

    (groups, stats)
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
        let files: Vec<FileEntry> = vec![];
        let (groups, stats) = group_by_size(files);

        assert!(groups.is_empty());
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.n_candidate_groups, 0);
    }

    #[test]
    fn test_single_file_filtered_out() {
        let files = vec![file_entry("/a.txt", 100)];
        let (groups, stats) = group_by_size(files);

        assert!(groups.is_empty());
        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.n_candidate_groups, 0);
    }

    #[test]
    fn test_unique_sizes_filtered_out() {
        let files = vec![
            file_entry("/a.txt", 100),
            file_entry("/b.txt", 200),
            file_entry("/c.txt", 300),
        ];
        let (groups, stats) = group_by_size(files);

        assert!(groups.is_empty());
        assert_eq!(stats.total_files, 3);
        assert_eq!(stats.n_candidate_groups, 0);
    }

    #[test]
    fn test_same_size_grouped() {
        let files = vec![file_entry("/a.txt", 100), file_entry("/b.txt", 100)];
        let (groups, stats) = group_by_size(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
        assert_eq!(stats.n_candidate_groups, 1);
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
        let (groups, stats) = group_by_size(files);

        assert_eq!(groups.len(), 2);
        assert_eq!(stats.n_candidate_groups, 2);
    }

    #[test]
    fn test_mixed_unique_and_duplicates() {
        let files = vec![
            file_entry("/unique.txt", 50),
            file_entry("/a.txt", 100),
            file_entry("/b.txt", 100),
            file_entry("/solo.txt", 999),
        ];
        let (groups, stats) = group_by_size(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(stats.total_files, 4);
        assert_eq!(stats.n_candidate_groups, 1);
    }

    #[test]
    fn test_zero_size_files() {
        let files = vec![file_entry("/empty1.txt", 0), file_entry("/empty2.txt", 0)];
        let (groups, stats) = group_by_size(files);

        assert_eq!(groups.len(), 1);
        assert_eq!(stats.n_candidate_groups, 1);
    }

    #[test]
    fn test_stats_calculation() {
        let files = vec![
            file_entry("/unique.txt", 50),
            file_entry("/a.txt", 100),
            file_entry("/b.txt", 100),
            file_entry("/c.txt", 200),
            file_entry("/d.txt", 200),
            file_entry("/solo.txt", 999),
        ];
        let (groups, stats) = group_by_size(files);

        assert_eq!(stats.total_files, 6);
        assert_eq!(stats.n_candidate_groups, 2);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_paths_preserved() {
        let files = vec![
            file_entry("/path/to/file1.txt", 100),
            file_entry("/another/path/file2.txt", 100),
        ];
        let (groups, _) = group_by_size(files);

        assert_eq!(groups.len(), 1);
        let paths: Vec<&str> = groups[0].iter().map(|p| p.to_str().unwrap()).collect();

        assert!(paths.contains(&"/path/to/file1.txt"));
        assert!(paths.contains(&"/another/path/file2.txt"));
    }
}
