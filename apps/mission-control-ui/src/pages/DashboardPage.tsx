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

import { useFleetStore } from '../stores/fleetStore';
import { useTelemetryStore } from '../stores/telemetryStore';
import { useReplayStore } from '../stores/replayStore';
import { FleetOverview } from '../components/FleetOverview';
import { GroundStationStatus } from '../components/GroundStationStatus';
import { OrbitViewer } from '../components/OrbitViewer';
import { ReplayControls } from '../components/ReplayControls';
import { SatelliteDetail } from '../components/SatelliteDetail';
import { IncidentPanel } from '../components/IncidentPanel';

interface DashboardPageProps {
  onToggleMode: (newMode: 'LIVE' | 'REPLAY') => void;
  onCreateSession: (satId: string | null, start: string, end: string, speed: number) => void;
  onControl: (action: string, position?: string, speed?: number) => void;
}

export function DashboardPage({ onToggleMode, onCreateSession, onControl }: DashboardPageProps) {
  const { satellites, groundStations, selectedSatId, mode, dashboardMode, setSelectedSatId } = useFleetStore();
  const { telemetry, health, events } = useTelemetryStore();
  const { replaySession } = useReplayStore();

  if (dashboardMode === 'EXECUTIVE') {
    const greenCount = satellites.filter((s) => s.state === 'GREEN').length;
    const readiness = ((greenCount / satellites.length) * 100).toFixed(0);
    const redCount = satellites.filter((s) => s.state === 'RED').length;
    const yellowCount = satellites.filter((s) => s.state === 'YELLOW').length;
    const riskLevel = redCount > 0 ? 'HIGH' : yellowCount > 0 ? 'ELEVATED' : 'LOW';

    return (
      <div
        style={{ display: 'grid', gridTemplateRows: 'auto 1fr', gap: '20px', height: '100%', width: '100%', color: '#f8fafc', padding: '20px', overflowY: 'auto' }}
        className="glass-panel"
      >
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '16px' }}>
          <div style={{ padding: '16px', border: '1px solid var(--border-color)', borderRadius: '6px', backgroundColor: 'rgba(255,255,255,0.01)' }}>
            <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-display)' }}>CONSTELLATION READINESS</div>
            <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#10b981', marginTop: '8px', fontFamily: 'var(--font-mono)' }}>{readiness}%</div>
          </div>
          <div style={{ padding: '16px', border: '1px solid var(--border-color)', borderRadius: '6px', backgroundColor: 'rgba(255,255,255,0.01)' }}>
            <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-display)' }}>ACTIVE ORBITAL ASSETS</div>
            <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#0ea5e9', marginTop: '8px', fontFamily: 'var(--font-mono)' }}>{satellites.length}</div>
          </div>
          <div style={{ padding: '16px', border: '1px solid var(--border-color)', borderRadius: '6px', backgroundColor: 'rgba(255,255,255,0.01)' }}>
            <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-display)' }}>ACTIVE GROUND STATIONS</div>
            <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#f59e0b', marginTop: '8px', fontFamily: 'var(--font-mono)' }}>{groundStations.length}</div>
          </div>
          <div style={{ padding: '16px', border: '1px solid var(--border-color)', borderRadius: '6px', backgroundColor: 'rgba(255,255,255,0.01)' }}>
            <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-display)' }}>FLEET RISK SCORE</div>
            <div
              style={{
                fontSize: '24px',
                fontWeight: 'bold',
                color: redCount > 0 ? '#ef4444' : '#10b981',
                marginTop: '8px',
                fontFamily: 'var(--font-mono)',
              }}
            >
              {riskLevel}
            </div>
          </div>
        </div>
        <div style={{ marginTop: '20px' }}>
          <div style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8', marginBottom: '12px' }}>
            CONSTELLATION FLEET INVENTORY
          </div>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid var(--border-color)', color: '#38bdf8', textAlign: 'left' }}>
                <th style={{ padding: '8px' }}>Spacecraft ID</th>
                <th style={{ padding: '8px' }}>Operational Mode</th>
                <th style={{ padding: '8px' }}>Subsystem Health Score</th>
                <th style={{ padding: '8px' }}>Telemetry Link Status</th>
              </tr>
            </thead>
            <tbody>
              {satellites.map((sat) => (
                <tr key={sat.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                  <td style={{ padding: '12px 8px', fontWeight: 'bold' }}>{sat.id}</td>
                  <td style={{ padding: '12px 8px' }}>{sat.state === 'GREEN' ? 'NOMINAL' : sat.state === 'YELLOW' ? 'DEGRADED' : 'SAFE MODE'}</td>
                  <td style={{ padding: '12px 8px', color: sat.state === 'GREEN' ? '#10b981' : sat.state === 'YELLOW' ? '#f59e0b' : '#ef4444' }}>
                    {sat.score}/100
                  </td>
                  <td style={{ padding: '12px 8px' }}>
                    <span className={`status-indicator ${sat.state === 'GREEN' ? 'green' : sat.state === 'YELLOW' ? 'yellow' : 'red'}`} style={{ marginRight: '6px' }} />
                    {sat.state === 'GREEN' ? 'Telemetry Locked' : 'Degraded Signal'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  }

  // Operator view grid
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '320px 1fr 340px', gap: '16px', height: '100%', overflow: 'hidden' }}>
      {/* Left Column: Fleet Listing & Ground Stations */}
      <div style={{ display: 'grid', gridTemplateRows: '1fr 260px', gap: '16px', height: '100%', overflow: 'hidden' }}>
        <FleetOverview satellites={satellites} selectedSatId={selectedSatId} onSelectSat={(id) => setSelectedSatId(id)} />
        <GroundStationStatus groundStations={groundStations} />
      </div>

      {/* Center Column: 3D Visualization & Playback Timelines */}
      <div className="center-column">
        <OrbitViewer satellites={satellites} groundStations={groundStations} selectedSatId={selectedSatId} />
        <ReplayControls
          mode={mode}
          session={replaySession}
          satellites={satellites}
          onToggleMode={onToggleMode}
          onCreateSession={onCreateSession}
          onControl={onControl}
        />
      </div>

      {/* Right Column: Diagnostic Cards & Events Log */}
      <div style={{ display: 'grid', gridTemplateRows: '1fr 280px', gap: '16px', height: '100%', overflow: 'hidden' }}>
        <SatelliteDetail satelliteId={selectedSatId} health={health} telemetry={telemetry} />
        <IncidentPanel events={events} />
      </div>
    </div>
  );
}
