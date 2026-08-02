FROM rust:1.97-bookworm

WORKDIR /app

# The shipping artifact is the Rust crate only. Verification Python is not
# copied into this image and is never linked into the crate.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests

RUN cargo build --release \
    && cargo test --lib \
    && cargo test --test original
