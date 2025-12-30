#!/usr/bin/env python3
"""
Dedup Benchmarking Suite

Usage:
    python benchmark.py                     # Run with defaults (500M, 1 run)
    python benchmark.py --size 1G           # Custom size
    python benchmark.py --size 500M --runs 3
"""

from __future__ import annotations

import argparse
from pathlib import Path

from config import DEFAULT_RESULTS_DIR, DEFAULT_RUNS, DEFAULT_SIZE
from generator import parse_size
from plotting import plot_matrix_results, print_matrix_summary, save_matrix_results
from runner import run_matrix_benchmark


def main():
    parser = argparse.ArgumentParser(description="Dedup Benchmarking Suite")
    parser.add_argument(
        "--size",
        "-s",
        default=DEFAULT_SIZE,
        help="Dataset size per combination (e.g. 500M, 1G)",
    )
    parser.add_argument(
        "--runs", "-r", type=int, default=DEFAULT_RUNS, help="Runs per tool"
    )
    parser.add_argument(
        "--results", default=str(DEFAULT_RESULTS_DIR), help="Results directory"
    )

    args = parser.parse_args()

    total_size = parse_size(args.size)

    print("Running matrix benchmark: 3 profiles x 3 dup ratios")
    print(f"Dataset size per combination: {args.size}")
    print(f"Runs per tool: {args.runs}")

    matrix_results = run_matrix_benchmark(total_size, args.runs)

    print_matrix_summary(matrix_results)

    results_dir = Path(args.results)
    save_matrix_results(matrix_results, results_dir / "matrix_benchmark.json")
    plot_matrix_results(matrix_results, results_dir)


if __name__ == "__main__":
    main()
