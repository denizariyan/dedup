mod actions;
mod grouping;
mod hasher;
mod output;
mod scanner;
mod util;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

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

    /// Maximum file size in bytes to consider (larger files are skipped)
    #[arg(short = 'S', long)]
    max_size: Option<u64>,

    /// Action to take on duplicates
    #[arg(short, long, value_enum, default_value_t = Action::None)]
    action: Action,

    /// Preview changes without actually modifying files
    #[arg(long)]
    dry_run: bool,

    /// Show detailed output
    #[arg(short, long)]
    verbose: bool,

    /// Disable progress bars
    #[arg(long)]
    no_progress: bool,

    /// Number of threads to use (defaults to number of CPU cores)
    #[arg(short = 'j', long)]
    jobs: Option<usize>,

    /// Glob patterns to exclude (can be specified multiple times)
    #[arg(short = 'e', long = "exclude", action = clap::ArgAction::Append)]
    exclude: Vec<String>,

    /// File containing exclude patterns (one per line, like .gitignore)
    #[arg(long = "exclude-file")]
    exclude_file: Option<PathBuf>,

    /// Glob patterns to include (can be specified multiple times). If specified, only matching files are scanned.
    #[arg(short = 'i', long = "include", action = clap::ArgAction::Append)]
    include: Vec<String>,

    /// File containing include patterns (one per line)
    #[arg(long = "include-file")]
    include_file: Option<PathBuf>,
}

/// Output format options
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable colored output
    Human,
    /// JSON output for scripting
    Json,
    /// No output (useful with report-exit-code action)
    Quiet,
}

/// What to do with found duplicates
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Action {
    /// Just report duplicates (default, no file changes)
    None,
    /// Report duplicates and exit with code 1 if any found
    ReportExitCode,
    /// Replace duplicates with hardlinks
    Hardlink,
}

