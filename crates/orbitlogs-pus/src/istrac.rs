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

//! ISRO ISTRAC (ISRO Telemetry, Tracking and Command Network) Protocol Frame Parser.

use crate::pus_services::{parse_pus_payload, PusPayload, PusSecondaryHeader};
use chrono::Utc;
use orbitlogs_models::{MissionId, SatelliteId, Subsystem, TelemetryRecord};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported ISRO ISTRAC Ground Network Stations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IstracStation {
    Bengaluru,    // ISTRAC-BLR (Byalalu 32m DSN & TTC)
    Lucknow,      // ISTRAC-LKW
    PortBlair,    // ISTRAC-PB
    Sriharikota,  // ISTRAC-SHAR
    Mauritius,    // ISTRAC-MAU
    Bhopal,       // ISTRAC-BPL (Central India relay)
    Brunei,       // ISTRAC-BRN (Southeast Asia coverage)
    Indonesia,    // ISTRAC-IDN (Biak ground station)
    Unknown,
}

impl IstracStation {
    pub fn from_id(id: u8) -> Self {
        match id {
            1 => IstracStation::Bengaluru,
            2 => IstracStation::Lucknow,
            3 => IstracStation::PortBlair,
            4 => IstracStation::Sriharikota,
            5 => IstracStation::Mauritius,
            6 => IstracStation::Bhopal,
            7 => IstracStation::Brunei,
            8 => IstracStation::Indonesia,
            _ => IstracStation::Unknown,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            IstracStation::Bengaluru => "ISTRAC-BLR",
            IstracStation::Lucknow => "ISTRAC-LKW",
            IstracStation::PortBlair => "ISTRAC-PB",
            IstracStation::Sriharikota => "ISTRAC-SHAR",
            IstracStation::Mauritius => "ISTRAC-MAU",
            IstracStation::Bhopal => "ISTRAC-BPL",
            IstracStation::Brunei => "ISTRAC-BRN",
            IstracStation::Indonesia => "ISTRAC-IDN",
            IstracStation::Unknown => "ISTRAC-UNK",
        }
    }
}

/// Pre-configured ground station profile with antenna characteristics,
/// frequency bands, and geographic coordinates (WGS-84).
///
/// Used by contact window predictor and link budget calculator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IstracStationProfile {
    pub station: IstracStation,
    pub name: &'static str,
    /// Antenna diameter in meters
    pub antenna_diameter_m: f64,
    /// Supported uplink/downlink frequency bands
    pub frequency_bands: &'static [&'static str],
    /// WGS-84 latitude in degrees
    pub latitude_deg: f64,
    /// WGS-84 longitude in degrees
    pub longitude_deg: f64,
    /// Altitude above mean sea level in meters
    pub altitude_m: f64,
    /// Minimum elevation mask in degrees
    pub min_elevation_deg: f64,
}

