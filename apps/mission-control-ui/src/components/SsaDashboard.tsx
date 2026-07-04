import { useState, useEffect } from 'react';

interface Conjunction {
  id: string;
  satellite_id: string;
  object_name: string;
  object_type: string;
  probability: number;
  distance_meters: number;
  conjunction_time: string;
  severity: string;
}

interface SsaStatus {
  satellite_id: string;
  nearest_object_name: string;
  nearest_object_distance: number;
  collision_probability: number;
  overall_risk_score: number;
}

interface SsaDashboardProps {
  selectedSatId: string | null;
}

const getBackendOpsUrl = () => {
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:8095`;
  }
  return `http://localhost:8095`;
};

export function SsaDashboard({ selectedSatId }: SsaDashboardProps) {
  const [conjunctions, setConjunctions] = useState<Conjunction[]>([]);
  const [status, setStatus] = useState<SsaStatus | null>(null);
  const BACKEND_OPS_URL = getBackendOpsUrl();

  useEffect(() => {
    if (!selectedSatId) return;

    const fetchSsa = async () => {
      try {
        const statusResp = await fetch(`${BACKEND_OPS_URL}/api/ssa/${selectedSatId}/status`);
        if (statusResp.ok) {
          const statusData = await statusResp.json();
          setStatus(statusData);
        }

        const conjResp = await fetch(`${BACKEND_OPS_URL}/api/ssa/${selectedSatId}/conjunctions`);
        if (conjResp.ok) {
          const conjData = await conjResp.json();
          setConjunctions(conjData);
        }
      } catch (err) {
        console.error('Failed to fetch SSA data:', err);
      }
    };

    fetchSsa();
    const interval = setInterval(fetchSsa, 5000);
    return () => clearInterval(interval);
  }, [selectedSatId]);

  return (
    <div className="glass-panel" style={{ height: '100%', padding: '20px', overflowY: 'auto' }}>
      <div className="panel-header" style={{ marginBottom: '20px', background: 'transparent', padding: 0 }}>
        <span>Space Situational Awareness (SSA) - {selectedSatId}</span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px', marginBottom: '24px' }}>
        {/* Risk Assessment Summary */}
        <div style={{
          padding: '16px',
          backgroundColor: 'rgba(255, 255, 255, 0.01)',
          border: '1px solid var(--border-color)',
          borderRadius: '6px'
        }}>
          <span style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8' }}>
            Risk Assessment Scorecard
          </span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '12px', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-secondary)' }}>Nearest Object:</span>
              <span>{status?.nearest_object_name || 'NO CLOSE DEBRIS'}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-secondary)' }}>Closest Distance:</span>
              <span style={{ color: status && status.nearest_object_distance < 2000 ? '#ef4444' : '#10b981', fontWeight: 'bold' }}>
                {status ? `${status.nearest_object_distance.toFixed(1)} meters` : 'N/A'}
              </span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-secondary)' }}>Collision Probability:</span>
              <span style={{ color: status && status.collision_probability > 0.01 ? '#ef4444' : '#10b981', fontWeight: 'bold' }}>
                {status ? status.collision_probability.toFixed(6) : '0.000000'}
              </span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid rgba(255,255,255,0.05)', paddingTop: '8px' }}>
              <span style={{ color: 'var(--text-secondary)' }}>Overall Orbit Plane Risk:</span>
              <span style={{
                color: status && status.overall_risk_score > 60 ? '#ef4444' : status && status.overall_risk_score > 20 ? '#f59e0b' : '#10b981',
                fontWeight: 'bold'
              }}>
                {status ? `${status.overall_risk_score.toFixed(0)} / 100` : 'NOMINAL'}
              </span>
            </div>
          </div>
        </div>

        {/* Informational Guidelines & Autonomous Recommendation */}
        <div style={{
          padding: '16px',
          backgroundColor: 'rgba(239, 68, 68, 0.05)',
          border: '1px solid rgba(239, 68, 68, 0.3)',
          borderRadius: '6px',
          fontSize: '11px',
          color: 'var(--text-secondary)',
          lineHeight: '1.6'
        }}>
          <span style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#ef4444', display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '8px' }}>
            <span>🚨</span> AI Autonomous Collision Recommendation
          </span>
          <div style={{ fontSize: '12px', fontFamily: 'var(--font-mono)', color: 'var(--text-primary)', marginBottom: '10px' }}>
            <strong>Maneuver Proposal:</strong> In-Track Prograde Firing (+0.45 m/s Δv)
            <br />
            <strong>TCA Window:</strong> T-45 mins before close approach
            <br />
            <strong>Propellant Impact:</strong> 0.42 kg (Post-burn fuel: 92.1%)
            <br />
            <strong>Post-Burn Miss Distance:</strong> 6.82 km (Pc &lt; 10⁻⁷)
          </div>
          <button
            onClick={() => alert(`Avoidance burn sequence authorized for ${selectedSatId}! Uplinking CMD_PROP_THRUST...`)}
            style={{
              padding: '6px 14px',
              backgroundColor: '#ef4444',
              color: '#ffffff',
              border: 'none',
              borderRadius: '4px',
              fontWeight: 'bold',
              fontSize: '11px',
              cursor: 'pointer',
              boxShadow: '0 0 10px rgba(239, 68, 68, 0.4)'
            }}
          >
            Authorize Collision Avoidance Burn
          </button>
        </div>
      </div>

      {/* Conjunction History Log */}
      <div>
        <span style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8', display: 'block', marginBottom: '12px' }}>
          Conjunction Event History
        </span>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
          <thead>
            <tr style={{ borderBottom: '1px solid var(--border-color)', color: '#38bdf8', textAlign: 'left' }}>
              <th style={{ padding: '8px' }}>Object Name</th>
              <th style={{ padding: '8px' }}>Type</th>
              <th style={{ padding: '8px' }}>Distance</th>
              <th style={{ padding: '8px' }}>Probability</th>
              <th style={{ padding: '8px' }}>Conjunction Time (UTC)</th>
              <th style={{ padding: '8px' }}>Severity</th>
            </tr>
          </thead>
          <tbody>
            {conjunctions.map((conj) => (
              <tr key={conj.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                <td style={{ padding: '12px 8px', fontWeight: 'bold' }}>{conj.object_name}</td>
                <td style={{ padding: '12px 8px' }}>{conj.object_type}</td>
                <td style={{ padding: '12px 8px', color: conj.severity === 'CRITICAL' ? '#ef4444' : '#f59e0b' }}>
                  {conj.distance_meters.toFixed(1)}m
                </td>
                <td style={{ padding: '12px 8px' }}>{conj.probability.toFixed(6)}</td>
                <td style={{ padding: '12px 8px' }}>{new Date(conj.conjunction_time).toISOString().replace('T', ' ').substring(0, 19)}</td>
                <td style={{ padding: '12px 8px' }}>
                  <span style={{
                    padding: '2px 6px',
                    borderRadius: '4px',
                    fontSize: '9px',
                    fontWeight: 'bold',
                    backgroundColor: conj.severity === 'CRITICAL' ? 'rgba(239, 68, 68, 0.15)' : 'rgba(245, 158, 11, 0.15)',
                    color: conj.severity === 'CRITICAL' ? '#ef4444' : '#f59e0b'
                  }}>
                    {conj.severity}
                  </span>
                </td>
              </tr>
            ))}
            {conjunctions.length === 0 && (
              <tr>
                <td colSpan={6} style={{ padding: '20px 8px', textAlign: 'center', color: 'var(--text-muted)' }}>
                  No close approaches cataloged. Space environment is nominal.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
