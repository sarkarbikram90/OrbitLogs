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
import { useAuthStore } from '../../stores/authStore';
import { useFleetStore } from '../../stores/fleetStore';

const TABS = [
  ['DASHBOARD', 'DASHBOARD'],
  ['FLEET', 'FLEET MGMT'],
  ['COMMANDS', 'COMMANDS'],
  ['PLANNER', 'PLANNER'],
  ['GROUND_OPS', 'GROUND OPS'],
  ['PASSES', 'PASSES'],
  ['SSA', 'SSA'],
  ['FLIGHT_DYNAMICS', 'DYNAMICS'],
  ['TWIN', 'TWIN'],
  ['TRAINING', 'TRAINING'],
  ['AI_DIRECTOR', 'DIRECTOR'],
  ['TIMELINE', 'TIMELINE'],
  ['AI', 'COPILOT'],
] as const;

export function Header() {
  const { logout, tenantSlug } = useAuthStore();
  const { mode, dashboardMode, activeTab, setDashboardMode, setActiveTab } = useFleetStore();
  const [utcTime, setUtcTime] = useState(new Date().toISOString().replace('T', ' ').substring(0, 19));

  useEffect(() => {
    const timer = setInterval(() => {
      setUtcTime(new Date().toISOString().replace('T', ' ').substring(0, 19));
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  return (
    <header className="header">
      <div className="header-title-container">
        <span className="logo-icon">🛰</span>
        <span className="header-title">OrbitLogs - Mission Control</span>
        {tenantSlug && (
          <span
            style={{
              fontSize: '10px',
              fontFamily: 'var(--font-mono)',
              color: '#38bdf8',
              backgroundColor: 'rgba(56, 189, 248, 0.1)',
              padding: '2px 6px',
              borderRadius: '3px',
              border: '1px solid rgba(56, 189, 248, 0.3)',
            }}
          >
            {tenantSlug.toUpperCase()}
          </span>
        )}
      </div>

      {/* Tab Navigation */}
      <div style={{ display: 'flex', gap: '4px', margin: '0 16px', overflowX: 'auto', maxWidth: 'calc(100% - 600px)' }}>
        {TABS.map(([tabId, label]) => (
          <button
            key={tabId}
            onClick={() => setActiveTab(tabId)}
            style={{
              padding: '4px 8px',
              backgroundColor: activeTab === tabId ? 'rgba(56, 189, 248, 0.15)' : 'transparent',
              color: activeTab === tabId ? '#38bdf8' : '#94a3b8',
              border: activeTab === tabId ? '1px solid rgba(56, 189, 248, 0.4)' : '1px solid transparent',
              borderRadius: '4px',
              fontSize: '10px',
              fontWeight: 'bold',
              fontFamily: 'var(--font-display)',
              letterSpacing: '0.5px',
              cursor: 'pointer',
              transition: 'all 0.2s ease',
              whiteSpace: 'nowrap',
            }}
          >
            {label}
          </button>
        ))}
      </div>

      <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
        <div className="header-status">
          <span>UTC: {utcTime}</span>
          <select
            value={dashboardMode}
            onChange={(e) => setDashboardMode(e.target.value as 'OPERATOR' | 'EXECUTIVE')}
            style={{
              backgroundColor: '#07090e',
              color: '#38bdf8',
              border: '1px solid rgba(56, 189, 248, 0.3)',
              borderRadius: '4px',
              padding: '4px 8px',
              fontSize: '10px',
              fontFamily: 'var(--font-display)',
              cursor: 'pointer',
              outline: 'none',
              fontWeight: 'bold',
            }}
          >
            <option value="OPERATOR">OPERATOR VIEW</option>
            <option value="EXECUTIVE">EXECUTIVE VIEW</option>
          </select>
          <div className={`mode-badge ${mode === 'LIVE' ? 'live' : 'replay'}`}>
            {mode === 'LIVE' ? '● LIVE' : '▼ REPLAY'}
          </div>
          <button
            onClick={logout}
            style={{
              backgroundColor: 'transparent',
              border: '1px solid #ef4444',
              color: '#ef4444',
              borderRadius: '4px',
              padding: '4px 8px',
              fontSize: '10px',
              fontFamily: 'var(--font-display)',
              cursor: 'pointer',
              fontWeight: 'bold',
              transition: 'all 0.2s ease',
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLButtonElement).style.backgroundColor = 'rgba(239, 68, 68, 0.1)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLButtonElement).style.backgroundColor = 'transparent';
            }}
          >
            DISCONNECT
          </button>
        </div>
      </div>
    </header>
  );
}
