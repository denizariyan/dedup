from __future__ import annotations

import re
import shutil
import subprocess
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path

from config import MATRIX_DUP_RATIOS, MATRIX_PROFILES
from generator import generate_dataset
from tools import ALL_TOOLS, DedupTool


def drop_caches() -> bool:
    """Drop filesystem caches. Requires root on Linux."""
    try:
        subprocess.run(
            ["sh", "-c", "sync; echo 3 > /proc/sys/vm/drop_caches"],
            check=True,
            capture_output=True,
        )
        return True
    except (subprocess.CalledProcessError, PermissionError, FileNotFoundError):
        return False


@dataclass
class BenchmarkResult:
    tool: str
    times: list[float] = field(default_factory=list)
    memory_kb: list[int] = field(default_factory=list)
    duplicates_found: int | None = None
    expected_duplicates: int | None = None
    error: str | None = None
    stdout_sample: str | None = None
    stderr_sample: str | None = None

    @property
    def avg_time(self) -> float:
        return sum(self.times) / len(self.times) if self.times else 0.0

    @property
    def min_time(self) -> float:
        return min(self.times) if self.times else 0.0

    @property
    def avg_memory_mb(self) -> float:
        return (
            (sum(self.memory_kb) / len(self.memory_kb) / 1024)
            if self.memory_kb
            else 0.0
        )

    @property
    def max_memory_mb(self) -> float:
        return (max(self.memory_kb) / 1024) if self.memory_kb else 0.0

    @property
    def accuracy(self) -> float | None:
        if self.duplicates_found is None or self.expected_duplicates is None:
            return None
        if self.expected_duplicates == 0:
            return 100.0 if self.duplicates_found == 0 else 0.0
        return (self.duplicates_found / self.expected_duplicates) * 100


def parse_time_output(stderr: str) -> int | None:
    """Extract max RSS from /usr/bin/time -v output."""
    match = re.search(r"Maximum resident set size \(kbytes\): (\d+)", stderr)
    return int(match.group(1)) if match else None


def strip_time_output(stderr: str) -> str:
    """Remove /usr/bin/time output from stderr."""
    lines = stderr.split("\n")
    result = []
    for line in lines:
        if "Command being timed:" in line:
            break
        result.append(line)
    return "\n".join(result)


def run_benchmark(
    tool: DedupTool,
    dataset_path: Path,
    runs: int,
    expected_duplicates: int | None = None,
    verbose: bool = False,
) -> BenchmarkResult:
    """Run a tool multiple times and collect timing data."""
    result = BenchmarkResult(tool=tool.name, expected_duplicates=expected_duplicates)

    if not tool.is_available():
        result.error = "Tool not available"
        return result

    cmd = tool.command(str(dataset_path))
    time_cmd = ["/usr/bin/time", "-v"] + cmd
    print(f"  Running {tool.name}... (cmd: {' '.join(cmd)})")

    drop_caches()
    print("    Warmup run (discarded)...")
    try:
        subprocess.run(time_cmd, capture_output=True, text=True, timeout=600)
    except (subprocess.TimeoutExpired, Exception):
        pass

    for i in range(runs):
        drop_caches()
        start = time.perf_counter()
        try:
            proc = subprocess.run(time_cmd, capture_output=True, text=True, timeout=600)
            elapsed = time.perf_counter() - start
            result.times.append(elapsed)

            memory_kb = parse_time_output(proc.stderr)
            if memory_kb:
                result.memory_kb.append(memory_kb)

            tool_stderr = strip_time_output(proc.stderr)
            if i == 0:
                result.duplicates_found = tool.parse_output(proc.stdout, tool_stderr)
                result.stdout_sample = proc.stdout[:2000] if proc.stdout else None
                result.stderr_sample = tool_stderr[:1000] if tool_stderr else None

            mem_str = f", {memory_kb / 1024:.1f} MB" if memory_kb else ""
            print(f"    Run {i + 1}/{runs}: {elapsed:.2f}s{mem_str}")

        except subprocess.TimeoutExpired:
            result.error = "Timeout"
            break
        except Exception as e:
            result.error = str(e)
            break

    if verbose or result.duplicates_found == 0 or result.duplicates_found is None:
        print(f"    [DEBUG] duplicates_found={result.duplicates_found}")
        if result.stdout_sample:
            print(
                f"    [DEBUG] stdout (first 500 chars):\n{result.stdout_sample[:500]}"
            )
        if result.stderr_sample:
            print(f"    [DEBUG] stderr:\n{result.stderr_sample[:300]}")

    return result


