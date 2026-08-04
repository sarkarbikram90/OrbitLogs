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

interface GroundStationFormProps {
  onSubmit: (payload: {
    station_id: string;
    station_name: string;
    latitude: number;
    longitude: number;
    altitude_m: number;
  }) => Promise<boolean>;
}

export function GroundStationForm({ onSubmit }: GroundStationFormProps) {
  const [stnId, setStnId] = useState('');
  const [stnName, setStnName] = useState('');
  const [lat, setLat] = useState('');
  const [lon, setLon] = useState('');
  const [altM, setAltM] = useState('500');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const success = await onSubmit({
      station_id: stnId,
      station_name: stnName,
      latitude: parseFloat(lat),
      longitude: parseFloat(lon),
      altitude_m: parseFloat(altM),
    });
    if (success) {
      setStnId('');
      setStnName('');
      setLat('');
      setLon('');
    }
  };

  return (
    <div
      style={{
        padding: '16px',
        border: '1px solid rgba(56, 189, 248, 0.2)',
        borderRadius: '8px',
        backgroundColor: 'rgba(4, 6, 10, 0.5)',
      }}
    >
      <h3 style={{ margin: '0 0 14px 0', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8' }}>
        REGISTER GROUND STATION
      </h3>
      <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
        <div>
          <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>STATION ID</label>
          <input
            type="text"
            value={stnId}
            onChange={(e) => setStnId(e.target.value)}
            placeholder="e.g. GS-BLR-01"
            required
            style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
          />
        </div>
        <div>
          <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>STATION NAME</label>
          <input
            type="text"
            value={stnName}
            onChange={(e) => setStnName(e.target.value)}
            placeholder="e.g. Bengaluru Primary GS"
            required
            style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
          />
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
          <div>
            <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>LATITUDE (°)</label>
            <input
              type="number"
              step="any"
              value={lat}
              onChange={(e) => setLat(e.target.value)}
              placeholder="e.g. 13.0300"
              required
              style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
            />
          </div>
          <div>
            <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>LONGITUDE (°)</label>
            <input
              type="number"
              step="any"
              value={lon}
              onChange={(e) => setLon(e.target.value)}
              placeholder="e.g. 77.5600"
              required
              style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
            />
          </div>
        </div>
        <div>
          <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>ALTITUDE (METERS)</label>
          <input
            type="number"
            value={altM}
            onChange={(e) => setAltM(e.target.value)}
            placeholder="900"
            style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
          />
        </div>
        <button
          type="submit"
          style={{
            marginTop: '6px',
            padding: '10px',
            backgroundColor: '#38bdf8',
            color: '#020408',
            border: 'none',
            borderRadius: '4px',
            fontWeight: 'bold',
            fontSize: '11px',
            fontFamily: 'var(--font-display)',
            cursor: 'pointer',
          }}
        >
          REGISTER GROUND STATION
        </button>
      </form>
    </div>
  );
}
