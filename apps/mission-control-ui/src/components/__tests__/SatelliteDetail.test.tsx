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

import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { SatelliteDetail } from '../SatelliteDetail';
import type { TelemetryRecord } from '../../types';

const mockTelemetry: TelemetryRecord = {
  satellite_id: 'SAT-0001',
  subsystem_id: 'Power',
  mission_id: 'STARLINK-MISSION',
  timestamp: new Date().toISOString(),
  battery_voltage: 14.2,
  battery_current: 2.1,
  solar_panel_output: 125.0,
  temperature: 24.5,
  cpu_usage: 18.4,
  memory_usage: 35.0,
};

describe('SatelliteDetail Component', () => {
  it('renders offline diagnostic state when no satellite is selected', () => {
    render(<SatelliteDetail satelliteId={null} health={null} telemetry={null} />);
    expect(screen.getByText(/System Diagnostic Offline/i)).toBeInTheDocument();
  });

  it('renders live subsystem telemetry and values when satellite is selected', () => {
    render(<SatelliteDetail satelliteId="SAT-0001" health={null} telemetry={mockTelemetry} />);

    expect(screen.getByText(/Spacecraft Diagnostics - SAT-0001/i)).toBeInTheDocument();
    expect(screen.getByText('14.20 V')).toBeInTheDocument();
    expect(screen.getByText('24.5 °C')).toBeInTheDocument();
    expect(screen.getByText('18.4 %')).toBeInTheDocument();
  });
});
