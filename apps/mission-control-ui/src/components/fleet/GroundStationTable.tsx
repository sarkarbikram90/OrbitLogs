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
import { RegisteredStation } from './types';

interface GroundStationTableProps {
  stations: RegisteredStation[];
}

export function GroundStationTable({ stations }: GroundStationTableProps) {
  return (
    <div>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
        <thead>
          <tr style={{ borderBottom: '1px solid var(--border-color)', color: '#38bdf8', textAlign: 'left' }}>
            <th style={{ padding: '8px' }}>Station ID</th>
            <th style={{ padding: '8px' }}>Name</th>
            <th style={{ padding: '8px' }}>Latitude</th>
            <th style={{ padding: '8px' }}>Longitude</th>
            <th style={{ padding: '8px' }}>Altitude</th>
            <th style={{ padding: '8px' }}>Status</th>
          </tr>
        </thead>
        <tbody>
          {stations.length === 0 ? (
            <tr>
              <td colSpan={6} style={{ padding: '20px', textAlign: 'center', color: '#64748b' }}>
                No registered ground stations found for your tenant.
              </td>
            </tr>
          ) : (
            stations.map((st) => (
              <tr key={st.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                <td style={{ padding: '10px 8px', fontWeight: 'bold', color: '#38bdf8' }}>{st.station_id}</td>
                <td style={{ padding: '10px 8px' }}>{st.station_name}</td>
                <td style={{ padding: '10px 8px' }}>{st.latitude.toFixed(4)}°</td>
                <td style={{ padding: '10px 8px' }}>{st.longitude.toFixed(4)}°</td>
                <td style={{ padding: '10px 8px' }}>{st.altitude_m} m</td>
                <td style={{ padding: '10px 8px', color: st.status === 'ONLINE' ? '#10b981' : '#ef4444', fontWeight: 'bold' }}>
                  {st.status}
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}
