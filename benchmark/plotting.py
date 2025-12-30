from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING

import matplotlib.pyplot as plt
import numpy as np

if TYPE_CHECKING:
    from runner import MatrixResult


def plot_matrix_results(matrix_results: list[MatrixResult], output_dir: Path):
    """Generate plots for matrix benchmark results."""
    output_dir.mkdir(parents=True, exist_ok=True)

    tools = []
    for mr in matrix_results:
        for r in mr.results:
            if r.tool not in tools and not r.error:
                tools.append(r.tool)

    profiles = list(dict.fromkeys(mr.profile for mr in matrix_results))
    dup_ratios = list(dict.fromkeys(mr.dup_ratio for mr in matrix_results))
    colors = ["#2ecc71", "#3498db", "#e74c3c", "#9b59b6", "#f39c12"]

    # Time by profile chart
    _, axes = plt.subplots(len(profiles), 1, figsize=(12, 4 * len(profiles)), squeeze=False)

    for i, profile in enumerate(profiles):
        ax = axes[i, 0]
        profile_results = [mr for mr in matrix_results if mr.profile == profile]

        x = np.arange(len(dup_ratios))
        width = 0.8 / len(tools)

        for j, tool in enumerate(tools):
            times = []
            for mr in profile_results:
                tool_result = next((r for r in mr.results if r.tool == tool), None)
                times.append(tool_result.avg_time if tool_result and not tool_result.error else 0)

            offset = (j - len(tools) / 2 + 0.5) * width
            bars = ax.bar(x + offset, times, width, label=tool, color=colors[j % len(colors)])

            for bar, t in zip(bars, times):
                if t > 0:
                    ax.annotate(f"{t:.2f}s", xy=(bar.get_x() + bar.get_width() / 2, bar.get_height()),
                               xytext=(0, 2), textcoords="offset points", ha="center", va="bottom", fontsize=8)

        ax.set_ylabel("Time (seconds)")
        ax.set_title(f"Profile: {profile}")
        ax.set_xticks(x)
        ax.set_xticklabels([f"{r:.0%}" for r in dup_ratios])
        ax.set_xlabel("Duplicate Ratio")
        ax.legend(loc="upper left")

    plt.suptitle("Benchmark Results by Profile and Duplicate Ratio", fontsize=14, fontweight="bold")
    plt.tight_layout()
    plt.savefig(output_dir / "matrix_by_profile.png", dpi=150)
    plt.close()

    # Performance heatmap
    config_labels = [f"{mr.profile[:5]}\n{mr.dup_ratio:.0%}" for mr in matrix_results]
    _plot_heatmap(matrix_results, tools, config_labels, output_dir, "time")

    # Memory by profile chart
    fig, axes = plt.subplots(len(profiles), 1, figsize=(12, 4 * len(profiles)), squeeze=False)

    for i, profile in enumerate(profiles):
        ax = axes[i, 0]
        profile_results = [mr for mr in matrix_results if mr.profile == profile]

        x = np.arange(len(dup_ratios))
        width = 0.8 / len(tools)

        for j, tool in enumerate(tools):
            memory_values = []
            for mr in profile_results:
                tool_result = next((r for r in mr.results if r.tool == tool), None)
                if tool_result and not tool_result.error and tool_result.memory_kb:
                    memory_values.append(tool_result.max_memory_mb)
                else:
                    memory_values.append(0)

            offset = (j - len(tools) / 2 + 0.5) * width
            bars = ax.bar(x + offset, memory_values, width, label=tool, color=colors[j % len(colors)])

            for bar, m in zip(bars, memory_values):
                if m > 0:
                    ax.annotate(f"{m:.0f}", xy=(bar.get_x() + bar.get_width() / 2, bar.get_height()),
                               xytext=(0, 2), textcoords="offset points", ha="center", va="bottom", fontsize=8)

        ax.set_ylabel("Peak Memory (MB)")
        ax.set_title(f"Profile: {profile}")
        ax.set_xticks(x)
        ax.set_xticklabels([f"{r:.0%}" for r in dup_ratios])
        ax.set_xlabel("Duplicate Ratio")
        ax.legend(loc="upper left")

    plt.suptitle("Memory Usage by Profile and Duplicate Ratio", fontsize=14, fontweight="bold")
    plt.tight_layout()
    plt.savefig(output_dir / "matrix_memory_by_profile.png", dpi=150)
    plt.close()

    # Memory heatmap
    _plot_heatmap(matrix_results, tools, config_labels, output_dir, "memory")

    print(f"Saved matrix plots to {output_dir}/")


