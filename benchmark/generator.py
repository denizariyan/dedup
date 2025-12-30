from __future__ import annotations

import hashlib
import json
import random
import shutil
from collections import Counter
from pathlib import Path

from tqdm import tqdm

from config import SIZE_PROFILES, SIZE_RANGES


def parse_size(size_str: str) -> int:
    """Parse size string like '500M', '2G' into bytes."""
    size_str = size_str.strip().upper()
    multipliers = {"B": 1, "K": 1024, "M": 1024**2, "G": 1024**3}
    if size_str[-1] in multipliers:
        return int(float(size_str[:-1]) * multipliers[size_str[-1]])
    return int(size_str)


def generate_content(seed: int, size: int) -> bytes:
    """Generate deterministic content based on seed."""
    h = hashlib.sha256(seed.to_bytes(8, "little")).digest()
    repeats = (size // len(h)) + 1
    return (h * repeats)[:size]


def pick_size(profile: dict[str, float], rng_seed: int) -> int:
    """Pick a file size based on profile distribution using cumulative weighted random choice."""
    rng = random.Random(rng_seed)
    r = rng.random()
    cumulative = 0.0
    category = None
    for cat, weight in profile.items():
        cumulative += weight
        if r < cumulative:
            category = cat
            break

    if category is None:
        raise ValueError("Failed to pick size category")
    min_size, max_size = SIZE_RANGES[category]
    return rng.randint(min_size, max_size)


def generate_dataset(
    output_dir: Path,
    total_size: int,
    dup_ratio: float,
    profile_name: str,
) -> dict:
    """Generate test dataset with specified parameters."""
    profile = SIZE_PROFILES[profile_name]
    rng = random.Random(42)

    if output_dir.resolve() in (Path("/").resolve(), Path.home().resolve()):
        raise ValueError("Output directory is too dangerous to delete")

    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True)

    unique_seeds: list[tuple[int, int]] = []
    seed_usage: Counter[int] = Counter()
    files_created = 0
    bytes_written = 0
    duplicates_created = 0

    with tqdm(
        total=total_size,
        unit="B",
        unit_scale=True,
        unit_divisor=1024,
        desc="Generating test files",
        leave=True,
    ) as pbar:
        while bytes_written < total_size:
            is_duplicate = rng.random() < dup_ratio and unique_seeds

            if is_duplicate:
                seed, size = rng.choice(unique_seeds)
                duplicates_created += 1
            else:
                seed = rng.randint(0, 2**32)
                size = pick_size(profile, seed)
                unique_seeds.append((seed, size))

            seed_usage[seed] += 1

            dir_a = f"{files_created % 256:02x}"
            dir_b = f"{(files_created // 256) % 256:02x}"
            file_dir = output_dir / dir_a / dir_b
            file_dir.mkdir(parents=True, exist_ok=True)

            file_path = file_dir / f"file_{files_created:06d}.bin"
            content = generate_content(seed, size)
            file_path.write_bytes(content)

            bytes_written += size
            files_created += 1
            pbar.update(size)

    files_in_dup_groups = sum(count for count in seed_usage.values() if count >= 2)
    dup_groups = sum(1 for count in seed_usage.values() if count >= 2)

    metadata = {
        "total_files": files_created,
        "total_bytes": bytes_written,
        "unique_files": len(unique_seeds),
        "duplicate_files": duplicates_created,
        "duplicate_ratio": dup_ratio,
        "profile": profile_name,
        "files_in_duplicate_groups": files_in_dup_groups,
        "duplicate_groups": dup_groups,
    }

    (output_dir / "metadata.json").write_text(json.dumps(metadata, indent=2))
    print(
        f"Created {files_created} files ({bytes_written / 1024**2:.1f} MB) - {len(unique_seeds)} unique, {duplicates_created} duplicates, {files_in_dup_groups} in dup groups"
    )

    return metadata
