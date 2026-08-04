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

interface SatelliteFormProps {
  onSubmit: (payload: {
    satellite_id: string;
    satellite_name: string;
    norad_id?: number;
    orbit_type: string;
    tle_line1?: string;
    tle_line2?: string;
  }) => Promise<boolean>;
}

export function SatelliteForm({ onSubmit }: SatelliteFormProps) {
  const [satId, setSatId] = useState('');
  const [satName, setSatName] = useState('');
  const [noradId, setNoradId] = useState('');
  const [orbitType, setOrbitType] = useState('LEO');
  const [tle1, setTle1] = useState('');
  const [tle2, setTle2] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const success = await onSubmit({
      satellite_id: satId,
      satellite_name: satName,
      norad_id: noradId ? parseInt(noradId, 10) : undefined,
      orbit_type: orbitType,
      tle_line1: tle1 || undefined,
      tle_line2: tle2 || undefined,
    });
    if (success) {
      setSatId('');
      setSatName('');
      setNoradId('');
      setTle1('');
      setTle2('');
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
        REGISTER NEW SATELLITE
      </h3>
      <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
        <div>
          <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>SATELLITE ID</label>
          <input
            type="text"
            value={satId}
            onChange={(e) => setSatId(e.target.value)}
            placeholder="e.g. PIXXEL-TD1"
            required
            style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
          />
        </div>
        <div>
          <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>SATELLITE NAME</label>
          <input
            type="text"
            value={satName}
            onChange={(e) => setSatName(e.target.value)}
            placeholder="e.g. Anand (Shakuntala)"
            required
            style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
          />
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
          <div>
            <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>NORAD ID</label>
            <input
              type="number"
              value={noradId}
              onChange={(e) => setNoradId(e.target.value)}
              placeholder="e.g. 54321"
              style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
            />
          </div>
          <div>
            <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>ORBIT TYPE</label>
            <select
              value={orbitType}
              onChange={(e) => setOrbitType(e.target.value)}
              style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '12px' }}
            >
              <option value="LEO">LEO</option>
              <option value="SSO">SSO</option>
              <option value="MEO">MEO</option>
              <option value="GEO">GEO</option>
            </select>
          </div>
        </div>
        <div>
          <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>TLE LINE 1</label>
          <input
            type="text"
            value={tle1}
            onChange={(e) => setTle1(e.target.value)}
            placeholder="1 54321U 22150A..."
            style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '11px', fontFamily: 'var(--font-mono)' }}
          />
        </div>
        <div>
          <label style={{ fontSize: '10px', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>TLE LINE 2</label>
          <input
            type="text"
            value={tle2}
            onChange={(e) => setTle2(e.target.value)}
            placeholder="2 54321  97.5000..."
            style={{ width: '100%', padding: '8px', backgroundColor: '#07090e', border: '1px solid #1e293b', color: '#fff', borderRadius: '4px', fontSize: '11px', fontFamily: 'var(--font-mono)' }}
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
          REGISTER SATELLITE
        </button>
      </form>
    </div>
  );
}
