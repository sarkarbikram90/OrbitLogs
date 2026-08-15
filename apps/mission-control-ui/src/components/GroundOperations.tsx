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
import { useState, useEffect } from 'react';

interface GroundStationOps {
  station_id: string;
  name: string;
  antenna_available: boolean;
  rf_utilization: number;
  active_satellite: string | null;
  total_uplinks: number;
  total_downlinks: number;
  failed_uplinks: number;
  failed_downlinks: number;
  missed_contacts: number;
}

const getBackendOpsUrl = () => {
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:8095`;
  }
  return `http://localhost:8095`;
};

export function GroundOperations() {
  const [stations, setStations] = useState<GroundStationOps[]>([]);
  const BACKEND_OPS_URL = getBackendOpsUrl();

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const resp = await fetch(`${BACKEND_OPS_URL}/api/ground-ops`);
        if (resp.ok) {
          const data = await resp.json();
          setStations(data);
        }
      } catch (err) {
        console.error('Failed to fetch ground station operations:', err);
      }
    };

    fetchStatus();
    const interval = setInterval(fetchStatus, 3000);
    return () => clearInterval(interval);
  }, [BACKEND_OPS_URL]);

  return (
    <div className="glass-panel" style={{ height: '100%', padding: '20px', overflowY: 'auto' }}>
      <div className="panel-header" style={{ marginBottom: '20px', background: 'transparent', padding: 0 }}>
        <span>Ground Segment Operations Center</span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: '16px' }}>
        {stations.map((gs) => (
          <div key={gs.station_id} style={{
            padding: '16px',
            backgroundColor: 'rgba(255, 255, 255, 0.02)',
            border: '1px solid var(--border-color)',
            borderRadius: '6px'
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
              <span style={{ fontWeight: 'bold', fontSize: '14px', fontFamily: 'var(--font-display)' }}>{gs.name}</span>
              <span className={`status-indicator ${gs.antenna_available ? 'green' : 'red'}`} />
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Status:</span>
                <span style={{ color: gs.antenna_available ? '#10b981' : '#ef4444', fontWeight: 'bold' }}>
                  {gs.antenna_available ? 'ANTENNA ACTIVE' : 'MAINTENANCE OUTAGE'}
                </span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span style={{ color: 'var(--text-secondary)' }}>RF Utilization:</span>
                <span>{gs.rf_utilization.toFixed(1)}%</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Active Pass:</span>
                <span style={{ color: '#0ea5e9', fontWeight: 'bold' }}>{gs.active_satellite || 'NO SATELLITE'}</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid rgba(255,255,255,0.05)', paddingTop: '8px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Uplink Success/Fail:</span>
                <span>{gs.total_uplinks} / <span style={{ color: '#ef4444' }}>{gs.failed_uplinks}</span></span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Downlink Success/Fail:</span>
                <span>{gs.total_downlinks} / <span style={{ color: '#ef4444' }}>{gs.failed_downlinks}</span></span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Missed Contacts:</span>
                <span style={{ color: '#f59e0b' }}>{gs.missed_contacts}</span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