/// Returns the pre-configured profile for all known ISTRAC stations.
pub fn get_istrac_station_profiles() -> Vec<IstracStationProfile> {
    vec![
        IstracStationProfile {
            station: IstracStation::Bengaluru,
            name: "ISTRAC Bengaluru (Byalalu DSN)",
            antenna_diameter_m: 32.0,
            frequency_bands: &["S-band", "X-band", "Ka-band"],
            latitude_deg: 13.0340,
            longitude_deg: 77.5855,
            altitude_m: 900.0,
            min_elevation_deg: 5.0,
        },
        IstracStationProfile {
            station: IstracStation::Sriharikota,
            name: "SHAR Centre Sriharikota",
            antenna_diameter_m: 11.0,
            frequency_bands: &["S-band", "UHF"],
            latitude_deg: 13.7199,
            longitude_deg: 80.2304,
            altitude_m: 5.0,
            min_elevation_deg: 10.0,
        },
        IstracStationProfile {
            station: IstracStation::Lucknow,
            name: "ISTRAC Lucknow",
            antenna_diameter_m: 7.5,
            frequency_bands: &["S-band"],
            latitude_deg: 26.8467,
            longitude_deg: 80.9462,
            altitude_m: 123.0,
            min_elevation_deg: 10.0,
        },
        IstracStationProfile {
            station: IstracStation::PortBlair,
            name: "ISTRAC Port Blair",
            antenna_diameter_m: 7.5,
            frequency_bands: &["S-band"],
            latitude_deg: 11.6234,
            longitude_deg: 92.7265,
            altitude_m: 14.0,
            min_elevation_deg: 10.0,
        },
        IstracStationProfile {
            station: IstracStation::Mauritius,
            name: "ISTRAC Mauritius",
            antenna_diameter_m: 7.5,
            frequency_bands: &["S-band"],
            latitude_deg: -20.2485,
            longitude_deg: 57.5001,
            altitude_m: 430.0,
            min_elevation_deg: 10.0,
        },
        IstracStationProfile {
            station: IstracStation::Bhopal,
            name: "ISTRAC Bhopal",
            antenna_diameter_m: 7.5,
            frequency_bands: &["S-band"],
            latitude_deg: 23.2599,
            longitude_deg: 77.4126,
            altitude_m: 527.0,
            min_elevation_deg: 10.0,
        },
        IstracStationProfile {
            station: IstracStation::Brunei,
            name: "ISTRAC Brunei (SE Asia)",
            antenna_diameter_m: 7.5,
            frequency_bands: &["S-band"],
            latitude_deg: 4.9031,
            longitude_deg: 114.9398,
            altitude_m: 15.0,
            min_elevation_deg: 10.0,
        },
        IstracStationProfile {
            station: IstracStation::Indonesia,
            name: "ISTRAC Biak (Indonesia)",
            antenna_diameter_m: 11.0,
            frequency_bands: &["S-band", "X-band"],
            latitude_deg: -1.1756,
            longitude_deg: 136.1082,
            altitude_m: 42.0,
            min_elevation_deg: 10.0,
        },
    ]
}


/// Parsed ISTRAC CCSDS Transfer Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IstracFrame {
    pub station: IstracStation,
    pub master_frame_count: u8,
    pub virtual_channel_id: u8,
    pub spacecraft_id: u16,
    pub pus_header: PusSecondaryHeader,
    pub pus_payload: PusPayload,
}

pub fn parse_istrac_frame(bytes: &[u8]) -> Result<IstracFrame, String> {
    if bytes.len() < 12 {
        return Err(format!("ISTRAC frame too short: {} bytes", bytes.len()));
    }

    let station = IstracStation::from_id(bytes[0]);
    let master_frame_count = bytes[1];
    let virtual_channel_id = bytes[2] & 0x07;
    let spacecraft_id = u16::from_be_bytes([bytes[3], bytes[4]]);

    let pus_header = PusSecondaryHeader::parse(&bytes[5..10])
        .map_err(|e| format!("PUS header parse error: {}", e))?;

    let pus_payload = parse_pus_payload(
        pus_header.service_type,
        pus_header.subservice_type,
        &bytes[10..],
    )
    .map_err(|e| format!("PUS payload parse error: {}", e))?;

    Ok(IstracFrame {
        station,
        master_frame_count,
        virtual_channel_id,
        spacecraft_id,
        pus_header,
        pus_payload,
    })
}

/// Converts an ISTRAC PUS Housekeeping payload into a canonical OrbitLogs TelemetryRecord
pub fn istrac_frame_to_telemetry_record(
    frame: &IstracFrame,
    satellite_id: &str,
) -> Option<TelemetryRecord> {
    if let PusPayload::Housekeeping(hk) = &frame.pus_payload {
        Some(TelemetryRecord {
            satellite_id: SatelliteId::new(satellite_id),
            subsystem_id: Subsystem::Unified,
            mission_id: MissionId::new("ISRO-MISSION"),
            timestamp: Utc::now(),
            battery_voltage: Some(hk.battery_voltage_v),
            battery_current: Some(hk.battery_current_a),
            solar_panel_output: Some(hk.solar_output_w),
            cpu_usage: Some(25.0),
            memory_usage: Some(40.0),
            temperature: Some(hk.temp_c),
            orientation_pitch: Some(0.0),
            orientation_roll: Some(0.0),
            orientation_yaw: Some(0.0),
            velocity: Some(7.67),
            altitude: Some(520.0),
            fuel_remaining: Some(hk.fuel_pct),
            communication_latency: Some(35.0),
            packet_loss: Some(0.0),
            signal_strength: Some(-65.0),
            payload_power_draw: Some(15.0),
            payload_active: Some(true),
            event_id: Uuid::new_v4(),
        })
    } else {
        None
    }
}
