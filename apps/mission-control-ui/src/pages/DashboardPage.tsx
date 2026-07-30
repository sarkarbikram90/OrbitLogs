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
    const readiness = satellites.length > 0 ? ((greenCount / satellites.length) * 100).toFixed(1) : '100.0';
    const redCount = satellites.filter((s) => s.state === 'RED').length;
    const yellowCount = satellites.filter((s) => s.state === 'YELLOW').length;
    const riskLevel = redCount > 0 ? 'HIGH RISK' : yellowCount > 0 ? 'ELEVATED' : 'NOMINAL';

    return (
      <div
        style={{ display: 'grid', gridTemplateRows: 'auto 1fr', gap: '20px', height: '100%', width: '100%', color: '#f8fafc', padding: '20px', overflowY: 'auto' }}
        className="glass-panel tech-corner-panel"
      >
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '16px' }}>
          <div style={{ padding: '20px', border: '1px solid var(--border-color)', borderRadius: '8px', backgroundColor: 'rgba(6, 12, 24, 0.7)', backdropFilter: 'blur(12px)' }}>
            <div style={{ fontSize: '10px', color: 'var(--text-muted)', fontFamily: 'var(--font-display)', letterSpacing: '1px' }}>CONSTELLATION READINESS</div>
            <div style={{ fontSize: '28px', fontWeight: 900, color: 'var(--status-green)', marginTop: '8px', fontFamily: 'var(--font-mono)', textShadow: '0 0 12px var(--status-green-glow)' }}>{readiness}%</div>
            <div style={{ fontSize: '9px', color: 'var(--text-secondary)', marginTop: '4px' }}>SLAs Meeting 99.999% Target</div>
          </div>
          <div style={{ padding: '20px', border: '1px solid var(--border-color)', borderRadius: '8px', backgroundColor: 'rgba(6, 12, 24, 0.7)', backdropFilter: 'blur(12px)' }}>
            <div style={{ fontSize: '10px', color: 'var(--text-muted)', fontFamily: 'var(--font-display)', letterSpacing: '1px' }}>ACTIVE ORBITAL ASSETS</div>
            <div style={{ fontSize: '28px', fontWeight: 900, color: 'var(--cyber-cyan)', marginTop: '8px', fontFamily: 'var(--font-mono)', textShadow: '0 0 12px var(--cyber-cyan-glow)' }}>{satellites.length}</div>
            <div style={{ fontSize: '9px', color: 'var(--text-secondary)', marginTop: '4px' }}>LEO & MEO Constellation Satellites</div>
          </div>
          <div style={{ padding: '20px', border: '1px solid var(--border-color)', borderRadius: '8px', backgroundColor: 'rgba(6, 12, 24, 0.7)', backdropFilter: 'blur(12px)' }}>
            <div style={{ fontSize: '10px', color: 'var(--text-muted)', fontFamily: 'var(--font-display)', letterSpacing: '1px' }}>GROUND STATIONS ONLINE</div>
            <div style={{ fontSize: '28px', fontWeight: 900, color: 'var(--status-yellow)', marginTop: '8px', fontFamily: 'var(--font-mono)', textShadow: '0 0 12px var(--status-yellow-glow)' }}>{groundStations.length}</div>
            <div style={{ fontSize: '9px', color: 'var(--text-secondary)', marginTop: '4px' }}>ISTRAC & Svalbard Network Active</div>
          </div>
          <div style={{ padding: '20px', border: '1px solid var(--border-color)', borderRadius: '8px', backgroundColor: 'rgba(6, 12, 24, 0.7)', backdropFilter: 'blur(12px)' }}>
            <div style={{ fontSize: '10px', color: 'var(--text-muted)', fontFamily: 'var(--font-display)', letterSpacing: '1px' }}>FLEET RISK MATRIX</div>
            <div
              style={{
                fontSize: '28px',
                fontWeight: 900,
                color: redCount > 0 ? 'var(--status-red)' : 'var(--status-green)',
                marginTop: '8px',
                fontFamily: 'var(--font-mono)',
                textShadow: redCount > 0 ? '0 0 12px var(--status-red-glow)' : '0 0 12px var(--status-green-glow)',
              }}
            >
              {riskLevel}
            </div>
            <div style={{ fontSize: '9px', color: 'var(--text-secondary)', marginTop: '4px' }}>Autonomous Collision Avoidance Ready</div>
          </div>
        </div>

        <div style={{ marginTop: '16px' }}>
          <div style={{ fontWeight: 800, fontSize: '14px', fontFamily: 'var(--font-display)', color: 'var(--cyber-cyan)', marginBottom: '16px', letterSpacing: '1.5px', display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span>🛰</span>
            <span>CONSTELLATION FLEET REAL-TIME INVENTORY</span>
          </div>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid var(--border-color)', color: 'var(--cyber-cyan)', textAlign: 'left', backgroundColor: 'rgba(0, 240, 255, 0.05)' }}>
                <th style={{ padding: '12px 10px' }}>SPACECRAFT ID</th>
                <th style={{ padding: '12px 10px' }}>OPERATIONAL STATE</th>
                <th style={{ padding: '12px 10px' }}>HEALTH INDEX SCORE</th>
                <th style={{ padding: '12px 10px' }}>TELEMETRY UPLINK STATUS</th>
              </tr>
            </thead>
            <tbody>
              {satellites.map((sat) => (
                <tr key={sat.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)', transition: 'background 0.2s ease' }}>
                  <td style={{ padding: '14px 10px', fontWeight: 'bold', color: '#fff' }}>{sat.id}</td>
                  <td style={{ padding: '14px 10px' }}>{sat.state === 'GREEN' ? 'NOMINAL (MODE 1)' : sat.state === 'YELLOW' ? 'DEGRADED (MODE 2)' : 'SAFE MODE (MODE 3)'}</td>
                  <td style={{ padding: '14px 10px', fontWeight: 'bold', color: sat.state === 'GREEN' ? 'var(--status-green)' : sat.state === 'YELLOW' ? 'var(--status-yellow)' : 'var(--status-red)' }}>
                    {sat.score}/100
                  </td>
                  <td style={{ padding: '14px 10px' }}>
                    <span className={`status-indicator ${sat.state === 'GREEN' ? 'green' : sat.state === 'YELLOW' ? 'yellow' : 'red'}`} style={{ marginRight: '8px' }} />
                    {sat.state === 'GREEN' ? '100% Locked (Svalbard)' : 'Degraded Signal'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  }

  // Operator view grid layout
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