def run_all_benchmarks(
    dataset_path: Path,
    runs: int,
    expected_duplicates: int | None = None,
) -> list[BenchmarkResult]:
    """Run all available tools."""
    results = []

    available = []
    unavailable = []
    for tool_class in ALL_TOOLS:
        tool = tool_class()
        if tool.is_available():
            available.append(tool)
        else:
            unavailable.append(tool.name)

    if unavailable:
        print(f"Skipping unavailable tools: {', '.join(unavailable)}")

    print(f"Running: {', '.join(t.name for t in available)}\n")

    for tool in available:
        result = run_benchmark(tool, dataset_path, runs, expected_duplicates)
        results.append(result)

    check_tool_results(results)
    return results


def check_tool_results(results: list[BenchmarkResult]) -> None:
    """Check if the tools found the expected number of duplicates."""
    counts = {}
    expected = None
    for r in results:
        if not r.error and r.duplicates_found is not None:
            counts[r.tool] = r.duplicates_found
        if r.expected_duplicates is not None:
            expected = r.expected_duplicates

    if not counts:
        return

    unique_counts = set(counts.values())
    all_match_expected = expected is not None and all(
        c == expected for c in counts.values()
    )

    if len(unique_counts) == 1 and all_match_expected:
        return

    print("\n  Duplicate count comparison:")
    if expected is not None:
        print(f"    Expected (generated): {expected}")
    for tool, count in counts.items():
        match = ""
        if expected is not None:
            match = " OK" if count == expected else f" (off by {count - expected:+d})"
        print(f"    {tool}: {count}{match}")
    print()


@dataclass
class MatrixResult:
    profile: str
    dup_ratio: float
    metadata: dict
    results: list[BenchmarkResult]


def run_matrix_benchmark(
    total_size: int,
    runs: int,
    profiles: list[str] | None = None,
    dup_ratios: list[float] | None = None,
) -> list[MatrixResult]:
    """Run benchmarks across all profile/dup_ratio combinations."""
    profiles = profiles or MATRIX_PROFILES
    dup_ratios = dup_ratios or MATRIX_DUP_RATIOS

    all_results: list[MatrixResult] = []
    total_combinations = len(profiles) * len(dup_ratios)
    current = 0

    for profile in profiles:
        for dup_ratio in dup_ratios:
            current += 1
            print(f"\n{'=' * 60}")
            print(
                f"[{current}/{total_combinations}] Profile: {profile}, Dup ratio: {dup_ratio:.0%}"
            )
            print("=" * 60)

            temp_dir = tempfile.mkdtemp(
                prefix=f"dedup_bench_{profile}_{int(dup_ratio * 100)}_"
            )
            output_dir = Path(temp_dir) / "demo_files"

            try:
                metadata = generate_dataset(
                    output_dir=output_dir,
                    total_size=total_size,
                    dup_ratio=dup_ratio,
                    profile_name=profile,
                )

                expected_dups = metadata.get("files_in_duplicate_groups")
                results = run_all_benchmarks(output_dir, runs, expected_dups)

                all_results.append(
                    MatrixResult(
                        profile=profile,
                        dup_ratio=dup_ratio,
                        metadata=metadata,
                        results=results,
                    )
                )
            finally:
                shutil.rmtree(temp_dir, ignore_errors=True)

    return all_results
