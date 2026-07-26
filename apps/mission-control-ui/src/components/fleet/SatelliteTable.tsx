// Copyright 2026 OrbitLogs Authors

import { RegisteredSat } from './types';

interface SatelliteTableProps {
  sats: RegisteredSat[];
  onDelete: (id: string) => void;
}

export function SatelliteTable({ sats, onDelete }: SatelliteTableProps) {
  return (
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
                      onClick={() => onDelete(s.satellite_id)}
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
  );
}
