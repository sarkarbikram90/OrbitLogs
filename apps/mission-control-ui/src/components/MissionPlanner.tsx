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
import { useState, useEffect, useCallback } from 'react';

interface MissionPlan {
  id: string;
  satellite_id: string;
  plan_type: string;
  description: string;
  scheduled_time: string;
  status: string;
  created_by: string;
  approved_by: string | null;
  created_at: string;
}

const getBackendOpsUrl = () => {
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:8095`;
  }
  return `http://localhost:8095`;
};

export function MissionPlanner() {
  const [plans, setPlans] = useState<MissionPlan[]>([]);
  const [satId, setSatId] = useState('SAT-0001');
  const [type, setType] = useState('PAYLOAD');
  const [desc, setDesc] = useState('');
  const [time, setTime] = useState('');
  const BACKEND_OPS_URL = getBackendOpsUrl();

  const fetchPlans = useCallback(async () => {
    try {
      const resp = await fetch(`${BACKEND_OPS_URL}/api/planner/plans`);
      if (resp.ok) {
        const data = await resp.json();
        setPlans(data);
      }
    } catch (err) {
      console.error('Failed to fetch planner schedules:', err);
    }
  }, [BACKEND_OPS_URL]);

  useEffect(() => {
    let isMounted = true;
    const load = async () => {
      try {
        const resp = await fetch(`${BACKEND_OPS_URL}/api/planner/plans`);
        if (resp.ok && isMounted) {
          const data = await resp.json();
          setPlans(data);
        }
      } catch (err) {
        console.error('Failed to fetch planner schedules:', err);
      }
    };
    load();
    return () => {
      isMounted = false;
    };
  }, [BACKEND_OPS_URL]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!desc || !time) return;

    try {
      const resp = await fetch(`${BACKEND_OPS_URL}/api/planner/plans`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          satellite_id: satId,
          plan_type: type,
          description: desc,
          scheduled_time: new Date(time).toISOString(),
          created_by: 'OPERATOR-CONSOLE',
        }),
      });

      if (resp.ok) {
        setDesc('');
        setTime('');
        fetchPlans();
      }
    } catch (err) {
      console.error('Failed to submit plan:', err);
    }
  };

  const handleAction = async (id: string, action: string) => {
    try {
      const resp = await fetch(`${BACKEND_OPS_URL}/api/planner/plans/${id}/status`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          status: action,
          operator: 'FLIGHT-DIRECTOR',
        }),
      });
      if (resp.ok) {
        fetchPlans();
      }
    } catch (err) {
      console.error('Failed to update plan status:', err);
    }
  };

  return (
    <div className="glass-panel" style={{ height: '100%', padding: '20px', overflowY: 'auto' }}>
      <div className="panel-header" style={{ marginBottom: '20px', background: 'transparent', padding: 0 }}>
        <span>Mission Planner & Scheduler</span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '320px 1fr', gap: '24px' }}>
        {/* Left Side: Create Plan Form */}
        <form onSubmit={handleCreate} style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '12px',
          padding: '16px',
          backgroundColor: 'rgba(255, 255, 255, 0.01)',
          border: '1px solid var(--border-color)',
          borderRadius: '6px'
        }}>
          <span style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8' }}>
            New Activity Schedule
          </span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>SATELLITE</label>
            <select value={satId} onChange={(e) => setSatId(e.target.value)} style={selectStyle}>
              {Array.from({ length: 10 }).map((_, idx) => {
                const id = `SAT-${String(idx + 1).padStart(4, '0')}`;
                return <option key={id} value={id}>{id}</option>;
              })}
            </select>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>ACTIVITY TYPE</label>
            <select value={type} onChange={(e) => setType(e.target.value)} style={selectStyle}>
              <option value="PAYLOAD">Payload Activation</option>
              <option value="IMAGE">Earth Observation Capture</option>
              <option value="MANEUVER">Station Keeping Maneuver</option>
              <option value="UPDATE">Flight Software Update</option>
              <option value="COMMAND">Uplink Command Sequence</option>
            </select>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>DESCRIPTION</label>
            <input type="text" value={desc} onChange={(e) => setDesc(e.target.value)} placeholder="e.g. imaging over Siliguri sector" style={inputStyle} />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>EXECUTION TIME (UTC)</label>
            <input type="datetime-local" value={time} onChange={(e) => setTime(e.target.value)} style={inputStyle} />
          </div>
          <button type="submit" style={buttonStyle}>SUBMIT FOR REVIEW</button>
        </form>

        {/* Right Side: Active Plans Table */}
        <div>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid var(--border-color)', color: '#38bdf8', textAlign: 'left' }}>
                <th style={{ padding: '8px' }}>Asset</th>
                <th style={{ padding: '8px' }}>Type</th>
                <th style={{ padding: '8px' }}>Description</th>
                <th style={{ padding: '8px' }}>Time (UTC)</th>
                <th style={{ padding: '8px' }}>Status</th>
                <th style={{ padding: '8px', textAlign: 'right' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {plans.map((plan) => (
                <tr key={plan.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                  <td style={{ padding: '12px 8px', fontWeight: 'bold' }}>{plan.satellite_id}</td>
                  <td style={{ padding: '12px 8px' }}>{plan.plan_type}</td>
                  <td style={{ padding: '12px 8px' }}>{plan.description}</td>
                  <td style={{ padding: '12px 8px' }}>{new Date(plan.scheduled_time).toISOString().replace('T', ' ').substring(0, 16)}</td>
                  <td style={{ padding: '12px 8px' }}>
                    <span style={{
                      padding: '2px 6px',
                      borderRadius: '4px',
                      fontSize: '10px',
                      fontWeight: 'bold',
                      backgroundColor: plan.status === 'APPROVED' || plan.status === 'SCHEDULED' ? 'rgba(16, 185, 129, 0.15)' : plan.status === 'DRAFT' ? 'rgba(148, 163, 184, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                      color: plan.status === 'APPROVED' || plan.status === 'SCHEDULED' ? '#10b981' : plan.status === 'DRAFT' ? '#94a3b8' : '#ef4444'
                    }}>
                      {plan.status}
                    </span>
                  </td>
                  <td style={{ padding: '12px 8px', textAlign: 'right' }}>
                    {plan.status === 'DRAFT' && (
                      <button onClick={() => handleAction(plan.id, 'REVIEW')} style={actionBtnStyle}>SEND TO REVIEW</button>
                    )}
                    {plan.status === 'REVIEW' && (
                      <div style={{ display: 'flex', gap: '4px', justifyContent: 'flex-end' }}>
                        <button onClick={() => handleAction(plan.id, 'APPROVED')} style={{ ...actionBtnStyle, color: '#10b981', borderColor: '#10b981' }}>APPROVE</button>
                        <button onClick={() => handleAction(plan.id, 'CANCELLED')} style={{ ...actionBtnStyle, color: '#ef4444', borderColor: '#ef4444' }}>CANCEL</button>
                      </div>
                    )}
                    {(plan.status === 'APPROVED' || plan.status === 'REVIEW') && (
                      <button onClick={() => handleAction(plan.id, 'SCHEDULED')} style={{ ...actionBtnStyle, color: '#0ea5e9', borderColor: '#0ea5e9' }}>SCHEDULE</button>
                    )}
                    {plan.status === 'SCHEDULED' && (
                      <button onClick={() => handleAction(plan.id, 'EXECUTED')} style={actionBtnStyle}>EXECUTE</button>
                    )}
                  </td>
                </tr>
              ))}
              {plans.length === 0 && (
                <tr>
                  <td colSpan={6} style={{ padding: '20px 8px', textAlign: 'center', color: 'var(--text-muted)' }}>
                    No planned actions configured.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

const selectStyle = {
  backgroundColor: '#07090e',
  color: '#f8fafc',
  border: '1px solid rgba(56, 189, 248, 0.2)',
  borderRadius: '4px',
  padding: '8px',
  fontSize: '12px',
  outline: 'none',
};

const inputStyle = {
  backgroundColor: '#07090e',
  color: '#f8fafc',
  border: '1px solid rgba(56, 189, 248, 0.2)',
  borderRadius: '4px',
  padding: '8px',
  fontSize: '12px',
  outline: 'none',
};

const buttonStyle = {
  backgroundColor: '#0ea5e9',
  color: '#ffffff',
  border: 'none',
  borderRadius: '4px',
  padding: '10px',
  fontSize: '11px',
  fontFamily: 'var(--font-display)',
  fontWeight: 'bold',
  letterSpacing: '1px',
  cursor: 'pointer',
  transition: 'all 0.2s ease',
  marginTop: '8px'
};

const actionBtnStyle = {
  backgroundColor: 'transparent',
  color: '#94a3b8',
  border: '1px solid rgba(255, 255, 255, 0.2)',
  borderRadius: '4px',
  padding: '4px 8px',
  fontSize: '10px',
  cursor: 'pointer',
  transition: 'all 0.2s ease'
};
