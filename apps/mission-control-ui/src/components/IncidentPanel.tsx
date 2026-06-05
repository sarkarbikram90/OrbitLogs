import React, { useState } from 'react';
import type { EventLog } from '../types';

interface IncidentPanelProps {
  events: EventLog[];
}

export const IncidentPanel: React.FC<IncidentPanelProps> = ({ events }) => {
  const [filter, setFilter] = useState<'ALL' | 'INFO' | 'WARNING' | 'CRITICAL'>('ALL');

  const filteredEvents = events.filter(e => {
    if (filter === 'ALL') return true;
    return e.severity === filter;
  });

  const getSeverityColor = (sev: string) => {
    if (sev === 'INFO') return 'var(--status-blue)';
    if (sev === 'WARNING') return 'var(--status-yellow)';
    return 'var(--status-red)';
  };

  const fmtTime = (isoString: string) => {
    try {
      const d = new Date(isoString);
      return d.toTimeString().split(' ')[0];
    } catch {
      return '--:--:--';
    }
  };

  return (
    <div className="glass-panel" style={{ height: '100%' }}>
      <div className="panel-header">
        <span>Incident & Events Log</span>
        <div style={{ display: 'flex', gap: '4px' }}>
          {(['ALL', 'INFO', 'WARNING', 'CRITICAL'] as const).map(f => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              style={{
                fontSize: '8px',
                padding: '2px 6px',
                background: filter === f ? 'rgba(56, 189, 248, 0.15)' : 'transparent',
                border: filter === f ? '1px solid var(--status-blue)' : '1px solid rgba(255, 255, 255, 0.05)',
                color: filter === f ? 'var(--status-blue)' : 'var(--text-secondary)',
                borderRadius: '3px',
                cursor: 'pointer',
                fontFamily: 'var(--font-display)',
                fontWeight: '700',
              }}
            >
              {f}
            </button>
          ))}
        </div>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '10px' }}>
        {filteredEvents.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '40px 10px', color: 'var(--text-muted)', fontSize: '11px', fontFamily: 'var(--font-mono)' }}>
            NO INCIDENTS LOGGED IN THIS WINDOW
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            {filteredEvents.map((evt, idx) => {
              const sevColor = getSeverityColor(evt.severity);

              return (
                <div
                  key={`${evt.timestamp}-${evt.satellite_id}-${idx}`}
                  style={{
                    padding: '8px',
                    borderRadius: '4px',
                    background: 'rgba(26, 36, 56, 0.15)',
                    border: '1px solid rgba(255, 255, 255, 0.03)',
                    borderLeft: `2.5px solid ${sevColor}`,
                    fontSize: '11px',
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px', fontFamily: 'var(--font-mono)' }}>
                    <span style={{ fontWeight: '700', color: 'var(--text-primary)' }}>
                      {evt.satellite_id}
                    </span>
                    <span style={{ color: 'var(--text-muted)', fontSize: '10px' }}>
                      {fmtTime(evt.timestamp)}
                    </span>
                  </div>
                  <div style={{ color: 'var(--text-secondary)', marginBottom: '4px', fontSize: '10px' }}>
                    Type: <span style={{ color: 'var(--text-primary)' }}>{evt.event_type}</span>
                  </div>
                  <p style={{ margin: 0, color: 'var(--text-primary)', lineHeight: '1.3' }}>
                    {evt.message}
                  </p>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