def _plot_heatmap(matrix_results: list[MatrixResult], tools: list[str], config_labels: list[str], output_dir: Path, metric: str):
    """Generate heatmap for time or memory."""
    fig, ax = plt.subplots(figsize=(10, 6))

    data = []
    for tool in tools:
        row = []
        for mr in matrix_results:
            tool_result = next((r for r in mr.results if r.tool == tool), None)
            if metric == "time":
                row.append(tool_result.avg_time if tool_result and not tool_result.error else float("nan"))
            else:
                if tool_result and not tool_result.error and tool_result.memory_kb:
                    row.append(tool_result.max_memory_mb)
                else:
                    row.append(float("nan"))
        data.append(row)

    data = np.array(data)
    with np.errstate(invalid="ignore"):
        normalized = np.nanmin(data, axis=0) / data

    im = ax.imshow(normalized, cmap="RdYlGn", aspect="auto", vmin=0, vmax=1)

    ax.set_xticks(np.arange(len(config_labels)))
    ax.set_yticks(np.arange(len(tools)))
    ax.set_xticklabels(config_labels, fontsize=8)
    ax.set_yticklabels(tools)

    for i in range(len(tools)):
        for j in range(len(matrix_results)):
            val = normalized[i, j]
            data_val = data[i, j]
            if not np.isnan(val):
                text = f"{data_val:.2f}s" if metric == "time" else f"{data_val:.0f}MB"
                color = "white" if val < 0.5 else "black"
                ax.text(j, i, text, ha="center", va="center", color=color, fontsize=8)

    title = "Performance Heatmap (green = faster)" if metric == "time" else "Memory Heatmap (green = less memory)"
    ax.set_title(title)
    label = "Relative Speed (1.0 = fastest)" if metric == "time" else "Relative Memory (1.0 = lowest)"
    plt.colorbar(im, ax=ax, label=label)
    plt.tight_layout()

    filename = "matrix_heatmap.png" if metric == "time" else "matrix_memory_heatmap.png"
    plt.savefig(output_dir / filename, dpi=150)
    plt.close()


def save_matrix_results(matrix_results: list[MatrixResult], output_path: Path):
    """Save matrix results to JSON."""
    data = {
        "matrix": [
            {
                "profile": mr.profile,
                "dup_ratio": mr.dup_ratio,
                "metadata": mr.metadata,
                "results": [
                    {
                        "tool": r.tool,
                        "avg_time": r.avg_time,
                        "min_time": r.min_time,
                        "times": r.times,
                        "memory_kb": r.memory_kb,
                        "avg_memory_mb": r.avg_memory_mb,
                        "max_memory_mb": r.max_memory_mb,
                        "duplicates_found": r.duplicates_found,
                        "expected_duplicates": r.expected_duplicates,
                        "accuracy": r.accuracy,
                        "error": r.error,
                    }
                    for r in mr.results
                ],
            }
            for mr in matrix_results
        ]
    }

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(data, indent=2))
    print(f"Saved matrix results to {output_path}")


def print_matrix_summary(matrix_results: list[MatrixResult]):
    """Print summary table of matrix results."""
    print("\n" + "=" * 80)
    print("MATRIX BENCHMARK SUMMARY")
    print("=" * 80)

    tools = []
    for mr in matrix_results:
        for r in mr.results:
            if r.tool not in tools:
                tools.append(r.tool)

    header = f"{'Config':<20}"
    for tool in tools:
        header += f"{tool:>12}"
    print(header)
    print("-" * 80)

    for mr in matrix_results:
        config = f"{mr.profile[:10]} {mr.dup_ratio:.0%}"
        row = f"{config:<20}"
        for tool in tools:
            result = next((r for r in mr.results if r.tool == tool), None)
            if result and not result.error:
                row += f"{result.avg_time:>10.2f}s "
            else:
                row += f"{'N/A':>12}"
        print(row)

    print("\n" + "-" * 80)
    print("PEAK MEMORY (MB)")
    print("-" * 80)

    header = f"{'Config':<20}"
    for tool in tools:
        header += f"{tool:>12}"
    print(header)
    print("-" * 80)

    for mr in matrix_results:
        config = f"{mr.profile[:10]} {mr.dup_ratio:.0%}"
        row = f"{config:<20}"
        for tool in tools:
            result = next((r for r in mr.results if r.tool == tool), None)
            if result and not result.error and result.memory_kb:
                row += f"{result.max_memory_mb:>10.1f}MB"
            else:
                row += f"{'N/A':>12}"
        print(row)

    print("=" * 80)
