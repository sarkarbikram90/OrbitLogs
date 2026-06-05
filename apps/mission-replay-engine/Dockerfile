# Builder Stage
FROM rust:alpine as builder

# Install build dependencies
RUN apk add --no-cache musl-dev gcc make pkgconfig openssl-dev cmake g++

WORKDIR /usr/src/open-spaceops

# Copy workspace configurations
COPY Cargo.toml Cargo.lock ./
COPY apps/telemetry-simulator/Cargo.toml apps/telemetry-simulator/Cargo.toml
COPY apps/telemetry-collector/Cargo.toml apps/telemetry-collector/Cargo.toml
COPY apps/mission-health-engine/Cargo.toml apps/mission-health-engine/Cargo.toml
COPY apps/ground-station-simulator/Cargo.toml apps/ground-station-simulator/Cargo.toml
COPY apps/mission-replay-engine/Cargo.toml apps/mission-replay-engine/Cargo.toml
COPY crates/spaceops-models/Cargo.toml crates/spaceops-models/Cargo.toml
COPY crates/spaceops-packets/Cargo.toml crates/spaceops-packets/Cargo.toml

# Create dummy source files to pre-compile dependencies
RUN mkdir -p apps/telemetry-simulator/src && echo "fn main() {}" > apps/telemetry-simulator/src/main.rs
RUN mkdir -p apps/telemetry-collector/src && echo "fn main() {}" > apps/telemetry-collector/src/main.rs
RUN mkdir -p apps/mission-health-engine/src && echo "fn main() {}" > apps/mission-health-engine/src/main.rs
RUN mkdir -p apps/ground-station-simulator/src && echo "fn main() {}" > apps/ground-station-simulator/src/main.rs
RUN mkdir -p apps/mission-replay-engine/src && echo "fn main() {}" > apps/mission-replay-engine/src/main.rs
RUN mkdir -p crates/spaceops-models/src && touch crates/spaceops-models/src/lib.rs
RUN mkdir -p crates/spaceops-packets/src && touch crates/spaceops-packets/src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release -p mission-replay-engine

# Copy actual source files
COPY crates/spaceops-models/src crates/spaceops-models/src
COPY crates/spaceops-packets/src crates/spaceops-packets/src
COPY apps/mission-replay-engine/src apps/mission-replay-engine/src

# Rebuild actual application
RUN touch crates/spaceops-models/src/lib.rs
RUN touch crates/spaceops-packets/src/lib.rs
RUN touch apps/mission-replay-engine/src/main.rs
RUN cargo build --release -p mission-replay-engine

# Runner Stage
FROM alpine:3.21
RUN apk add --no-cache ca-certificates libgcc libstdc++ openssl

# Create non-root user for security hardening (UID/GID 10001)
RUN addgroup -g 10001 -S spaceops && adduser -u 10001 -S spaceops -G spaceops

COPY --from=builder /usr/src/open-spaceops/target/release/mission-replay-engine /usr/local/bin/mission-replay-engine

USER spaceops

ENTRYPOINT ["mission-replay-engine"]
