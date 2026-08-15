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

interface Pass {
  station: string;
  aos: string;
  los: string;
  duration: number;
  max_elevation: number;
}

interface PassScheduleProps {
  selectedSatId: string | null;
}

const getBackendOpsUrl = () => {
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:8095`;
  }
  return `http://localhost:8095`;
};

export function PassSchedule({ selectedSatId }: PassScheduleProps) {
  const [passes, setPasses] = useState<Pass[]>([]);
  const [loading, setLoading] = useState(false);
  const BACKEND_OPS_URL = getBackendOpsUrl();

  useEffect(() => {
    if (!selectedSatId) return;

    const fetchPasses = async () => {
      setLoading(true);
      try {
        const resp = await fetch(`${BACKEND_OPS_URL}/api/orbit/${selectedSatId}/passes`);
        if (resp.ok) {
          const data = (await resp.json()) as Pass[];
          setPasses(data);
        }
      } catch (err) {
        console.error('Failed to fetch pass schedule:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchPasses();
    const interval = setInterval(fetchPasses, 10000);
    return () => clearInterval(interval);
  }, [selectedSatId, BACKEND_OPS_URL]);

  return (
    <div className="glass-panel" style={{ height: '100%', padding: '20px', overflowY: 'auto' }}>
      <div className="panel-header" style={{ marginBottom: '16px', background: 'transparent', padding: 0 }}>
        <span>Upcoming Ground Contact Windows (24 Hours) - {selectedSatId}</span>
      </div>

      {loading && passes.length === 0 ? (
        <div style={{ color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}>Calculating Keplerian orbits...</div>
      ) : (
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid var(--border-color)', color: '#38bdf8', textAlign: 'left' }}>
                <th style={{ padding: '8px' }}>Station</th>
                <th style={{ padding: '8px' }}>AOS (UTC)</th>
                <th style={{ padding: '8px' }}>LOS (UTC)</th>
                <th style={{ padding: '8px' }}>Duration</th>
                <th style={{ padding: '8px' }}>Max Elevation</th>
              </tr>
            </thead>
            <tbody>
              {passes.map((pass, i) => (
                <tr key={i} style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.05)' }}>
                  <td style={{ padding: '12px 8px', fontWeight: 'bold' }}>{pass.station}</td>
                  <td style={{ padding: '12px 8px' }}>{new Date(pass.aos).toISOString().replace('T', ' ').substring(0, 19)}</td>
                  <td style={{ padding: '12px 8px' }}>{new Date(pass.los).toISOString().replace('T', ' ').substring(0, 19)}</td>
                  <td style={{ padding: '12px 8px', color: '#10b981' }}>{pass.duration.toFixed(1)} mins</td>
                  <td style={{ padding: '12px 8px', color: '#f59e0b' }}>{pass.max_elevation.toFixed(1)}°</td>
                </tr>
              ))}
              {passes.length === 0 && (
                <tr>
                  <td colSpan={5} style={{ padding: '20px 8px', textAlign: 'center', color: 'var(--text-muted)' }}>
                    No passes predicted in current orbital epoch.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
