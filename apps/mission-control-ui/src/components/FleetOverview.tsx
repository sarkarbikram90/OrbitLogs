import React from 'react';
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
  // Sort satellites by ID for a clean display
  const sortedSats = [...satellites].sort((a, b) => a.id.localeCompare(b.id));

  // Determine linked ground station mapping helper
  const getLinkedGs = (satId: string) => {
    if (satId.startsWith('SAT-')) {
      const numStr = satId.substring(4);
      const num = parseInt(numStr, 10);
      if (!isNaN(num)) {
        const gsNum = ((num - 1) % 4) + 1;
        const gsNames = ['Svalbard', 'Troll', 'Kiruna', 'McMurdo'];
        return `GS-000${gsNum} (${gsNames[gsNum - 1]})`;
      }
    }
    return 'GS-0001 (Svalbard)';
  };

  return (
    <div className="glass-panel" style={{ height: '100%' }}>
      <div className="panel-header">
        <span>Satellite Fleet Status</span>
        <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
          Total: {sortedSats.length}
        </span>
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '10px' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {sortedSats.map(sat => {
            const isSelected = selectedSatId === sat.id;
            const healthColorClass = 
              sat.state === 'GREEN' ? 'green' : 
              sat.state === 'YELLOW' ? 'yellow' : 'red';
            
            const healthTextClass = 
              sat.state === 'GREEN' ? 'glow-text-green' : 
              sat.state === 'YELLOW' ? 'glow-text-yellow' : 'glow-text-red';

            return (
              <div
                key={sat.id}
                onClick={() => onSelectSat(sat.id)}
                style={{
                  padding: '12px',
                  borderRadius: '6px',
                  background: isSelected ? 'rgba(56, 189, 248, 0.08)' : 'rgba(26, 36, 56, 0.2)',
                  border: isSelected 
                    ? '1px solid rgba(56, 189, 248, 0.5)' 
                    : '1px solid rgba(56, 189, 248, 0.1)',
                  cursor: 'pointer',
                  transition: 'all 0.2s ease',
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                }}
                className="fleet-item"
              >
                <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span className={`status-indicator ${healthColorClass}`} />
                    <span 
                      style={{ 
                        fontFamily: 'var(--font-display)', 
                        fontWeight: '700', 
                        fontSize: '14px',
                        color: isSelected ? 'var(--text-primary)' : 'var(--text-secondary)'
                      }}
                    >
                      {sat.id}
                    </span>
                  </div>
                  <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                    Link: {getLinkedGs(sat.id)}
                  </span>
                </div>
                <div style={{ textAlign: 'right' }}>
                  <span 
                    style={{ 
                      fontFamily: 'var(--font-display)', 
                      fontWeight: '800', 
                      fontSize: '16px' 
                    }}
                    className={healthTextClass}
                  >
                    {sat.score}%
                  </span>
                  <div style={{ fontSize: '9px', color: 'var(--text-muted)', marginTop: '2px' }}>
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
