#!/usr/bin/env python3
"""
Dedup Benchmarking Suite

Usage:
    python benchmark.py run                      # Run benchmark with defaults
    python benchmark.py run --size 1G            # Custom size
    python benchmark.py run --size 500M --runs 3

    python benchmark.py generate --output /path/to/dir  # Generate test data only
    python benchmark.py generate -o /tmp/test --profile mixed --dup-ratio 0.3 --size 1G
"""

from __future__ import annotations

import argparse
from pathlib import Path

from config import (
    DEFAULT_RESULTS_DIR,
    DEFAULT_RUNS,
    DEFAULT_SIZE,
    MATRIX_DUP_RATIOS,
    MATRIX_PROFILES,
)
from generator import generate_dataset, parse_size
from plotting import plot_matrix_results, print_matrix_summary, save_matrix_results
from runner import run_matrix_benchmark


def cmd_run(args):
    """Run the full benchmark suite."""
    total_size = parse_size(args.size)

    print("Running matrix benchmark: 3 profiles x 3 dup ratios")
    print(f"Dataset size per combination: {args.size}")
    print(f"Runs per tool: {args.runs}")

    matrix_results = run_matrix_benchmark(total_size, args.runs)

    print_matrix_summary(matrix_results)

    results_dir = Path(args.results)
    save_matrix_results(matrix_results, results_dir / "matrix_benchmark.json")
    plot_matrix_results(matrix_results, results_dir)


def cmd_generate(args):
    """Generate test dataset without running benchmarks."""
    total_size = parse_size(args.size)
    output_dir = Path(args.output)

    print("Generating test dataset:")
    print(f"  Output: {output_dir}")
    print(f"  Size: {args.size}")
    print(f"  Profile: {args.profile}")
    print(f"  Duplicate ratio: {args.dup_ratio}")

    metadata = generate_dataset(
        output_dir=output_dir,
        total_size=total_size,
        dup_ratio=args.dup_ratio,
        profile_name=args.profile,
    )

    print("\nDataset generated:")
    print(f"  Total files: {metadata['total_files']}")
    print(f"  Unique files: {metadata['unique_files']}")
    print(f"  Duplicate groups: {metadata['duplicate_groups']}")
    print(f"  Files in duplicate groups: {metadata['files_in_duplicate_groups']}")


def main():
    parser = argparse.ArgumentParser(description="Dedup Benchmarking Suite")
    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # Run subcommand
    run_parser = subparsers.add_parser("run", help="Run the full benchmark suite")
    run_parser.add_argument(
        "--size",
        "-s",
        default=DEFAULT_SIZE,
        help="Dataset size per combination (e.g. 500M, 1G)",
    )
    run_parser.add_argument(
        "--runs", "-r", type=int, default=DEFAULT_RUNS, help="Runs per tool"
    )
    run_parser.add_argument(
        "--results", default=str(DEFAULT_RESULTS_DIR), help="Results directory"
    )

    # Generate subcommand
    gen_parser = subparsers.add_parser(
        "generate", help="Generate test dataset without running benchmarks"
    )
    gen_parser.add_argument(
        "--output", "-o", required=True, help="Output directory for generated files"
    )
    gen_parser.add_argument(
        "--size",
        "-s",
        default=DEFAULT_SIZE,
        help="Total dataset size (e.g. 500M, 1G, 10G)",
    )
    gen_parser.add_argument(
        "--profile",
        "-p",
        choices=MATRIX_PROFILES,
        default="mixed",
        help="File size distribution profile",
    )
    gen_parser.add_argument(
        "--dup-ratio",
        "-d",
        type=float,
        choices=MATRIX_DUP_RATIOS,
        default=0.30,
        help="Duplicate ratio (0.10, 0.30, or 0.60)",
    )

    args = parser.parse_args()

    if args.command == "run":
        cmd_run(args)
    elif args.command == "generate":
        cmd_generate(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
