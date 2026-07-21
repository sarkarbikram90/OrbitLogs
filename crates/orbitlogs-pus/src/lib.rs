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

pub mod istrac;
pub mod pus_services;

pub use istrac::{parse_istrac_frame, IstracFrame, IstracStation};
pub use pus_services::{parse_pus_payload, PusError, PusPayload, PusSecondaryHeader};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pus_header_parse() {
        let bytes = vec![0x10, 0x03, 0x19, 0x00, 0x0A]; // PUS v1, Service 3, Subservice 25, Source 10
        let hdr = PusSecondaryHeader::parse(&bytes).unwrap();
        assert_eq!(hdr.service_type, 3);
        assert_eq!(hdr.subservice_type, 25);
        assert_eq!(hdr.source_id, 10);
    }

    #[test]
    fn test_istrac_frame_parse() {
        // ISTRAC BLR (1), MCF (42), VCID (1), SCID (1001)
        // PUS header: Service 3, Subservice 25, Source 10
        // HK payload: struct_id (1), 28.5V (2850), 2.5A (2500), 120W (1200), 25.0C (250), fuel 95%
        let mut raw = vec![1, 42, 1, 0x03, 0xE9, 0x10, 3, 25, 0, 10];
        raw.extend_from_slice(&(2850u16).to_be_bytes()); // Voltage
        raw.extend_from_slice(&(2500u16).to_be_bytes()); // Current
        raw.extend_from_slice(&(1200u16).to_be_bytes()); // Solar Output
        raw.extend_from_slice(&(250i16).to_be_bytes());  // Temperature
        raw.push(95);                                    // Fuel %

        let frame = parse_istrac_frame(&raw).unwrap();
        assert_eq!(frame.station, IstracStation::Bengaluru);
        assert_eq!(frame.spacecraft_id, 1001);

        if let PusPayload::Housekeeping(hk) = frame.pus_payload {
            assert_eq!(hk.battery_voltage_v, 28.5);
            assert_eq!(hk.battery_current_a, 2.5);
            assert_eq!(hk.solar_output_w, 120.0);
            assert_eq!(hk.temp_c, 25.0);
            assert_eq!(hk.fuel_pct, 95.0);
        } else {
            panic!("Expected payload");
        }
    }
}
