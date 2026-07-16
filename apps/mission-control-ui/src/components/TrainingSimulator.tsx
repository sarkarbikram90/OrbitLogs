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

interface TrainingSession {
  session_id: string;
  operator_name: string;
  scenario_name: string;
  satellite_id: string;
  started_at: string;
  acknowledged_at: string | null;
  commands_sent: string[];
  is_resolved: boolean;
}

interface Scorecard {
  id: string;
  operator_name: string;
  scenario_name: string;
  response_time_seconds: number;
  score: number;
  passed: boolean;
  timestamp: string;
  details: {
    commands_executed: string[];
  };
}

interface TrainingSimulatorProps {
  selectedSatId: string | null;
}

const getBackendOpsUrl = () => {
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:8095`;
  }
  return `http://localhost:8095`;
};

const getBackendTwinUrl = () => {
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:8086`;
  }
  return `http://localhost:8086`;
};

export function TrainingSimulator({ selectedSatId }: TrainingSimulatorProps) {
  const [operator, setOperator] = useState('CADET_OPERATOR');
  const [scenario, setScenario] = useState('THERMAL_RUNAWAY');
  const [session, setSession] = useState<TrainingSession | null>(null);
  const [scorecard, setScorecard] = useState<Scorecard | null>(null);
  const [statusLogs, setStatusLogs] = useState<string[]>([]);
  const BACKEND_OPS_URL = getBackendOpsUrl();
  const BACKEND_TWIN_URL = getBackendTwinUrl();

  const addLog = (msg: string) => {
    setStatusLogs(prev => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
  };

  const handleStart = async () => {
    if (!selectedSatId) return;
    setScorecard(null);
    setStatusLogs([]);
    addLog(`Loading simulation scenario: ${scenario}...`);

    try {
      // 1. Inject failure directly into Digital Twin Engine first
      const subMap: Record<string, string> = {
        'THERMAL_RUNAWAY': 'thermal',
        'BATTERY_FAILURE': 'power',
        'COMMS_OUTAGE': 'communications',
        'ATTITUDE_DRIFT': 'adcs',
      };
      const sub = subMap[scenario];
      const twin_key = sessionStorage.getItem('VITE_API_KEY') || 'orbitlogs-secret-api-key';

      await fetch(`${BACKEND_TWIN_URL}/api/twin/${selectedSatId}/simulate-failure`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${twin_key}`
        },
        body: JSON.stringify({ subsystem: sub }),
      });

      // 2. Register training session on the Ops Service
      const resp = await fetch(`${BACKEND_OPS_URL}/api/training/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          operator_name: operator,
          scenario_name: scenario,
          satellite_id: selectedSatId,
        }),
      });

      if (resp.ok) {
        const data = await resp.json();
        setSession(data);
        addLog(`CRITICAL ANOMALY ALERT! ${scenario} detected on satellite ${selectedSatId}.`);
        addLog(`Awaiting operator acknowledgement...`);
      }
    } catch (err) {
      addLog(`ERROR: Failed to establish link with simulation coordinator.`);
    }
  };

  const handleAcknowledge = async () => {
    if (!session) return;
    try {
      const resp = await fetch(`${BACKEND_OPS_URL}/api/training/${operator}/acknowledge`, {
        method: 'POST',
      });
      if (resp.ok) {
        const data = await resp.json();
        setSession(data);
        addLog(`Alert acknowledged by operator. Emergency procedures initialized.`);
        addLog(`Please send appropriate corrective commands via command center.`);
      }
    } catch (err) {
      addLog(`ERROR: Acknowledgment carrier lost.`);
    }
  };

  const handleResolve = async () => {
    if (!session) return;
    addLog(`Sending resolution packet. Compiling evaluation metrics...`);
    try {
      const resp = await fetch(`${BACKEND_OPS_URL}/api/training/${operator}/resolve`, {
        method: 'POST',
      });
      if (resp.ok) {
        const scoreData = await resp.json();
        setScorecard(scoreData);
        setSession(null);
        addLog(`Scenario finalized. Scorecard generated successfully.`);
      }
    } catch (err) {
      addLog(`ERROR: Scorecard compile timed out.`);
    }
  };

  return (
    <div className="glass-panel" style={{ height: '100%', padding: '20px', overflowY: 'auto' }}>
      <div className="panel-header" style={{ marginBottom: '20px', background: 'transparent', padding: 0 }}>
        <span>OrbitLogs Training Simulator Mode</span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '320px 1fr', gap: '24px' }}>
        {/* Scenario Select Panel */}
        <div style={{
          padding: '16px',
          backgroundColor: 'rgba(255, 255, 255, 0.01)',
          border: '1px solid var(--border-color)',
          borderRadius: '6px',
          display: 'flex',
          flexDirection: 'column',
          gap: '12px'
        }}>
          <span style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8' }}>
            Configure Training Run
          </span>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>OPERATOR NAME</label>
            <input type="text" value={operator} onChange={(e) => setOperator(e.target.value)} style={inputStyle} disabled={session !== null} />
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>ANOMALY SCENARIO</label>
            <select value={scenario} onChange={(e) => setScenario(e.target.value)} style={selectStyle} disabled={session !== null}>
              <option value="THERMAL_RUNAWAY">Thermal Runaway Cascade</option>
              <option value="BATTERY_FAILURE">EPS Battery Under-Voltage</option>
              <option value="COMMS_OUTAGE">TTC High Signal Attenuation</option>
              <option value="ATTITUDE_DRIFT">ADCS Orientation Pitch Drift</option>
            </select>
          </div>

          {!session ? (
            <button onClick={handleStart} style={buttonStyle}>INITIALIZE FAILURE INCIDENT</button>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {!session.acknowledged_at ? (
                <button onClick={handleAcknowledge} style={{ ...buttonStyle, backgroundColor: '#f59e0b' }}>
                  ACKNOWLEDGE TELEMETRY OUTAGE
                </button>
              ) : (
                <button onClick={handleResolve} style={{ ...buttonStyle, backgroundColor: '#10b981' }}>
                  RESOLVE & GRADE SCENARIO
                </button>
              )}
            </div>
          )}
        </div>

        {/* Live Logs & Scorecard Panel */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {/* Logs */}
          <div style={{
            flex: '1',
            minHeight: '160px',
            backgroundColor: '#020408',
            border: '1px solid var(--border-color)',
            borderRadius: '6px',
            padding: '12px',
            fontFamily: 'var(--font-mono)',
            fontSize: '11px',
            overflowY: 'auto'
          }}>
            <div style={{ color: '#64748b', borderBottom: '1px solid rgba(255,255,255,0.05)', paddingBottom: '4px', marginBottom: '8px' }}>
              SIMULATOR STDOUT CONNECTED:
            </div>
            {statusLogs.map((log, i) => (
              <div key={i} style={{
                color: log.includes('ALERT') || log.includes('ERROR') ? '#ef4444' : log.includes('acknowledged') || log.includes('Result:') ? '#10b981' : '#38bdf8',
                marginBottom: '4px'
              }}>
                {log}
              </div>
            ))}
          </div>

          {/* Scorecard Display */}
          {scorecard && (
            <div style={{
              padding: '16px',
              backgroundColor: scorecard.passed ? 'rgba(16, 185, 129, 0.05)' : 'rgba(239, 68, 68, 0.05)',
              border: scorecard.passed ? '1px solid #10b981' : '1px solid #ef4444',
              borderRadius: '6px'
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
                <span style={{ fontWeight: 'bold', fontFamily: 'var(--font-display)', color: scorecard.passed ? '#10b981' : '#ef4444', fontSize: '14px' }}>
                  OPERATOR EVALUATION SCORECARD
                </span>
                <span style={{
                  fontSize: '14px',
                  fontFamily: 'var(--font-mono)',
                  fontWeight: 'bold',
                  color: '#f8fafc'
                }}>
                  {scorecard.score} / 100 ({scorecard.passed ? 'PASS' : 'FAIL'})
                </span>
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
                <div>Operator: <span style={{ color: '#38bdf8' }}>{scorecard.operator_name}</span></div>
                <div>Response Time: <span style={{ color: '#f59e0b' }}>{scorecard.response_time_seconds.toFixed(2)} seconds</span></div>
                <div style={{ gridColumn: 'span 2', marginTop: '4px' }}>
                  Commands Sent: {scorecard.details.commands_executed.length > 0 ? (
                    scorecard.details.commands_executed.map((c, i) => <code key={i} style={{ color: '#c084fc', marginLeft: '4px' }}>{c}</code>)
                  ) : (
                    <span style={{ color: '#ef4444' }}>NONE</span>
                  )}
                </div>
              </div>
            </div>
          )}
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
  padding: '12px',
  fontSize: '11px',
  fontFamily: 'var(--font-display)',
  fontWeight: 'bold',
  letterSpacing: '1px',
  cursor: 'pointer',
  transition: 'all 0.2s ease',
  marginTop: '8px'
};
