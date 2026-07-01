# Builder Stage
FROM rust:alpine as builder

# Install building system requirements
RUN apk add --no-cache musl-dev gcc make pkgconfig openssl-dev cmake g++

WORKDIR /usr/src/orbitlogs

# Copy workspace configuration and profiles
COPY Cargo.toml Cargo.lock ./
COPY apps/telemetry-simulator/Cargo.toml apps/telemetry-simulator/Cargo.toml
COPY apps/telemetry-collector/Cargo.toml apps/telemetry-collector/Cargo.toml
COPY apps/mission-health-engine/Cargo.toml apps/mission-health-engine/Cargo.toml
COPY apps/ground-station-simulator/Cargo.toml apps/ground-station-simulator/Cargo.toml
COPY apps/mission-replay-engine/Cargo.toml apps/mission-replay-engine/Cargo.toml
COPY apps/mission-command-service/Cargo.toml apps/mission-command-service/Cargo.toml
COPY apps/digital-twin-engine/Cargo.toml apps/digital-twin-engine/Cargo.toml
COPY apps/mission-timeline/Cargo.toml apps/mission-timeline/Cargo.toml
COPY apps/ai-incident-commander/Cargo.toml apps/ai-incident-commander/Cargo.toml
COPY apps/constellation-simulator/Cargo.toml apps/constellation-simulator/Cargo.toml
COPY apps/orbit-ops-service/Cargo.toml apps/orbit-ops-service/Cargo.toml
COPY crates/orbitlogs-models/Cargo.toml crates/orbitlogs-models/Cargo.toml
COPY crates/orbitlogs-packets/Cargo.toml crates/orbitlogs-packets/Cargo.toml
COPY crates/space-schema-registry/Cargo.toml crates/space-schema-registry/Cargo.toml
COPY crates/fleet-health/Cargo.toml crates/fleet-health/Cargo.toml
COPY crates/space-otel-collector/Cargo.toml crates/space-otel-collector/Cargo.toml

# Pre-compile fake sources to cache cargo dependencies
RUN mkdir -p apps/telemetry-simulator/src && echo "fn main() {}" > apps/telemetry-simulator/src/main.rs
RUN mkdir -p apps/telemetry-collector/src && echo "fn main() {}" > apps/telemetry-collector/src/main.rs
RUN mkdir -p apps/mission-health-engine/src && echo "fn main() {}" > apps/mission-health-engine/src/main.rs
RUN mkdir -p apps/ground-station-simulator/src && echo "fn main() {}" > apps/ground-station-simulator/src/main.rs
RUN mkdir -p apps/mission-replay-engine/src && echo "fn main() {}" > apps/mission-replay-engine/src/main.rs
RUN mkdir -p apps/mission-command-service/src && echo "fn main() {}" > apps/mission-command-service/src/main.rs
RUN mkdir -p apps/digital-twin-engine/src && echo "fn main() {}" > apps/digital-twin-engine/src/main.rs
RUN mkdir -p apps/mission-timeline/src && echo "fn main() {}" > apps/mission-timeline/src/main.rs
RUN mkdir -p apps/ai-incident-commander/src && echo "fn main() {}" > apps/ai-incident-commander/src/main.rs
RUN mkdir -p apps/constellation-simulator/src && echo "fn main() {}" > apps/constellation-simulator/src/main.rs
RUN mkdir -p apps/orbit-ops-service/src && echo "fn main() {}" > apps/orbit-ops-service/src/main.rs
RUN mkdir -p crates/orbitlogs-models/src && touch crates/orbitlogs-models/src/lib.rs
RUN mkdir -p crates/orbitlogs-packets/src && touch crates/orbitlogs-packets/src/lib.rs
RUN mkdir -p crates/space-schema-registry/src && touch crates/space-schema-registry/src/lib.rs
RUN mkdir -p crates/fleet-health/src && touch crates/fleet-health/src/lib.rs
RUN mkdir -p crates/space-otel-collector/src && touch crates/space-otel-collector/src/lib.rs

# Build dependencies only
RUN cargo build --release -p orbit-ops-service

# Copy actual source files
COPY crates/orbitlogs-models/src crates/orbitlogs-models/src
COPY crates/orbitlogs-packets/src crates/orbitlogs-packets/src
COPY crates/space-schema-registry/src crates/space-schema-registry/src
COPY crates/fleet-health/src crates/fleet-health/src
COPY crates/space-otel-collector/src crates/space-otel-collector/src
COPY apps/orbit-ops-service/src apps/orbit-ops-service/src

# Rebuild actual application
RUN touch crates/orbitlogs-models/src/lib.rs
RUN touch crates/orbitlogs-packets/src/lib.rs
RUN touch crates/space-schema-registry/src/lib.rs
RUN touch crates/fleet-health/src/lib.rs
RUN touch crates/space-otel-collector/src/lib.rs
RUN touch apps/orbit-ops-service/src/main.rs
RUN cargo build --release -p orbit-ops-service

# Runner Stage
FROM alpine:3.21
RUN apk add --no-cache ca-certificates libgcc libstdc++ openssl

# Create non-root user for security hardening (UID/GID 10001)
RUN addgroup -g 10001 -S orbitlogs && adduser -u 10001 -S orbitlogs -G orbitlogs

COPY --from=builder /usr/src/orbitlogs/target/release/orbit-ops-service /usr/local/bin/orbit-ops-service

USER orbitlogs

ENTRYPOINT ["orbit-ops-service"]
