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
import { useAuthStore } from '../stores/authStore';
import { API_CONFIG } from '../config';

interface RegisteredSat {
  id: string;
  satellite_id: string;
  satellite_name: string;
  norad_id?: number;
  orbit_type: string;
  tle_line1?: string;
  tle_line2?: string;
  status: string;
}

interface RegisteredStation {
  id: string;
  station_id: string;
  station_name: string;
  latitude: number;
  longitude: number;
  altitude_m: number;
  antenna_type?: string;
  frequency_band?: string;
  status: string;
}

export function FleetManagementPage() {
  const { jwtToken, apiKey } = useAuthStore();
  const [sats, setSats] = useState<RegisteredSat[]>([]);
  const [stations, setStations] = useState<RegisteredStation[]>([]);
  const [activeSubTab, setActiveSubTab] = useState<'SATELLITES' | 'STATIONS'>('SATELLITES');

  // Form states for adding satellite
  const [satId, setSatId] = useState('');
  const [satName, setSatName] = useState('');
  const [noradId, setNoradId] = useState('');
  const [orbitType, setOrbitType] = useState('LEO');
  const [tle1, setTle1] = useState('');
  const [tle2, setTle2] = useState('');

  // Form states for adding station
  const [stnId, setStnId] = useState('');
  const [stnName, setStnName] = useState('');
  const [lat, setLat] = useState('');
  const [lon, setLon] = useState('');
  const [altM, setAltM] = useState('500');

  const [message, setMessage] = useState<{ text: string; isError: boolean } | null>(null);

  const tokenToUse = jwtToken || apiKey;

  const fetchFleetData = async () => {
    try {
      const satsResp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/satellites`, {
        headers: { Authorization: `Bearer ${tokenToUse}` },
      });
      if (satsResp.ok) {
        const data = await satsResp.json();
        setSats(data.satellites || []);
      }

      const stnResp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/ground-stations`, {
        headers: { Authorization: `Bearer ${tokenToUse}` },
      });
      if (stnResp.ok) {
        const data = await stnResp.json();
        setStations(data.ground_stations || []);
      }
    } catch (err) {
      console.error('Failed to fetch registered fleet:', err);
    }
  };

  useEffect(() => {
    fetchFleetData();
  }, [tokenToUse]);

  const handleAddSatellite = async (e: React.FormEvent) => {
    e.preventDefault();
    setMessage(null);
    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/satellites`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${tokenToUse}`,
        },
        body: JSON.stringify({
          satellite_id: satId,
          satellite_name: satName,
          norad_id: noradId ? parseInt(noradId, 10) : undefined,
          orbit_type: orbitType,
          tle_line1: tle1 || undefined,
          tle_line2: tle2 || undefined,
        }),
      });

      if (resp.ok) {
        setMessage({ text: `Satellite ${satId} registered successfully!`, isError: false });
        setSatId('');
        setSatName('');
        setNoradId('');
        setTle1('');
        setTle2('');
        fetchFleetData();
      } else {
        const err = await resp.json();
        setMessage({ text: err.message || 'Failed to register satellite', isError: true });
      }
    } catch {
      setMessage({ text: 'Command service unreachable', isError: true });
    }
  };

  const handleAddStation = async (e: React.FormEvent) => {
    e.preventDefault();
    setMessage(null);
    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/ground-stations`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${tokenToUse}`,
        },
        body: JSON.stringify({
          station_id: stnId,
          station_name: stnName,
          latitude: parseFloat(lat),
          longitude: parseFloat(lon),
          altitude_m: parseFloat(altM),
        }),
      });

      if (resp.ok) {
        setMessage({ text: `Ground station ${stnId} registered successfully!`, isError: false });
        setStnId('');
        setStnName('');
        setLat('');
        setLon('');
        fetchFleetData();
      } else {
        const err = await resp.json();
        setMessage({ text: err.message || 'Failed to register station', isError: true });
      }
    } catch {
      setMessage({ text: 'Command service unreachable', isError: true });
    }
  };

  const handleDeleteSat = async (id: string) => {
    if (!confirm(`Decommission satellite ${id}?`)) return;
    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/satellites/${id}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${tokenToUse}` },
      });
      if (resp.ok) {
        fetchFleetData();
      }
    } catch (err) {
      console.error(err);
    }
  };

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
          {/* Satellite List Table */}
          <div>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid var(--border-color)', color: '#38bdf8', textAlign: 'left' }}>
                  <th style={{ padding: '8px' }}>Spacecraft ID</th>
                  <th style={{ padding: '8px' }}>Name</th>
                  <th style={{ padding: '8px' }}>Orbit</th>
                  <th style={{ padding: '8px' }}>NORAD ID</th>
                  <th style={{ padding: '8px' }}>Status</th>
                  <th style={{ padding: '8px' }}>Actions</th>
                </tr>
              </thead>
              <tbody>
                {sats.length === 0 ? (
                  <tr>
                    <td colSpan={6} style={{ padding: '20px', textAlign: 'center', color: '#64748b' }}>
                      No registered satellites found for your tenant. Register one using the form.
                    </td>
                  </tr>
                ) : (
                  sats.map((s) => (
                    <tr key={s.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                      <td style={{ padding: '10px 8px', fontWeight: 'bold', color: '#38bdf8' }}>{s.satellite_id}</td>
                      <td style={{ padding: '10px 8px' }}>{s.satellite_name}</td>
                      <td style={{ padding: '10px 8px' }}>{s.orbit_type}</td>
                      <td style={{ padding: '10px 8px' }}>{s.norad_id || 'N/A'}</td>
                      <td style={{ padding: '10px 8px' }}>
                        <span
                          style={{
                            color: s.status === 'ACTIVE' ? '#10b981' : '#ef4444',
                            fontWeight: 'bold',
                          }}
                        >
                          {s.status}
                        </span>
                      </td>
                      <td style={{ padding: '10px 8px' }}>
                        {s.status === 'ACTIVE' && (
                          <button
                            onClick={() => handleDeleteSat(s.satellite_id)}
                            style={{
                              backgroundColor: 'transparent',
                              border: '1px solid #ef4444',
                              color: '#ef4444',
                              borderRadius: '3px',
                              padding: '2px 6px',
                              fontSize: '10px',
                              cursor: 'pointer',
                            }}
                          >
                            DECOMMISSION
                          </button>
                        )}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>

          {/* Add Satellite Form */}
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
            <form onSubmit={handleAddSatellite} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
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
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 340px', gap: '20px' }}>
          {/* Station List Table */}
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

          {/* Add Station Form */}
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
            <form onSubmit={handleAddStation} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
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
        </div>
      )}
    </div>
  );
}
