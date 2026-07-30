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
import { useFleetStore } from '../stores/fleetStore';

export const TelemetryHud: React.FC = () => {
  const { satellites, selectedSatId } = useFleetStore();

  const selectedSat = satellites.find((s) => s.id === selectedSatId) || satellites[0];

  return (
    <div style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: 5 }}>
      {/* Scanline CRT overlay */}
      <div className="scanline-overlay" />

      {/* Top Left Corner Crosshair Reticle */}
      <div style={{ position: 'absolute', top: 12, left: 16, display: 'flex', gap: '8px', alignItems: 'center' }}>
        <div style={{ width: 12, height: 12, borderTop: '2px solid var(--cyber-cyan)', borderLeft: '2px solid var(--cyber-cyan)' }} />
        <span style={{ fontSize: '9px', fontFamily: 'var(--font-mono)', color: 'var(--cyber-cyan)', letterSpacing: '1px', opacity: 0.8 }}>
          GRID-LOC: 77°42'N 14°54'E (SVALBARD DSN)
        </span>
      </div>

      {/* Top Right Corner Data Stream Indicator */}
      <div style={{ position: 'absolute', top: 12, right: 16, display: 'flex', gap: '12px', alignItems: 'center' }}>
        <span style={{ fontSize: '9px', fontFamily: 'var(--font-mono)', color: 'var(--status-green)', letterSpacing: '1px' }}>
          INGEST RATE: 1,420 PKTS/SEC • STREAM LOCKED
        </span>
        <div style={{ width: 12, height: 12, borderTop: '2px solid var(--cyber-cyan)', borderRight: '2px solid var(--cyber-cyan)' }} />
      </div>

      {/* Bottom Floating Telemetry Overlay if satellite selected */}
      {selectedSat && (
        <div
          style={{
            position: 'absolute',
            bottom: 16,
            left: '50%',
            transform: 'translateX(-50%)',
            backgroundColor: 'rgba(6, 12, 24, 0.85)',
            backdropFilter: 'blur(12px)',
            border: '1px solid var(--border-color)',
            borderRadius: '20px',
            padding: '6px 18px',
            display: 'flex',
            gap: '16px',
            alignItems: 'center',
            boxShadow: '0 0 20px rgba(0, 240, 255, 0.15)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <span className={`status-indicator ${selectedSat.state === 'GREEN' ? 'green' : 'red'}`} />
            <span style={{ fontFamily: 'var(--font-display)', fontWeight: 800, fontSize: '11px', color: 'var(--cyber-cyan)' }}>
              {selectedSat.id}
            </span>
          </div>
          <span style={{ color: 'var(--border-color)' }}>|</span>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: '10px', color: 'var(--text-secondary)' }}>
            ALTITUDE: 524.8 KM
          </span>
          <span style={{ color: 'var(--border-color)' }}>|</span>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: '10px', color: 'var(--text-secondary)' }}>
            VELOCITY: 7.67 KM/S
          </span>
          <span style={{ color: 'var(--border-color)' }}>|</span>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: '10px', color: 'var(--status-green)' }}>
            HEALTH: {selectedSat.score}%
          </span>
        </div>
      )}
    </div>
  );
};
