# dedup

A fast duplicate file finder written in Rust. Identifies duplicate files by content hash and optionally replaces them with hardlinks to reclaim disk space.

## Features

- Multi-stage filtering: size grouping -> partial hash (8KB) -> full hash
- Parallel processing with rayon
- BLAKE3 hashing (fast, cryptographically secure)
- Hardlink replacement with dry-run support
- Human-readable and JSON output formats

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/dedup`.

## Usage

```bash
# Scan current directory, report duplicates
dedup

# Scan specific directory
dedup /path/to/directory

# Show each duplicate group (verbose)
dedup /path -v

# Output as JSON
dedup /path --format json

# Dry-run replacing duplicates with hardlinks
dedup /path --action hardlink --dry-run

# Actually replace duplicates with hardlinks
dedup /path --action hardlink

# Skip files smaller than 1KB
dedup /path --min-size 1024

# Disable progress bars (useful for scripts/CI)
dedup /path --no-progress
```

## CLI Options

| Option               | Short | Description                                      |
| -------------------- | ----- | ------------------------------------------------ |
| `--format <FORMAT>`  | `-f`  | Output format: `human` (default) or `json`       |
| `--action <ACTION>`  | `-a`  | Action: `report` (default) or `hardlink`         |
| `--min-size <BYTES>` | `-s`  | Skip files smaller than this size                |
| `--verbose`          | `-v`  | Show detailed output with file paths             |
| `--dry-run`          |       | Preview hardlink changes without modifying files |
| `--no-progress`      |       | Disable progress bars                            |

## How It Works

The tool uses a multi-stage pipeline to minimize disk I/O to reduce runtime:

1. **Scan**: Walk directory tree, collect file paths and sizes
2. **Size grouping**: Group files by size. Files with unique sizes cannot be duplicates — skip them early.
3. **Partial hash**: For remaining candidates, hash only the first 8KB. Different partial hashes = different files.
4. **Full hash**: For files with matching partial hashes, compute full content hash to confirm duplicates.

This approach avoids reading entire file contents for most files.

```
1000 files
    ↓ size grouping
  200 candidates (800 unique sizes skipped)
    ↓ partial hash (8KB each)
   50 candidates (150 different starts)
    ↓ full hash
   20 confirmed duplicates
```

## Hardlinking

When using `--action hardlink`, duplicate files are replaced with hardlinks to a single copy.

Note that hardlinking files means the metadata such as file ownership and permissions are lost for the duplicates which are replaced by hardlinks.
It is planned to improve this in future versions, but hardlinking is the only option for now.

If you are packaging the deduplicated files later, consider using a hardlink-aware archiver like `tar` to benefit from space savings.

- All hardlinked files share the same inode (same data on disk)
- Deleting any one file doesn't affect the others
- Original selection: shortest path is kept as the "original", others are replaced with hardlinks to it.
- Only works within the same filesystem and requires write permissions on the "original" file.

Use `--dry-run --verbose` first to preview what would change.

## Output Formats

### Human (default)

```
Duplicate Report
  Groups: 3
  Total duplicate files: 12
  Wasted space: 45.2 MB
```

### JSON

```json
{
  "stats": {
    "duplicate_groups": 3,
    "duplicate_files": 12,
    "wasted_bytes": 47412224
  },
  "groups": [
    {
      "size": 15804074,
      "files": ["/path/to/file1.jpg", "/path/to/file2.jpg"]
    }
  ]
}
```

## Limitations

- Hardlinks only work within the same filesystem (cross-device links will fail and be reported as errors)
- Symlinks are ignored

## License

MIT License. See `LICENSE` file for details.
