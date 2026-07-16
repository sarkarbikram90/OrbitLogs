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
import type { GroundStation } from '../types';

interface GroundStationStatusProps {
  groundStations: GroundStation[];
}

export const GroundStationStatus: React.FC<GroundStationStatusProps> = ({ groundStations }) => {
  const getStatusClass = (status: string) => {
    if (status === 'ONLINE') return 'green';
    if (status === 'DEGRADED') return 'yellow';
    return 'red';
  };

  const getStatusColor = (status: string) => {
    if (status === 'ONLINE') return 'var(--status-green)';
    if (status === 'DEGRADED') return 'var(--status-yellow)';
    return 'var(--status-red)';
  };

  // Ensure all 4 ground stations display, using defaults if not loaded yet
  const defaultStations = [
    { id: 'GS-0001', name: 'Svalbard' },
    { id: 'GS-0002', name: 'Troll' },
    { id: 'GS-0003', name: 'Kiruna' },
    { id: 'GS-0004', name: 'McMurdo' }
  ];

  return (
    <div className="glass-panel" style={{ height: '100%' }}>
      <div className="panel-header">
        <span>Ground Segment Network</span>
        <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
          Active Stations: 4
        </span>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '10px' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {defaultStations.map(def => {
            const live = groundStations.find(g => g.station_id === def.id);
            const status = live ? live.status : 'ONLINE';
            const antenna = live ? live.antenna_health : 100.0;
            const weather = live ? live.weather_state : 'Clear';
            const congestion = live ? live.congestion : 0.0;
            const latency = live ? live.latency : 45.0;
            const bandwidth = live ? live.bandwidth : 1000.0;
            const explanation = live ? live.explanation : 'Station operating within nominal limits.';

            const statusClass = getStatusClass(status);
            const statusColor = getStatusColor(status);

            return (
              <div
                key={def.id}
                style={{
                  padding: '10px',
                  borderRadius: '6px',
                  background: 'rgba(26, 36, 56, 0.15)',
                  border: `1px solid ${statusColor}22`,
                  borderLeft: `3px solid ${statusColor}`,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '6px',
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                    <span className={`status-indicator ${statusClass}`} />
                    <span style={{ fontFamily: 'var(--font-display)', fontWeight: '700', fontSize: '12px' }}>
                      {def.name} ({def.id})
                    </span>
                  </div>
                  <span className={`status-badge ${statusClass}`} style={{ fontSize: '9px', padding: '1px 6px' }}>
                    {status}
                  </span>
                </div>

                {/* Micro parameters */}
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr', gap: '4px', fontSize: '10px' }}>
                  <div style={paramBoxStyle}>
                    <div style={labelStyle}>Antenna</div>
                    <div style={valStyle}>{antenna.toFixed(0)}%</div>
                  </div>
                  <div style={paramBoxStyle}>
                    <div style={labelStyle}>Load</div>
                    <div style={valStyle}>{congestion.toFixed(0)}%</div>
                  </div>
                  <div style={paramBoxStyle}>
                    <div style={labelStyle}>Delay</div>
                    <div style={valStyle}>{latency.toFixed(0)}ms</div>
                  </div>
                  <div style={paramBoxStyle}>
                    <div style={labelStyle}>Speed</div>
                    <div style={valStyle}>{bandwidth.toFixed(0)}M</div>
                  </div>
                </div>

                <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)', lineHeight: '1.3' }}>
                  <span style={{ color: 'var(--text-muted)' }}>Weather:</span> {weather} | {explanation}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};

const paramBoxStyle: React.CSSProperties = {
  background: 'rgba(7, 9, 14, 0.4)',
  padding: '4px',
  borderRadius: '3px',
  textAlign: 'center',
};

const labelStyle: React.CSSProperties = {
  fontSize: '8px',
  color: 'var(--text-muted)',
  textTransform: 'uppercase',
  marginBottom: '1px',
};

const valStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontWeight: '700',
  color: 'var(--text-primary)',
};
