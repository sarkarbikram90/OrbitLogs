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

import { useState } from 'react';
import { useFleetRegistry } from '../hooks/useFleetRegistry';
import { SatelliteTable } from '../components/fleet/SatelliteTable';
import { SatelliteForm } from '../components/fleet/SatelliteForm';
import { GroundStationTable } from '../components/fleet/GroundStationTable';
import { GroundStationForm } from '../components/fleet/GroundStationForm';

export function FleetManagementPage() {
  const [activeSubTab, setActiveSubTab] = useState<'SATELLITES' | 'STATIONS'>('SATELLITES');
  const { sats, stations, message, addSatellite, addStation, deleteSatellite } = useFleetRegistry();

  return (
    <div style={{ padding: '20px', height: '100%', overflowY: 'auto', color: '#f8fafc' }} className="glass-panel">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
        <div>
          <h2 style={{ margin: 0, fontFamily: 'var(--font-display)', color: '#38bdf8', fontSize: '18px' }}>
            ORGANIZATION FLEET & GROUND NETWORK REGISTRY
          </h2>
          <span style={{ fontSize: '11px', color: '#64748b', fontFamily: 'var(--font-mono)' }}>
            REGISTER CUSTOMER SATELLITES & GROUND STATIONS (MULTI-TENANT ISOLATED)
          </span>
        </div>

        <div style={{ display: 'flex', gap: '8px' }}>
          <button
            onClick={() => setActiveSubTab('SATELLITES')}
            style={{
              padding: '6px 12px',
              backgroundColor: activeSubTab === 'SATELLITES' ? 'rgba(56, 189, 248, 0.2)' : 'transparent',
              color: activeSubTab === 'SATELLITES' ? '#38bdf8' : '#94a3b8',
              border: '1px solid rgba(56, 189, 248, 0.4)',
              borderRadius: '4px',
              cursor: 'pointer',
              fontWeight: 'bold',
              fontSize: '11px',
              fontFamily: 'var(--font-display)',
            }}
          >
            SATELLITES ({sats.length})
          </button>
          <button
            onClick={() => setActiveSubTab('STATIONS')}
            style={{
              padding: '6px 12px',
              backgroundColor: activeSubTab === 'STATIONS' ? 'rgba(56, 189, 248, 0.2)' : 'transparent',
              color: activeSubTab === 'STATIONS' ? '#38bdf8' : '#94a3b8',
              border: '1px solid rgba(56, 189, 248, 0.4)',
              borderRadius: '4px',
              cursor: 'pointer',
              fontWeight: 'bold',
              fontSize: '11px',
              fontFamily: 'var(--font-display)',
            }}
          >
            GROUND STATIONS ({stations.length})
          </button>
        </div>
      </div>

      {message && (
        <div
          style={{
            padding: '10px 14px',
            marginBottom: '16px',
            borderRadius: '6px',
            backgroundColor: message.isError ? 'rgba(239, 68, 68, 0.15)' : 'rgba(16, 185, 129, 0.15)',
            border: `1px solid ${message.isError ? '#ef4444' : '#10b981'}`,
            color: message.isError ? '#ef4444' : '#10b981',
            fontSize: '12px',
            fontFamily: 'var(--font-mono)',
          }}
        >
          {message.text}
        </div>
      )}

      {activeSubTab === 'SATELLITES' ? (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 340px', gap: '20px' }}>
          <SatelliteTable sats={sats} onDelete={deleteSatellite} />
          <SatelliteForm onSubmit={addSatellite} />
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 340px', gap: '20px' }}>
          <GroundStationTable stations={stations} />
          <GroundStationForm onSubmit={addStation} />
        </div>
      )}
    </div>
  );
}
