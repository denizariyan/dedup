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

    println!("Scanning: {:?}", cli.path);
    println!("Format: {:?}", cli.format);
    println!("Action: {:?}", cli.action);

    if let Some(min) = cli.min_size {
        println!("Min size: {} bytes", min);
    }

    if cli.dry_run {
        println!("Dry run mode enabled");
    }
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
