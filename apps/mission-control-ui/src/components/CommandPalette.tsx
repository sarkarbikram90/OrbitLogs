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

import React, { useState, useEffect } from 'react';
import { useFleetStore } from '../stores/fleetStore';

interface CommandOption {
  id: string;
  title: string;
  category: 'NAVIGATION' | 'COMMAND' | 'SATELLITE';
  action: () => void;
  badge?: string;
}

export const CommandPalette: React.FC<{ isOpen: boolean; onClose: () => void }> = ({ isOpen, onClose }) => {
  const { setActiveTab, setSelectedSatId, satellites } = useFleetStore();
  const [query, setQuery] = useState('');

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        if (isOpen) onClose();
        else onClose(); // parent toggles state
      }
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const options: CommandOption[] = [
    { id: 'nav-dash', title: 'Open Fleet Dashboard Overview', category: 'NAVIGATION', action: () => { setActiveTab('DASHBOARD'); onClose(); }, badge: 'Tab 1' },
    { id: 'nav-fleet', title: 'Manage Satellites Fleet Registry', category: 'NAVIGATION', action: () => { setActiveTab('FLEET'); onClose(); }, badge: 'Tab 2' },
    { id: 'nav-cmd', title: 'Open Tactical Command Center', category: 'NAVIGATION', action: () => { setActiveTab('COMMANDS'); onClose(); }, badge: 'Tab 3' },
    { id: 'nav-plan', title: 'Launch Autonomous Mission Planner', category: 'NAVIGATION', action: () => { setActiveTab('PLANNER'); onClose(); }, badge: 'Tab 4' },
    { id: 'nav-ssa', title: 'View Space Situational Awareness & Debris', category: 'NAVIGATION', action: () => { setActiveTab('SSA'); onClose(); }, badge: 'Tab 7' },
    { id: 'nav-twin', title: 'Inspect Subsystem Digital Twin CAD', category: 'NAVIGATION', action: () => { setActiveTab('TWIN'); onClose(); }, badge: 'Tab 9' },
    { id: 'nav-billing', title: 'Enterprise SaaS Subscription & Invoicing', category: 'NAVIGATION', action: () => { setActiveTab('BILLING'); onClose(); }, badge: 'Tab 14' },
    ...satellites.map((s) => ({
      id: `sat-${s.id}`,
      title: `Select Spacecraft ${s.id} (Health: ${s.score}%)`,
      category: 'SATELLITE' as const,
      action: () => { setSelectedSatId(s.id); setActiveTab('DASHBOARD'); onClose(); },
      badge: s.state,
    })),
  ];

  const filtered = options.filter(
    (o) => o.title.toLowerCase().includes(query.toLowerCase()) || o.category.toLowerCase().includes(query.toLowerCase())
  );

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        width: '100vw',
        height: '100vh',
        backgroundColor: 'rgba(2, 4, 9, 0.85)',
        backdropFilter: 'blur(12px)',
        zIndex: 99999,
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'flex-start',
        paddingTop: '12vh',
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: '560px',
          maxWidth: '90vw',
          backgroundColor: 'rgba(6, 12, 24, 0.95)',
          border: '1px solid var(--cyber-cyan)',
          borderRadius: '10px',
          boxShadow: '0 0 40px rgba(0, 240, 255, 0.25)',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
        }}
        onClick={(e) => e.stopPropagation()}
        className="tech-corner-panel"
      >
        <div style={{ padding: '16px', borderBottom: '1px solid var(--border-color)', display: 'flex', alignItems: 'center', gap: '12px' }}>
          <span style={{ color: 'var(--cyber-cyan)', fontSize: '18px' }}>🔍</span>
          <input
            type="text"
            autoFocus
            placeholder="Type a command or spacecraft ID... (ESC to exit)"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{
              flex: 1,
              backgroundColor: 'transparent',
              border: 'none',
              color: '#fff',
              fontSize: '14px',
              fontFamily: 'var(--font-mono)',
              outline: 'none',
            }}
          />
          <span style={{ fontSize: '10px', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)', backgroundColor: 'rgba(255,255,255,0.05)', padding: '3px 6px', borderRadius: '3px' }}>
            ESC
          </span>
        </div>

        <div style={{ maxHeight: '360px', overflowY: 'auto', padding: '8px' }}>
          {filtered.length === 0 ? (
            <div style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
              No matching spacecraft or commands found.
            </div>
          ) : (
            filtered.map((item) => (
              <div
                key={item.id}
                onClick={item.action}
                style={{
                  padding: '12px 14px',
                  borderRadius: '6px',
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                  border: '1px solid transparent',
                  marginBottom: '4px',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.backgroundColor = 'rgba(0, 240, 255, 0.12)';
                  e.currentTarget.style.borderColor = 'rgba(0, 240, 255, 0.3)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.backgroundColor = 'transparent';
                  e.currentTarget.style.borderColor = 'transparent';
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                  <span style={{ fontSize: '10px', fontFamily: 'var(--font-display)', color: 'var(--cyber-cyan)', backgroundColor: 'rgba(0, 240, 255, 0.1)', padding: '2px 6px', borderRadius: '3px' }}>
                    {item.category}
                  </span>
                  <span style={{ fontSize: '13px', fontFamily: 'var(--font-sans)', color: 'var(--text-primary)', fontWeight: 500 }}>
                    {item.title}
                  </span>
                </div>
                {item.badge && (
                  <span style={{ fontSize: '10px', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)' }}>
                    {item.badge}
                  </span>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
