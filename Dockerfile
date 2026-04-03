# Stage 1: Build
FROM rust:1.88-bookworm as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    libsqlite3-dev \
    cmake \
    g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/socialkube

# 1. Create a dummy project to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
COPY Cargo.toml Cargo.lock ./

# Limit parallel jobs to prevent OOM during DuckDB build
ENV CARGO_BUILD_JOBS=1

# Use BuildKit cache mount for cargo registry and git
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release

# 2. Copy actual source and build
RUN rm -f target/release/deps/socialkube*
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/socialkube/target/release/socialkube .

RUN mkdir -p /app/logs /app/data

EXPOSE 8080
ENTRYPOINT ["./socialkube"]
