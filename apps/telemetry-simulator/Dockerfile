# Builder Stage
FROM rust:1.82-alpine as builder

# Install build dependencies
RUN apk add --no-cache musl-dev gcc make pkgconfig openssl-dev cmake g++

WORKDIR /usr/src/open-spaceops

# Copy workspace configurations
COPY Cargo.toml ./
COPY apps/telemetry-simulator/Cargo.toml apps/telemetry-simulator/Cargo.toml
COPY apps/telemetry-collector/Cargo.toml apps/telemetry-collector/Cargo.toml
COPY crates/spaceops-models/Cargo.toml crates/spaceops-models/Cargo.toml
COPY crates/spaceops-packets/Cargo.toml crates/spaceops-packets/Cargo.toml

# Create dummy source files to pre-compile dependencies
RUN mkdir -p apps/telemetry-simulator/src && echo "fn main() {}" > apps/telemetry-simulator/src/main.rs
RUN mkdir -p apps/telemetry-collector/src && echo "fn main() {}" > apps/telemetry-collector/src/main.rs
RUN mkdir -p crates/spaceops-models/src && touch crates/spaceops-models/src/lib.rs
RUN mkdir -p crates/spaceops-packets/src && touch crates/spaceops-packets/src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release -p telemetry-simulator

# Copy actual source files
COPY crates/spaceops-models/src crates/spaceops-models/src
COPY crates/spaceops-packets/src crates/spaceops-packets/src
COPY apps/telemetry-simulator/src apps/telemetry-simulator/src

# Rebuild actual application
RUN touch crates/spaceops-models/src/lib.rs
RUN touch apps/telemetry-simulator/src/main.rs
RUN cargo build --release -p telemetry-simulator

# Runner Stage
FROM alpine:3.18
RUN apk add --no-cache ca-certificates libgcc libstdc++

# Create non-root user for security hardening (UID/GID 10001)
RUN addgroup -g 10001 -S spaceops && adduser -u 10001 -S spaceops -G spaceops

COPY --from=builder /usr/src/open-spaceops/target/release/telemetry-simulator /usr/local/bin/telemetry-simulator

USER spaceops

ENTRYPOINT ["telemetry-simulator"]
