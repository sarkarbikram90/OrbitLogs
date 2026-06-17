import { useState, useEffect } from 'react';

interface FlightDynamicsState {
  satellite_id: string;
  altitude_decay_rate_m_day: number;
  raan_drift_deg_day: number;
  fuel_burn_rate_pct_day: number;
  maneuver_effectiveness_index: number;
  recommendation: string | null;
  days_until_maneuver: number;
}

interface FlightDynamicsProps {
  selectedSatId: string | null;
}

const getBackendOpsUrl = () => {
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:8095`;
  }
  return `http://localhost:8095`;
};

export function FlightDynamics({ selectedSatId }: FlightDynamicsProps) {
  const [data, setData] = useState<FlightDynamicsState | null>(null);
  const BACKEND_OPS_URL = getBackendOpsUrl();

  useEffect(() => {
    if (!selectedSatId) return;

    const fetchDynamics = async () => {
      try {
        const resp = await fetch(`${BACKEND_OPS_URL}/api/flight-dynamics/${selectedSatId}`);
        if (resp.ok) {
          const res = await resp.json();
          setData(res);
        }
      } catch (err) {
        console.error('Failed to fetch flight dynamics metrics:', err);
      }
    };

    fetchDynamics();
    const interval = setInterval(fetchDynamics, 5000);
    return () => clearInterval(interval);
  }, [selectedSatId]);

  return (
    <div className="glass-panel" style={{ height: '100%', padding: '20px', overflowY: 'auto' }}>
      <div className="panel-header" style={{ marginBottom: '20px', background: 'transparent', padding: 0 }}>
        <span>Flight Dynamics Analysis - {selectedSatId}</span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '16px', marginBottom: '24px' }}>
        {/* Metric 1 */}
        <div style={cardStyle}>
          <span style={cardTitleStyle}>Altitude Decay Rate</span>
          <span style={{ fontSize: '24px', fontWeight: 'bold', color: '#10b981', fontFamily: 'var(--font-mono)' }}>
            {data ? `${data.altitude_decay_rate_m_day.toFixed(1)} m/day` : 'N/A'}
          </span>
        </div>

        {/* Metric 2 */}
        <div style={cardStyle}>
          <span style={cardTitleStyle}>RAAN Drift Rate</span>
          <span style={{ fontSize: '24px', fontWeight: 'bold', color: '#f59e0b', fontFamily: 'var(--font-mono)' }}>
            {data ? `${data.raan_drift_deg_day.toFixed(3)}°/day` : 'N/A'}
          </span>
        </div>

        {/* Metric 3 */}
        <div style={cardStyle}>
          <span style={cardTitleStyle}>Propellant Burn Rate</span>
          <span style={{ fontSize: '24px', fontWeight: 'bold', color: '#38bdf8', fontFamily: 'var(--font-mono)' }}>
            {data ? `${data.fuel_burn_rate_pct_day.toFixed(2)} %/day` : 'N/A'}
          </span>
        </div>

        {/* Metric 4 */}
        <div style={cardStyle}>
          <span style={cardTitleStyle}>Maneuver Effectiveness</span>
          <span style={{ fontSize: '24px', fontWeight: 'bold', color: '#a855f7', fontFamily: 'var(--font-mono)' }}>
            {data ? `${(data.maneuver_effectiveness_index * 100).toFixed(1)} %` : 'N/A'}
          </span>
        </div>
      </div>

      {/* Advisory recommendation block */}
      <div style={{
        padding: '20px',
        backgroundColor: data?.recommendation ? 'rgba(245, 158, 11, 0.05)' : 'rgba(16, 185, 129, 0.05)',
        border: data?.recommendation ? '1px solid rgba(245, 158, 11, 0.3)' : '1px solid rgba(16, 185, 129, 0.3)',
        borderRadius: '6px'
      }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
          <span style={{
            fontWeight: 'bold',
            fontFamily: 'var(--font-display)',
            color: data?.recommendation ? '#f59e0b' : '#10b981',
            fontSize: '13px'
          }}>
            FLIGHT DYNAMICS ADVISORY
          </span>
          <span style={{
            fontSize: '11px',
            fontFamily: 'var(--font-mono)',
            color: 'var(--text-secondary)'
          }}>
            Days until next burn window: <span style={{ fontWeight: 'bold', color: '#f8fafc' }}>{data?.days_until_maneuver} days</span>
          </span>
        </div>

        <p style={{
          fontSize: '12px',
          color: 'var(--text-primary)',
          lineHeight: '1.6',
          margin: 0
        }}>
          {data?.recommendation || 'All orbit elements are locked within constellation boundaries. No correction maneuvers scheduled.'}
        </p>
      </div>
    </div>
  );
}

const cardStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  padding: '16px',
  backgroundColor: 'rgba(255, 255, 255, 0.01)',
  border: '1px solid var(--border-color)',
  borderRadius: '6px'
};

const cardTitleStyle: React.CSSProperties = {
  fontSize: '10px',
  fontFamily: 'var(--font-display)',
  color: 'var(--text-secondary)',
  textTransform: 'uppercase' as const,
  letterSpacing: '1px'
};
