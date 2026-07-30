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

import React, { useState } from 'react';
import type { Satellite } from '../types';

interface FleetOverviewProps {
  satellites: Satellite[];
  selectedSatId: string | null;
  onSelectSat: (id: string) => void;
}

export const FleetOverview: React.FC<FleetOverviewProps> = ({
  satellites,
  selectedSatId,
  onSelectSat,
}) => {
  const [filter, setFilter] = useState<'ALL' | 'NOMINAL' | 'DEGRADED'>('ALL');

  const sortedSats = [...satellites].sort((a, b) => a.id.localeCompare(b.id));
  const filteredSats = sortedSats.filter((s) => {
    if (filter === 'NOMINAL') return s.state === 'GREEN';
    if (filter === 'DEGRADED') return s.state !== 'GREEN';
    return true;
  });

  const getLinkedGs = (satId: string) => {
    if (satId.startsWith('SAT-')) {
      const numStr = satId.substring(4);
      const num = parseInt(numStr, 10);
      if (!isNaN(num)) {
        const gsNum = ((num - 1) % 4) + 1;
        const gsNames = ['Svalbard DSN', 'Troll TTC', 'Bengaluru ISTRAC', 'McMurdo DSN'];
        return `GS-000${gsNum} (${gsNames[gsNum - 1]})`;
      }
    }
    return 'GS-0001 (Svalbard DSN)';
  };

  return (
    <div className="glass-panel tech-corner-panel" style={{ height: '100%' }}>
      <div className="panel-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span className="panel-header-icon">📡</span>
          <span>SATELLITE FLEET STATUS</span>
        </div>
        <span style={{ fontSize: '10px', fontFamily: 'var(--font-mono)', color: 'var(--cyber-cyan)' }}>
          TOTAL: {sortedSats.length}
        </span>
      </div>

      {/* Filter Tabs */}
      <div style={{ display: 'flex', gap: '4px', padding: '8px 10px', borderBottom: '1px solid var(--border-color)', backgroundColor: 'rgba(3, 7, 16, 0.4)' }}>
        {(['ALL', 'NOMINAL', 'DEGRADED'] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            style={{
              flex: 1,
              padding: '4px 6px',
              fontSize: '9px',
              fontFamily: 'var(--font-display)',
              fontWeight: 700,
              backgroundColor: filter === f ? 'var(--cyber-cyan)' : 'transparent',
              color: filter === f ? '#020409' : 'var(--text-muted)',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              transition: 'all 0.15s ease',
            }}
          >
            {f}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '10px' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {filteredSats.map((sat) => {
            const isSelected = selectedSatId === sat.id;
            const healthColorClass =
              sat.state === 'GREEN' ? 'green' : sat.state === 'YELLOW' ? 'yellow' : 'red';
            const healthTextClass =
              sat.state === 'GREEN' ? 'glow-text-green' : sat.state === 'YELLOW' ? 'glow-text-yellow' : 'glow-text-red';

            return (
              <div
                key={sat.id}
                onClick={() => onSelectSat(sat.id)}
                style={{
                  padding: '12px',
                  borderRadius: '6px',
                  background: isSelected ? 'rgba(0, 240, 255, 0.12)' : 'rgba(10, 18, 34, 0.6)',
                  border: isSelected ? '1px solid var(--cyber-cyan)' : '1px solid rgba(0, 240, 255, 0.1)',
                  boxShadow: isSelected ? '0 0 16px rgba(0, 240, 255, 0.25)' : 'none',
                  cursor: 'pointer',
                  transition: 'all 0.2s cubic-bezier(0.16, 1, 0.3, 1)',
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                }}
              >
                <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span className={`status-indicator ${healthColorClass}`} />
                    <span
                      style={{
                        fontFamily: 'var(--font-display)',
                        fontWeight: 800,
                        fontSize: '13px',
                        letterSpacing: '1px',
                        color: isSelected ? '#fff' : 'var(--text-primary)',
                      }}
                    >
                      {sat.id}
                    </span>
                  </div>
                  <span style={{ fontSize: '10px', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)' }}>
                    LINK: {getLinkedGs(sat.id)}
                  </span>
                </div>

                <div style={{ textAlign: 'right' }}>
                  <span
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontWeight: 800,
                      fontSize: '15px',
                    }}
                    className={healthTextClass}
                  >
                    {sat.score}%
                  </span>
                  <div style={{ fontSize: '9px', fontFamily: 'var(--font-display)', color: 'var(--text-muted)', marginTop: '2px', letterSpacing: '0.5px' }}>
                    HEALTH INDEX
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
