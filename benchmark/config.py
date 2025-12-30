from pathlib import Path

DEFAULT_RESULTS_DIR = Path("results")
DEFAULT_SIZE = "500M"
DEFAULT_RUNS = 1

# Percentage distribution of file sizes for each profile
SIZE_PROFILES = {
    "small-heavy": {"small": 0.70, "medium": 0.25, "large": 0.05},
    "mixed": {"small": 0.40, "medium": 0.40, "large": 0.20},
    "large-heavy": {"small": 0.10, "medium": 0.30, "large": 0.60},
}

# File size ranges in KB for each category
SIZE_RANGES = {
    "small": (1, 1024),
    "medium": (1024, 100 * 1024),
    "large": (100 * 1024, 10 * 1024 * 1024),
}

MATRIX_PROFILES = ["small-heavy", "mixed", "large-heavy"]
MATRIX_DUP_RATIOS = [0.10, 0.30, 0.60]
