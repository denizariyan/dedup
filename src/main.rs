mod grouping;
mod scanner;

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dedup")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Directory to scan for duplicates
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,

    /// Minimum file size in bytes to consider (smaller files are skipped)
    #[arg(short = 's', long)]
    min_size: Option<u64>,

    /// Action to take on duplicates
    #[arg(short, long, value_enum, default_value_t = Action::Report)]
    action: Action,

    /// Preview changes without actually modifying files
    #[arg(long)]
    dry_run: bool,
}

/// Output format options
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable colored output
    Human,
    /// JSON output for scripting
    Json,
}

/// What to do with found duplicates
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Action {
    /// Just report duplicates (default, no file changes)
    Report,
    /// Replace duplicates with hardlinks
    Hardlink,
}

fn main() {
    let cli = Cli::parse();

    // Stage 1: Scan directory for all files
    let files = scanner::scan_directory(&cli.path, cli.min_size);
    println!("Found {} files", files.len());

    let total_size: u64 = files.iter().map(|f| f.size).sum();
    println!("Total size: {} bytes", total_size);

    // Stage 2: Group by size to find potential duplicates
    let (groups, stats) = grouping::group_by_size_with_stats(files);

    println!("\nSize grouping results:");
    println!("  Candidate groups: {}", stats.n_candidate_groups);
    println!(
        "  Candidate files (need hashing): {}",
        stats.n_candidate_files
    );
    println!(
        "  Files eliminated (unique size): {}",
        stats.total_files - stats.n_candidate_files
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli_config() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_default_values() {
        let cli = Cli::parse_from(["dedup"]);

        assert_eq!(cli.path, PathBuf::from("."));
        assert!(matches!(cli.format, OutputFormat::Human));
        assert!(matches!(cli.action, Action::Report));
        assert_eq!(cli.min_size, None);
        assert!(!cli.dry_run);
    }

    #[test]
    fn test_custom_path() {
        let cli = Cli::parse_from(["dedup", "/some/path"]);
        assert_eq!(cli.path, PathBuf::from("/some/path"));
    }

    #[test]
    fn test_json_format() {
        let cli = Cli::parse_from(["dedup", "--format", "json"]);
        assert!(matches!(cli.format, OutputFormat::Json));
    }

    #[test]
    fn test_short_format_flag() {
        let cli = Cli::parse_from(["dedup", "-f", "json"]);
        assert!(matches!(cli.format, OutputFormat::Json));
    }

    #[test]
    fn test_hardlink_action() {
        let cli = Cli::parse_from(["dedup", "--action", "hardlink"]);
        assert!(matches!(cli.action, Action::Hardlink));
    }

    #[test]
    fn test_min_size() {
        let cli = Cli::parse_from(["dedup", "--min-size", "1024"]);
        assert_eq!(cli.min_size, Some(1024));
    }

    #[test]
    fn test_short_min_size() {
        let cli = Cli::parse_from(["dedup", "-s", "4096"]);
        assert_eq!(cli.min_size, Some(4096));
    }

    #[test]
    fn test_dry_run() {
        let cli = Cli::parse_from(["dedup", "--dry-run"]);
        assert!(cli.dry_run);
    }

    #[test]
    fn test_combined_options() {
        let cli = Cli::parse_from([
            "dedup",
            "/home/user/photos",
            "-f",
            "json",
            "-a",
            "hardlink",
            "-s",
            "100",
            "--dry-run",
        ]);

        assert_eq!(cli.path, PathBuf::from("/home/user/photos"));
        assert!(matches!(cli.format, OutputFormat::Json));
        assert!(matches!(cli.action, Action::Hardlink));
        assert_eq!(cli.min_size, Some(100));
        assert!(cli.dry_run);
    }
}
