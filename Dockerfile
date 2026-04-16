FROM rust:1.85-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    liblcms2-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build everything (release + bench profile)
RUN cargo build --release
RUN cargo test --release --no-run

# Default: run accuracy tests, then benchmarks
CMD ["sh", "-c", "cargo test --release -- --nocapture && cargo bench"]
