# Builder Stage
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev gcc make pkgconfig openssl-dev cmake g++

WORKDIR /usr/src/orbitlogs

COPY Cargo.toml Cargo.lock ./

# Copy ALL workspace member Cargo.toml files
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
COPY apps/space-otel-collector/Cargo.toml apps/space-otel-collector/Cargo.toml
COPY apps/contact-window-predictor/Cargo.toml apps/contact-window-predictor/Cargo.toml
COPY apps/constellation-scheduler/Cargo.toml apps/constellation-scheduler/Cargo.toml
COPY apps/orbit-determination-service/Cargo.toml apps/orbit-determination-service/Cargo.toml
COPY apps/autonomous-mission-planning/Cargo.toml apps/autonomous-mission-planning/Cargo.toml
COPY apps/telemetry-metering-service/Cargo.toml apps/telemetry-metering-service/Cargo.toml
COPY apps/alert-notification-service/Cargo.toml apps/alert-notification-service/Cargo.toml
COPY crates/orbitlogs-models/Cargo.toml crates/orbitlogs-models/Cargo.toml
COPY crates/orbitlogs-packets/Cargo.toml crates/orbitlogs-packets/Cargo.toml
COPY crates/space-schema-registry/Cargo.toml crates/space-schema-registry/Cargo.toml
COPY crates/fleet-health/Cargo.toml crates/fleet-health/Cargo.toml
COPY crates/orbitlogs-astrodynamics/Cargo.toml crates/orbitlogs-astrodynamics/Cargo.toml
COPY crates/orbitlogs-propulsion/Cargo.toml crates/orbitlogs-propulsion/Cargo.toml
COPY crates/orbitlogs-test-utils/Cargo.toml crates/orbitlogs-test-utils/Cargo.toml
COPY crates/orbitlogs-pus/Cargo.toml crates/orbitlogs-pus/Cargo.toml

# Create dummy sources for all workspace members
RUN set -e; \
    for app in telemetry-simulator telemetry-collector mission-health-engine \
               ground-station-simulator mission-replay-engine mission-command-service \
               digital-twin-engine mission-timeline ai-incident-commander \
               constellation-simulator orbit-ops-service space-otel-collector \
               contact-window-predictor constellation-scheduler orbit-determination-service \
               autonomous-mission-planning telemetry-metering-service alert-notification-service; do \
        mkdir -p "apps/${app}/src" && echo 'fn main() {}' > "apps/${app}/src/main.rs"; \
    done; \
    for crt in orbitlogs-models orbitlogs-packets space-schema-registry fleet-health \
               orbitlogs-astrodynamics orbitlogs-propulsion orbitlogs-test-utils orbitlogs-pus; do \
        mkdir -p "crates/${crt}/src" && touch "crates/${crt}/src/lib.rs"; \
    done

RUN cargo build --release -p mission-health-engine

# Copy actual source files
COPY crates/orbitlogs-models/src crates/orbitlogs-models/src
COPY crates/orbitlogs-packets/src crates/orbitlogs-packets/src
COPY crates/space-schema-registry/src crates/space-schema-registry/src
COPY crates/fleet-health/src crates/fleet-health/src
COPY apps/mission-health-engine/src apps/mission-health-engine/src

RUN touch crates/orbitlogs-models/src/lib.rs \
    && touch crates/orbitlogs-packets/src/lib.rs \
    && touch crates/space-schema-registry/src/lib.rs \
    && touch crates/fleet-health/src/lib.rs \
    && touch apps/mission-health-engine/src/main.rs \
    && cargo build --release -p mission-health-engine

# Runner Stage
FROM alpine:3.21
RUN apk add --no-cache ca-certificates libgcc libstdc++ openssl

RUN addgroup -g 10001 -S orbitlogs && adduser -u 10001 -S orbitlogs -G orbitlogs

COPY --from=builder /usr/src/orbitlogs/target/release/mission-health-engine /usr/local/bin/mission-health-engine

USER orbitlogs

EXPOSE 8082

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8082/health || exit 1

ENTRYPOINT ["mission-health-engine"]
