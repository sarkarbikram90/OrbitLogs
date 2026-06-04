# Builder Stage
FROM rust:1.85-alpine AS builder

# Install building system requirements
RUN apk add --no-cache musl-dev gcc make pkgconfig openssl-dev cmake g++

WORKDIR /usr/src/open-spaceops

# Copy workspace configuration and profiles
COPY Cargo.toml ./
COPY apps/telemetry-simulator/Cargo.toml apps/telemetry-simulator/Cargo.toml
COPY apps/telemetry-collector/Cargo.toml apps/telemetry-collector/Cargo.toml
COPY crates/spaceops-models/Cargo.toml crates/spaceops-models/Cargo.toml

# Pre-compile fake sources to cache cargo dependencies
RUN mkdir -p apps/telemetry-simulator/src && echo "fn main() {}" > apps/telemetry-simulator/src/main.rs
RUN mkdir -p apps/telemetry-collector/src && echo "fn main() {}" > apps/telemetry-collector/src/main.rs
RUN mkdir -p crates/spaceops-models/src && touch crates/spaceops-models/src/lib.rs

# Build dependencies only
RUN cargo build --release -p telemetry-collector

# Copy actual source files
COPY crates/spaceops-models/src crates/spaceops-models/src
COPY apps/telemetry-collector/src apps/telemetry-collector/src

# Rebuild actual application
RUN touch crates/spaceops-models/src/lib.rs
RUN touch apps/telemetry-collector/src/main.rs
RUN cargo build --release -p telemetry-collector

# Runner Stage
FROM alpine:3.18
RUN apk add --no-cache ca-certificates libgcc libstdc++ openssl

# Create non-root user for security hardening (UID/GID 10001)
RUN addgroup -g 10001 -S spaceops && adduser -u 10001 -S spaceops -G spaceops

COPY --from=builder /usr/src/open-spaceops/target/release/telemetry-collector /usr/local/bin/telemetry-collector

USER spaceops

ENTRYPOINT ["telemetry-collector"]
