# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [UNRELEASED]

### Fixed

- Potential data loss if linking process is interrupted in critical section.

## [0.3.0] - 2026-01-02

### Added

- `--jobs` / `-j` option to limit the number of threads used for parallel processing.
- `-e` / `--exclude` option to specify glob patterns to exclude files or directories from scanning.
- `--exclude-file` option to read exclude patterns from a file. One pattern per line, similar to gitignore.
- `-i` / `--include` option to specify glob patterns to include files for scanning. If specified, only matching files are included.
- `--include-file` option to read include patterns from a file. One pattern per line.

## [0.2.0] - 2025-12-31

### Added

- `report-exit-code` action. Exits with exit code 1 if duplicates found, 0 otherwise.
- `quiet` output format. Suppresses all output, useful for scripting in combination with `report-exit-code` action.
- `--max-size` filter to skip files larger than a given size.

## [0.1.1] - 2025-12-30

### Added

- Cargo install instructions

## [0.1.0] - 2025-12-30

### Added

- Initial release
- Multi-stage duplicate detection
- Parallel file scanning and hashing with rayon
- BLAKE3 cryptographic hashing
- Hardlink replacement with `--hardlink` flag
- Dry-run mode with `--dry-run` flag
- Human-readable and JSON output formats
- Verbose mode showing duplicate groups
- Minimum file size filter with `--min-size`