/// Parse a glob file (gitignore-style) and return patterns
fn parse_glob_file(path: &std::path::Path) -> Vec<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(String::from)
            .collect(),
        Err(e) => {
            eprintln!(
                "Warning: could not read exclude file '{}': {}",
                path.display(),
                e
            );
            Vec::new()
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if let Some(num_threads) = cli.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .expect("Failed to initialize thread pool");
    }

    let human = matches!(cli.format, OutputFormat::Human);
    let quiet = matches!(cli.format, OutputFormat::Quiet);
    let show_progress = human && !cli.no_progress;

    // Stage 1: Scan directory for all files
    let scan_spinner = if show_progress {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        sp.set_message("Scanning files...");
        sp.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(sp)
    } else {
        None
    };

    // Combine exclude patterns from --exclude and --exclude-file
    let mut exclude_patterns = cli.exclude.clone();
    if let Some(ref exclude_file) = cli.exclude_file {
        exclude_patterns.extend(parse_glob_file(exclude_file));
    }

    // Combine include patterns from --include and --include-file
    let mut include_patterns = cli.include.clone();
    if let Some(ref include_file) = cli.include_file {
        include_patterns.extend(parse_glob_file(include_file));
    }

    let files = scanner::scan_directory(
        &cli.path,
        cli.min_size,
        cli.max_size,
        &exclude_patterns,
        &include_patterns,
    );
    let total_files = files.len();

    if let Some(sp) = scan_spinner {
        sp.finish_and_clear();
    }

    // Stage 2: Group by size to find potential duplicates
    let size_groups = grouping::group_by_size(files);
    let candidate_count: usize = size_groups.iter().map(|g| g.len()).sum();

    // Stage 3 & 4: Process each size group through partial hash -> full hash pipeline
    let progress_bar = if show_progress && candidate_count > 0 {
        let pb = ProgressBar::new(candidate_count as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} files compared")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let processed = AtomicUsize::new(0);

    let duplicate_groups: hasher::HashGroups = size_groups
        .into_par_iter()
        .flat_map(|size_group| {
            let group_size = size_group.len();
            let partial_groups = hasher::group_by_partial_hash(size_group);

            let final_groups = partial_groups
                .into_par_iter()
                .flat_map(hasher::group_by_full_hash);

            if let Some(ref pb) = progress_bar {
                let prev = processed.fetch_add(group_size, Ordering::Relaxed);
                pb.set_position((prev + group_size) as u64);
            }

            final_groups
        })
        .collect();

    let report = output::DuplicateReport::from_groups(duplicate_groups, total_files);

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    match cli.format {
        OutputFormat::Human => report.print_human(cli.verbose),
        OutputFormat::Json => report.print_json(),
        OutputFormat::Quiet => {}
    }

    match cli.action {
        Action::None => {}
        Action::ReportExitCode => {
            if !report.groups.is_empty() {
                std::process::exit(1);
            }
        }
        Action::Hardlink => {
            let result =
                actions::hardlink_duplicates(&report.groups, cli.dry_run, cli.verbose && !quiet);

            if human {
                if cli.dry_run {
                    println!(
                        "\n[dry-run] Would link {} files, saving {}",
                        result.files_linked,
                        util::format_bytes(result.bytes_saved)
                    );
                } else {
                    println!(
                        "\nLinked {} files, saved {}",
                        result.files_linked,
                        util::format_bytes(result.bytes_saved)
                    );
                }

                if !result.errors.is_empty() {
                    eprintln!("\nErrors ({}):", result.errors.len());
                    for (path, err) in &result.errors {
                        eprintln!("  {}: {}", path.display(), err);
                    }
                }
            }
        }
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
        assert!(matches!(cli.action, Action::None));
        assert_eq!(cli.min_size, None);
        assert!(!cli.dry_run);
        assert!(!cli.verbose);
        assert!(!cli.no_progress);
    }

    #[test]
    fn test_report_exit_code_action() {
        let cli = Cli::parse_from(["dedup", "--action", "report-exit-code"]);
        assert!(matches!(cli.action, Action::ReportExitCode));
    }

    #[test]
    fn test_verbose_flag() {
        let cli = Cli::parse_from(["dedup", "--verbose"]);
        assert!(cli.verbose);

        let cli = Cli::parse_from(["dedup", "-v"]);
        assert!(cli.verbose);
    }

    #[test]
    fn test_no_progress_flag() {
        let cli = Cli::parse_from(["dedup", "--no-progress"]);
        assert!(cli.no_progress);
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
    fn test_quiet_format() {
        let cli = Cli::parse_from(["dedup", "--format", "quiet"]);
        assert!(matches!(cli.format, OutputFormat::Quiet));
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
    fn test_max_size() {
        let cli = Cli::parse_from(["dedup", "--max-size", "1048576"]);
        assert_eq!(cli.max_size, Some(1048576));
    }

    #[test]
    fn test_short_max_size() {
        let cli = Cli::parse_from(["dedup", "-S", "2048"]);
        assert_eq!(cli.max_size, Some(2048));
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

    #[test]
    fn test_jobs_flag() {
        let cli = Cli::parse_from(["dedup", "--jobs", "4"]);
        assert_eq!(cli.jobs, Some(4));

        let cli = Cli::parse_from(["dedup", "-j", "2"]);
        assert_eq!(cli.jobs, Some(2));
    }

    #[test]
    fn test_exclude_flag() {
        let cli = Cli::parse_from(["dedup", "--exclude", "*.log"]);
        assert_eq!(cli.exclude, vec!["*.log"]);

        let cli = Cli::parse_from(["dedup", "-e", "*.tmp", "-e", "*.log"]);
        assert_eq!(cli.exclude, vec!["*.tmp", "*.log"]);
    }

    #[test]
    fn test_exclude_file_flag() {
        let cli = Cli::parse_from(["dedup", "--exclude-file", ".gitignore"]);
        assert_eq!(cli.exclude_file, Some(PathBuf::from(".gitignore")));
    }

    #[test]
    fn test_parse_glob_file() {
        use std::io::Write;
        let temp = tempfile::NamedTempFile::new().unwrap();
        writeln!(temp.as_file(), "# This is a comment").unwrap();
        writeln!(temp.as_file(), "*.log").unwrap();
        #[allow(clippy::writeln_empty_string)] // Testing empty lines
        writeln!(temp.as_file(), "").unwrap();
        writeln!(temp.as_file(), " ").unwrap();
        writeln!(temp.as_file(), "  # Another comment with leading space").unwrap();
        writeln!(temp.as_file(), "node_modules").unwrap();
        writeln!(temp.as_file(), "  *.tmp  ").unwrap();

        let patterns = parse_glob_file(temp.path());
        assert_eq!(patterns, vec!["*.log", "node_modules", "*.tmp"]);
    }

    #[test]
    fn test_parse_glob_file_nonexistent() {
        let patterns = parse_glob_file(std::path::Path::new("/nonexistent/file"));
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_include_flag() {
        let cli = Cli::parse_from(["dedup", "--include", "*.rs"]);
        assert_eq!(cli.include, vec!["*.rs"]);

        let cli = Cli::parse_from(["dedup", "-i", "*.txt", "-i", "*.rs"]);
        assert_eq!(cli.include, vec!["*.txt", "*.rs"]);
    }

    #[test]
    fn test_include_file_flag() {
        let cli = Cli::parse_from(["dedup", "--include-file", "include.txt"]);
        assert_eq!(cli.include_file, Some(PathBuf::from("include.txt")));
    }
}
