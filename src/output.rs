use std::fs;
use std::path::PathBuf;

use colored::Colorize;
use serde::Serialize;

use crate::hasher::HashGroup;
use crate::util::{format_bytes, format_number};

/// Statistics about duplicate files found
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateStats {
    /// Total number of files scanned
    pub total_files: usize,
    /// Total number of files that are duplicates
    pub duplicate_files: usize,
    /// Total wasted space in bytes (could be reclaimed)
    pub wasted_bytes: u64,
}

/// A group of duplicate files for output
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    /// Size of each file in this group
    pub size: u64,
    /// Paths to all duplicate files
    pub files: Vec<PathBuf>,
}

/// Complete report of duplicate findings
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateReport {
    pub stats: DuplicateStats,
    pub groups: Vec<DuplicateGroup>,
}

impl DuplicateReport {
    /// Build a report from hash groups
    pub fn from_groups(hash_groups: Vec<HashGroup>, total_files: usize) -> Self {
        let mut groups = Vec::with_capacity(hash_groups.len());
        let mut wasted_bytes: u64 = 0;
        let mut duplicate_files: usize = 0;

        for hash_group in hash_groups {
            // Get size from first file (all files in group have same size)
            let size = hash_group
                .first()
                .and_then(|p| fs::metadata(p).ok())
                .map(|m| m.len())
                .unwrap_or(0);

            let file_count = hash_group.len();
            duplicate_files += file_count;

            // Wasted space = size * (count - 1), since we keep one copy
            if file_count > 1 {
                wasted_bytes += size * (file_count - 1) as u64;
            }

            groups.push(DuplicateGroup {
                size,
                files: hash_group,
            });
        }

        let stats = DuplicateStats {
            total_files,
            duplicate_files,
            wasted_bytes,
        };

        Self { stats, groups }
    }

    /// Output as human-readable colored text
    pub fn print_human(&self, verbose: bool) {
        println!("\n{}", "Duplicate Report".bold().underline());
        println!(
            "  Scanned: {} files",
            format_number(self.stats.total_files).cyan()
        );
        println!(
            "  Duplicate files: {}",
            format_number(self.stats.duplicate_files).cyan()
        );
        println!(
            "  Wasted space: {}",
            format_bytes(self.stats.wasted_bytes).yellow()
        );

        if self.groups.is_empty() {
            println!("\n{}", "No duplicates found.".green());
            return;
        }

        if !verbose {
            return;
        }

        //
        // Verbose logs
        //

        for (i, group) in self.groups.iter().enumerate() {
            println!(
                "\n{} {} ({} each)",
                format!("Group {}:", format_number(i + 1)).bold(),
                format!("{} files", format_number(group.files.len())).cyan(),
                format_bytes(group.size).yellow()
            );

            for path in &group.files {
                println!("  {}", path.display());
            }
        }
    }

    /// Output as JSON
    pub fn print_json(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error serializing to JSON: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_report() {
        let report = DuplicateReport::from_groups(vec![], 100);

        assert_eq!(report.stats.total_files, 100);
        assert_eq!(report.stats.duplicate_files, 0);
        assert_eq!(report.stats.wasted_bytes, 0);
        assert!(report.groups.is_empty());
    }

    #[test]
    fn test_report_json_serialization() {
        let report = DuplicateReport {
            stats: DuplicateStats {
                total_files: 100,
                duplicate_files: 2,
                wasted_bytes: 1024,
            },
            groups: vec![DuplicateGroup {
                size: 1024,
                files: vec![PathBuf::from("/a.txt"), PathBuf::from("/b.txt")],
            }],
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"total_files\":100"));
        assert!(json.contains("\"wasted_bytes\":1024"));
    }
}
