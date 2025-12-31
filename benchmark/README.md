# Benchmarks

This directory contains benchmarking tools and results for dedup and other duplicate file finders running on a variety of synthetic datasets.
Where possible, progress bars and other non-essential output are disabled and the tools are running with their default settings.

All benchmarks use the latest available version of each tool at the time.

## Compared Tools

- dedup (this project)
- fdupes
- fclones
- rdfind
- bash script using `find` and `md5sum`

## Benchmark Datasets

Synthetic datasets are generated using `benchmark/generator.py` based on predefined size distributions (see `SIZE_PROFILES` in the script).
Datasets use a set set size and change the distribution of file sizes and ratio of duplicates to simulate different scenarios.

## Running Benchmarks

To run benchmarks, first build the dedup binary in release mode:

```bash
cargo build --release
```

Then run the benchmark inside the container. A privileged container is needed to flush the cache between runs.

All results in the repository were generated with:

```bash
run-docker.sh --size 10G
```

This will generate datasets of size 10GB for each profile and run all tools on them, saving results to `benchmark/results/`.

There are preset dataset profiles to simulate different scenarios:

- `small-heavy`: mostly small files.
- `mixed`: a more uniform distribution of file sizes.
- `large-heavy`: mostly large files.

## Results

Repository includes pre-generated results for reference in `benchmark/results/`.

There are two sets of results:

- `results/fast_disk/`: benchmarks run on a faster disk with average 1.75 GB/s read/write speeds.
- `results/slow_disk/`: benchmarks run on a slower disk with average 500 MB/s read/write speeds.
