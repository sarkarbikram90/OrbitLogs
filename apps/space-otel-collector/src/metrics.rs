// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use dashmap::DashMap;
use prometheus::{GaugeVec, Opts, Registry};
use std::sync::Arc;
use std::sync::LazyLock;

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

pub static BATTERY_VOLTAGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("battery_voltage", "Spacecraft battery voltage (V)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static BATTERY_CURRENT: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("battery_current", "Spacecraft battery current (A)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static SOLAR_OUTPUT: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("solar_output", "Solar array electrical power output (W)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static CPU_USAGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("cpu_usage", "Flight computer CPU usage in percentage (%)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static MEMORY_USAGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "memory_usage",
            "Flight computer memory usage in percentage (%)",
        ),
        &["satellite_id"],
    )
    .unwrap()
});

pub static FUEL_REMAINING: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("fuel_remaining", "Propellant remaining in percentage (%)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static SIGNAL_STRENGTH: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("signal_strength", "RF signal strength (dBm)"),
        &["satellite_id", "station_id"],
    )
    .unwrap()
});

pub static PACKET_LOSS: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("packet_loss", "Link packet loss in percentage (%)"),
        &["satellite_id", "station_id"],
    )
    .unwrap()
});

pub static LINK_LATENCY: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("link_latency", "Downlink / uplink latency (ms)"),
        &["satellite_id", "station_id"],
    )
    .unwrap()
});

pub struct SatelliteGauges {
    pub battery_voltage: prometheus::Gauge,
    pub battery_current: prometheus::Gauge,
    pub solar_output: prometheus::Gauge,
    pub cpu_usage: prometheus::Gauge,
    pub memory_usage: prometheus::Gauge,
    pub fuel_remaining: prometheus::Gauge,
    pub signal_strength: prometheus::Gauge,
    pub packet_loss: prometheus::Gauge,
    pub link_latency: prometheus::Gauge,
}

impl SatelliteGauges {
    pub fn new(satellite_id: &str) -> Self {
        let station_id = "default-station";
        Self {
            battery_voltage: BATTERY_VOLTAGE.with_label_values(&[satellite_id]),
            battery_current: BATTERY_CURRENT.with_label_values(&[satellite_id]),
            solar_output: SOLAR_OUTPUT.with_label_values(&[satellite_id]),
            cpu_usage: CPU_USAGE.with_label_values(&[satellite_id]),
            memory_usage: MEMORY_USAGE.with_label_values(&[satellite_id]),
            fuel_remaining: FUEL_REMAINING.with_label_values(&[satellite_id]),
            signal_strength: SIGNAL_STRENGTH.with_label_values(&[satellite_id, station_id]),
            packet_loss: PACKET_LOSS.with_label_values(&[satellite_id, station_id]),
            link_latency: LINK_LATENCY.with_label_values(&[satellite_id, station_id]),
        }
    }
}

pub struct GroundStationGauges {
    pub link_latency: prometheus::Gauge,
    pub packet_loss: prometheus::Gauge,
}

impl GroundStationGauges {
    pub fn new(station_id: &str) -> Self {
        Self {
            link_latency: LINK_LATENCY.with_label_values(&["fleet", station_id]),
            packet_loss: PACKET_LOSS.with_label_values(&["fleet", station_id]),
        }
    }
}

pub static SATELLITE_GAUGES_CACHE: LazyLock<DashMap<String, Arc<SatelliteGauges>>> =
    LazyLock::new(DashMap::new);

pub static GS_GAUGES_CACHE: LazyLock<DashMap<String, Arc<GroundStationGauges>>> =
    LazyLock::new(DashMap::new);

pub fn register_metrics() {
    REGISTRY
        .register(Box::new(BATTERY_VOLTAGE.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(BATTERY_CURRENT.clone()))
        .unwrap();
    REGISTRY.register(Box::new(SOLAR_OUTPUT.clone())).unwrap();
    REGISTRY.register(Box::new(CPU_USAGE.clone())).unwrap();
    REGISTRY.register(Box::new(MEMORY_USAGE.clone())).unwrap();
    REGISTRY.register(Box::new(FUEL_REMAINING.clone())).unwrap();
    REGISTRY
        .register(Box::new(SIGNAL_STRENGTH.clone()))
        .unwrap();
    REGISTRY.register(Box::new(PACKET_LOSS.clone())).unwrap();
    REGISTRY.register(Box::new(LINK_LATENCY.clone())).unwrap();
}
