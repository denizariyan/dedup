FROM rust:bookworm AS rust-builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

RUN cargo install fclones --locked

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    time \
    python3 \
    python3-pip \
    python3-venv \
    fdupes \
    rdfind \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /build/target/release/dedup /usr/local/bin/dedup
COPY --from=rust-builder /usr/local/cargo/bin/fclones /usr/local/bin/fclones

WORKDIR /benchmark
COPY benchmark/pyproject.toml benchmark/poetry.lock* ./

# Skip poetry in container
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"
RUN pip install --no-cache-dir matplotlib tqdm numpy

COPY benchmark/*.py ./

ENTRYPOINT ["python3", "benchmark.py", "run"]
