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
import React from 'react';
import type { SatelliteHealth, TelemetryRecord } from '../types';

interface SatelliteDetailProps {
  satelliteId: string | null;
  health: SatelliteHealth | null;
  telemetry: TelemetryRecord | null;
}

export const SatelliteDetail: React.FC<SatelliteDetailProps> = ({
  satelliteId,
  health,
  telemetry,
}) => {
  if (!satelliteId) {
    return (
      <div className="glass-panel" style={{ height: '100%', justifyContent: 'center', alignItems: 'center', padding: '20px' }}>
        <div style={{ color: 'var(--status-blue)', fontSize: '24px', marginBottom: '10px' }}>⚡</div>
        <div style={{ fontFamily: 'var(--font-display)', fontSize: '13px', letterSpacing: '1px', textTransform: 'uppercase', color: 'var(--text-muted)' }}>
          System Diagnostic Offline
        </div>
        <div style={{ color: 'var(--text-secondary)', fontSize: '12px', textAlign: 'center', marginTop: '6px', maxWidth: '250px' }}>
          Select a satellite from the fleet overview to load real-time telemetry streams.
        </div>
      </div>
    );
  }

  // Helper to get status class
  const getStatusClass = (state?: string) => {
    if (state === 'GREEN') return 'green';
    if (state === 'YELLOW') return 'yellow';
    return 'red';
  };

  // Helper to format values
  const fmt = (val?: number, unit = '', decimals = 1) => {
    if (val === undefined || val === null) return 'N/A';
    return `${val.toFixed(decimals)}${unit}`;
  };

  return (
    <div className="glass-panel" style={{ height: '100%' }}>
      <div className="panel-header">
        <span>Spacecraft Diagnostics - {satelliteId}</span>
        <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>
          Mission: {health?.mission_id || telemetry?.mission_id || 'UNKNOWN'}
        </span>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '16px' }}>
        {/* Overall Status banner */}
        <div
          style={{
            padding: '12px',
            borderRadius: '6px',
            background: 'rgba(26, 36, 56, 0.3)',
            borderLeft: `4px solid var(--status-${getStatusClass(health?.overall_state)})`,
            marginBottom: '16px',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '6px' }}>
            <span style={{ fontSize: '12px', fontWeight: '700', color: 'var(--text-secondary)' }}>SYSTEM STATUS SUMMARY</span>
            <span className={`status-badge ${getStatusClass(health?.overall_state)}`}>
              {health?.overall_state || 'UNINITIALIZED'} ({health?.overall_score || 0}%)
            </span>
          </div>
          <p style={{ fontSize: '12px', color: 'var(--text-primary)', margin: 0, lineHeight: '1.4', fontFamily: 'var(--font-mono)' }}>
            {health?.explanation || 'No active diagnostics log collected.'}
          </p>
        </div>

        {/* Subsystems grid */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          
          {/* 1. POWER SYSTEM */}
          <div style={subsystemStyle}>
            <div style={subsystemHeaderStyle}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span className={`status-indicator ${getStatusClass(health?.power.state)}`} />
                <span style={subsystemTitleStyle}>Power Subsystem</span>
              </span>
              <span className={`status-badge ${getStatusClass(health?.power.state)}`}>{health?.power.state || 'GREEN'}</span>
            </div>
            <div style={telemetryRowStyle}>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Battery Voltage</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.battery_voltage, ' V', 2)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Battery Current</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.battery_current, ' A', 2)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Solar Panel Out</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.solar_panel_output, ' W', 1)}</div>
              </div>
            </div>
          </div>

          {/* 2. THERMAL CONTROL SYSTEM */}
          <div style={subsystemStyle}>
            <div style={subsystemHeaderStyle}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span className={`status-indicator ${getStatusClass(health?.thermal.state)}`} />
                <span style={subsystemTitleStyle}>Thermal Subsystem</span>
              </span>
              <span className={`status-badge ${getStatusClass(health?.thermal.state)}`}>{health?.thermal.state || 'GREEN'}</span>
            </div>
            <div style={telemetryRowStyle}>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Core Temperature</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.temperature, ' °C', 1)}</div>
              </div>
            </div>
          </div>

          {/* 3. COMMUNICATIONS SYSTEM */}
          <div style={subsystemStyle}>
            <div style={subsystemHeaderStyle}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span className={`status-indicator ${getStatusClass(health?.communications.state)}`} />
                <span style={subsystemTitleStyle}>Comms Subsystem</span>
              </span>
              <span className={`status-badge ${getStatusClass(health?.communications.state)}`}>{health?.communications.state || 'GREEN'}</span>
            </div>
            <div style={telemetryRowStyle}>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Signal Strength</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.signal_strength, ' dBm', 1)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Packet Loss</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.packet_loss, ' %', 2)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Latency</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.communication_latency, ' ms', 1)}</div>
              </div>
            </div>
          </div>

          {/* 4. PROPULSION SYSTEM */}
          <div style={subsystemStyle}>
            <div style={subsystemHeaderStyle}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span className={`status-indicator ${getStatusClass(health?.propulsion.state)}`} />
                <span style={subsystemTitleStyle}>Propulsion Subsystem</span>
              </span>
              <span className={`status-badge ${getStatusClass(health?.propulsion.state)}`}>{health?.propulsion.state || 'GREEN'}</span>
            </div>
            <div style={telemetryRowStyle}>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Fuel Remaining</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.fuel_remaining, ' %', 1)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Altitude</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.altitude, ' km', 1)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Velocity</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.velocity, ' km/s', 3)}</div>
              </div>
            </div>
          </div>

          {/* 5. ADCS */}
          <div style={subsystemStyle}>
            <div style={subsystemHeaderStyle}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span className={`status-indicator ${getStatusClass(health?.adcs.state)}`} />
                <span style={subsystemTitleStyle}>ADCS Loop</span>
              </span>
              <span className={`status-badge ${getStatusClass(health?.adcs.state)}`}>{health?.adcs.state || 'GREEN'}</span>
            </div>
            <div style={telemetryRowStyle}>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Pitch</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.orientation_pitch, '°', 2)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Roll</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.orientation_roll, '°', 2)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Yaw</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.orientation_yaw, '°', 2)}</div>
              </div>
            </div>
          </div>

          {/* 6. PAYLOAD */}
          <div style={subsystemStyle}>
            <div style={subsystemHeaderStyle}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span className={`status-indicator ${getStatusClass(health?.payload.state)}`} />
                <span style={subsystemTitleStyle}>Payload Instrument</span>
              </span>
              <span className={`status-badge ${getStatusClass(health?.payload.state)}`}>{health?.payload.state || 'GREEN'}</span>
            </div>
            <div style={telemetryRowStyle}>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Active State</div>
                <div style={telemetryValStyle}>
                  {telemetry?.payload_active === undefined ? 'N/A' : telemetry.payload_active ? 'ON' : 'OFF'}
                </div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Power Draw</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.payload_power_draw, ' W', 1)}</div>
              </div>
            </div>
          </div>

          {/* 7. FLIGHT COMPUTER */}
          <div style={subsystemStyle}>
            <div style={subsystemHeaderStyle}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <span className={`status-indicator ${getStatusClass(health?.flight_computer.state)}`} />
                <span style={subsystemTitleStyle}>Flight Computer</span>
              </span>
              <span className={`status-badge ${getStatusClass(health?.flight_computer.state)}`}>{health?.flight_computer.state || 'GREEN'}</span>
            </div>
            <div style={telemetryRowStyle}>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>CPU Load</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.cpu_usage, ' %', 1)}</div>
              </div>
              <div style={telemetryColStyle}>
                <div style={telemetryLabelStyle}>Memory Usage</div>
                <div style={telemetryValStyle}>{fmt(telemetry?.memory_usage, ' %', 1)}</div>
              </div>
            </div>
          </div>

        </div>
      </div>
    </div>
  );
};

// Component style tokens
const subsystemStyle: React.CSSProperties = {
  padding: '12px',
  borderRadius: '6px',
  background: 'rgba(26, 36, 56, 0.15)',
  border: '1px solid rgba(56, 189, 248, 0.08)',
};

const subsystemHeaderStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  marginBottom: '8px',
};

const subsystemTitleStyle: React.CSSProperties = {
  fontFamily: 'var(--font-display)',
  fontSize: '12px',
  fontWeight: '600',
  color: 'var(--text-primary)',
};

const telemetryRowStyle: React.CSSProperties = {
  display: 'flex',
  gap: '10px',
};

const telemetryColStyle: React.CSSProperties = {
  flex: 1,
  background: 'rgba(7, 9, 14, 0.4)',
  padding: '6px 10px',
  borderRadius: '4px',
  border: '1px solid rgba(56, 189, 248, 0.04)',
};

const telemetryLabelStyle: React.CSSProperties = {
  fontSize: '9px',
  color: 'var(--text-muted)',
  textTransform: 'uppercase',
  marginBottom: '2px',
};

const telemetryValStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: '13px',
  fontWeight: '700',
  color: 'var(--text-primary)',
};
