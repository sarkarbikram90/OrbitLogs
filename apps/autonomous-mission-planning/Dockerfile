# Builder Stage
FROM rust:alpine as builder

RUN apk add --no-cache musl-dev gcc make pkgconfig openssl-dev cmake g++

WORKDIR /usr/src/orbitlogs

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
COPY crates/orbitlogs-models/Cargo.toml crates/orbitlogs-models/Cargo.toml
COPY crates/orbitlogs-packets/Cargo.toml crates/orbitlogs-packets/Cargo.toml
COPY crates/space-schema-registry/Cargo.toml crates/space-schema-registry/Cargo.toml
COPY crates/fleet-health/Cargo.toml crates/fleet-health/Cargo.toml
COPY apps/space-otel-collector/Cargo.toml apps/space-otel-collector/Cargo.toml
COPY crates/orbitlogs-astrodynamics/Cargo.toml crates/orbitlogs-astrodynamics/Cargo.toml
COPY crates/orbitlogs-propulsion/Cargo.toml crates/orbitlogs-propulsion/Cargo.toml
COPY apps/contact-window-predictor/Cargo.toml apps/contact-window-predictor/Cargo.toml
COPY apps/constellation-scheduler/Cargo.toml apps/constellation-scheduler/Cargo.toml
COPY apps/orbit-determination-service/Cargo.toml apps/orbit-determination-service/Cargo.toml
COPY apps/autonomous-mission-planning/Cargo.toml apps/autonomous-mission-planning/Cargo.toml
COPY crates/orbitlogs-test-utils/Cargo.toml crates/orbitlogs-test-utils/Cargo.toml

# Pre-compile dummy source to cache dependencies
RUN mkdir -p apps/autonomous-mission-planning/src && echo "fn main() {}" > apps/autonomous-mission-planning/src/main.rs

RUN cargo build --release -p autonomous-mission-planning

# Copy actual source files
COPY apps/autonomous-mission-planning/src apps/autonomous-mission-planning/src

# Rebuild actual application
RUN touch apps/autonomous-mission-planning/src/main.rs
RUN cargo build --release -p autonomous-mission-planning

# Runner Stage
FROM alpine:3.21
RUN apk add --no-cache ca-certificates libgcc libstdc++ openssl

# Create non-root user (UID/GID 10001)
RUN addgroup -g 10001 -S orbitlogs && adduser -u 10001 -S orbitlogs -G orbitlogs

COPY --from=builder /usr/src/orbitlogs/target/release/autonomous-mission-planning /usr/local/bin/autonomous-mission-planning

USER orbitlogs

ENTRYPOINT ["autonomous-mission-planning"]
