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
import { CommandPalette } from '../CommandPalette';

const TAB_GROUPS = [
  {
    group: 'FLIGHT OPS',
    tabs: [
      ['DASHBOARD', '⚡ DASHBOARD'],
      ['FLEET', '🛰 FLEET'],
      ['COMMANDS', '📡 COMMANDS'],
      ['PLANNER', '🧠 PLANNER'],
      ['TIMELINE', '⏱ TIMELINE'],
    ],
  },
  {
    group: 'ASTRODYNAMICS',
    tabs: [
      ['GROUND_OPS', '🌐 GROUND OPS'],
      ['PASSES', '📡 PASSES'],
      ['SSA', '🛡 SSA / DEBRIS'],
      ['FLIGHT_DYNAMICS', '📐 DYNAMICS'],
      ['TWIN', '🧊 DIGITAL TWIN'],
    ],
  },
  {
    group: 'INTELLIGENCE & SAAS',
    tabs: [
      ['AI_DIRECTOR', '🎯 AI DIRECTOR'],
      ['AI', '🤖 COPILOT'],
      ['TRAINING', '🎮 SIMULATOR'],
      ['BILLING', '💳 SAAS BILLING'],
    ],
  },
] as const;

export function Header() {
  const { logout, tenantSlug } = useAuthStore();
  const { mode, dashboardMode, activeTab, setDashboardMode, setActiveTab } = useFleetStore();
  const [utcTime, setUtcTime] = useState('');
  const [isCommandOpen, setIsCommandOpen] = useState(false);
  const [soundEnabled, setSoundEnabled] = useState(true);

  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setUtcTime(now.toISOString().replace('T', ' ').substring(0, 23) + ' UTC');
    };
    updateTime();
    const timer = setInterval(updateTime, 100);
    return () => clearInterval(timer);
  }, []);

  return (
    <>
      <header className="header">
        {/* Brand & Logo */}
        <div className="header-title-container">
          <span className="logo-icon">🛰</span>
          <div>
            <div className="header-title">ORBITLOGS • MISSION CONTROL</div>
            <div style={{ fontSize: '9px', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)', letterSpacing: '1px' }}>
              SPACECRAFT OBSERVABILITY & DIGITAL TWIN PLATFORM
            </div>
          </div>
          {tenantSlug && (
            <span
              style={{
                fontSize: '10px',
                fontFamily: 'var(--font-mono)',
                color: 'var(--cyber-cyan)',
                backgroundColor: 'rgba(0, 240, 255, 0.1)',
                padding: '3px 8px',
                borderRadius: '4px',
                border: '1px solid rgba(0, 240, 255, 0.3)',
                fontWeight: 700,
                letterSpacing: '1px',
              }}
            >
              {tenantSlug.toUpperCase()}
            </span>
          )}
        </div>

        {/* Tab Navigation */}
        <div style={{ display: 'flex', gap: '16px', margin: '0 12px', overflowX: 'auto', alignItems: 'center' }}>
          {TAB_GROUPS.map((group) => (
            <div key={group.group} style={{ display: 'flex', gap: '3px', alignItems: 'center', backgroundColor: 'rgba(6, 12, 24, 0.6)', padding: '3px', borderRadius: '6px', border: '1px solid rgba(255, 255, 255, 0.05)' }}>
              {group.tabs.map(([tabId, label]) => {
                const isActive = activeTab === tabId;
                return (
                  <button
                    key={tabId}
                    onClick={() => setActiveTab(tabId as any)}
                    style={{
                      padding: '5px 10px',
                      backgroundColor: isActive ? 'var(--cyber-cyan)' : 'transparent',
                      color: isActive ? '#020409' : 'var(--text-secondary)',
                      border: 'none',
                      borderRadius: '4px',
                      fontSize: '10px',
                      fontWeight: 800,
                      fontFamily: 'var(--font-display)',
                      letterSpacing: '0.5px',
                      cursor: 'pointer',
                      transition: 'all 0.2s cubic-bezier(0.16, 1, 0.3, 1)',
                      whiteSpace: 'nowrap',
                      boxShadow: isActive ? '0 0 12px var(--cyber-cyan-glow)' : 'none',
                    }}
                  >
                    {label}
                  </button>
                );
              })}
            </div>
          ))}
        </div>

        {/* Tactical Actions & UTC HUD */}
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
          {/* Quick Search / Command Palette Button */}
          <button
            onClick={() => setIsCommandOpen(true)}
            className="cyber-btn-secondary"
            style={{ fontSize: '10px', display: 'flex', alignItems: 'center', gap: '6px' }}
          >
            <span>🔍 SEARCH</span>
            <span style={{ fontSize: '9px', opacity: 0.6, fontFamily: 'var(--font-mono)' }}>Ctrl+K</span>
          </button>

          {/* Sound FX Toggle */}
          <button
            onClick={() => setSoundEnabled(!soundEnabled)}
            className="cyber-btn-secondary"
            style={{ padding: '6px 8px', fontSize: '12px' }}
            title={soundEnabled ? 'Telemetry Audio Active' : 'Muted'}
          >
            {soundEnabled ? '🔊' : '🔇'}
          </button>

          <div className="header-status">
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: '11px', color: 'var(--text-secondary)' }}>
              {utcTime}
            </span>

            <select
              value={dashboardMode}
              onChange={(e) => setDashboardMode(e.target.value as 'OPERATOR' | 'EXECUTIVE')}
              style={{
                backgroundColor: 'rgba(3, 7, 16, 0.9)',
                color: 'var(--cyber-cyan)',
                border: '1px solid var(--border-color)',
                borderRadius: '4px',
                padding: '4px 8px',
                fontSize: '10px',
                fontFamily: 'var(--font-display)',
                cursor: 'pointer',
                outline: 'none',
                fontWeight: 700,
              }}
            >
              <option value="OPERATOR">OPERATOR VIEW</option>
              <option value="EXECUTIVE">EXECUTIVE VIEW</option>
            </select>

            <div className={`mode-badge ${mode === 'LIVE' ? 'live' : 'replay'}`}>
              {mode === 'LIVE' ? '● TELEMETRY LOCK' : '▼ REPLAY MODE'}
            </div>

            <button
              onClick={logout}
              style={{
                backgroundColor: 'rgba(255, 46, 84, 0.1)',
                border: '1px solid var(--status-red)',
                color: 'var(--status-red)',
                borderRadius: '4px',
                padding: '4px 10px',
                fontSize: '10px',
                fontFamily: 'var(--font-display)',
                cursor: 'pointer',
                fontWeight: 700,
                transition: 'all 0.2s ease',
              }}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLButtonElement).style.backgroundColor = 'var(--status-red)';
                (e.currentTarget as HTMLButtonElement).style.color = '#fff';
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLButtonElement).style.backgroundColor = 'rgba(255, 46, 84, 0.1)';
                (e.currentTarget as HTMLButtonElement).style.color = 'var(--status-red)';
              }}
            >
              DISCONNECT
            </button>
          </div>
        </div>
      </header>

      <CommandPalette isOpen={isCommandOpen} onClose={() => setIsCommandOpen(false)} />
    </>
  );
}
