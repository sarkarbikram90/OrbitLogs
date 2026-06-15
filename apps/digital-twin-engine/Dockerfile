# Builder Stage
FROM rust:alpine as builder

# Install building system requirements
RUN apk add --no-cache musl-dev gcc make pkgconfig openssl-dev cmake g++

WORKDIR /usr/src/open-spaceops

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
COPY crates/spaceops-models/Cargo.toml crates/spaceops-models/Cargo.toml
COPY crates/spaceops-packets/Cargo.toml crates/spaceops-packets/Cargo.toml
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
RUN mkdir -p crates/spaceops-models/src && touch crates/spaceops-models/src/lib.rs
RUN mkdir -p crates/spaceops-packets/src && touch crates/spaceops-packets/src/lib.rs
RUN mkdir -p crates/space-schema-registry/src && touch crates/space-schema-registry/src/lib.rs
RUN mkdir -p crates/fleet-health/src && touch crates/fleet-health/src/lib.rs
RUN mkdir -p crates/space-otel-collector/src && touch crates/space-otel-collector/src/lib.rs

# Build dependencies only
RUN cargo build --release -p digital-twin-engine

# Copy actual source files
COPY crates/spaceops-models/src crates/spaceops-models/src
COPY crates/spaceops-packets/src crates/spaceops-packets/src
COPY crates/space-schema-registry/src crates/space-schema-registry/src
COPY crates/fleet-health/src crates/fleet-health/src
COPY crates/space-otel-collector/src crates/space-otel-collector/src
COPY apps/digital-twin-engine/src apps/digital-twin-engine/src

# Rebuild actual application
RUN touch crates/spaceops-models/src/lib.rs
RUN touch crates/spaceops-packets/src/lib.rs
RUN touch crates/space-schema-registry/src/lib.rs
RUN touch crates/fleet-health/src/lib.rs
RUN touch crates/space-otel-collector/src/lib.rs
RUN touch apps/digital-twin-engine/src/main.rs
RUN cargo build --release -p digital-twin-engine

# Runner Stage
FROM alpine:3.21
RUN apk add --no-cache ca-certificates libgcc libstdc++ openssl

# Create non-root user for security hardening (UID/GID 10001)
RUN addgroup -g 10001 -S spaceops && adduser -u 10001 -S spaceops -G spaceops

COPY --from=builder /usr/src/open-spaceops/target/release/digital-twin-engine /usr/local/bin/digital-twin-engine

USER spaceops

ENTRYPOINT ["digital-twin-engine"]
