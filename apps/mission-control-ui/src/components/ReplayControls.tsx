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
import type { ReplaySession } from '../types';

interface ReplayControlsProps {
  mode: 'LIVE' | 'REPLAY';
  session: ReplaySession | null;
  satellites: { id: string }[];
  onToggleMode: (newMode: 'LIVE' | 'REPLAY') => void;
  onCreateSession: (satId: string | null, start: string, end: string, speed: number) => void;
  onControl: (action: string, position?: string, speed?: number) => void;
}

export const ReplayControls: React.FC<ReplayControlsProps> = ({
  mode,
  session,
  satellites,
  onToggleMode,
  onCreateSession,
  onControl,
}) => {
  // Session creation form state
  const [targetSat, setTargetSat] = useState<string>('ALL');
  
  // Set defaults: start_time = 1 hour ago, end_time = now
  const [startTime, setStartTime] = useState(() => new Date(Date.now() - 3600000).toISOString().slice(0, 16));
  const [endTime, setEndTime] = useState(() => new Date().toISOString().slice(0, 16));
  const initialSpeed = 1.0;

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault();
    const satId = targetSat === 'ALL' ? null : targetSat;
    // Convert local inputs to ISO strings
    const startIso = new Date(startTime).toISOString();
    const endIso = new Date(endTime).toISOString();
    onCreateSession(satId, startIso, endIso, initialSpeed);
  };

  // Timeline position calculation helpers
  const startTs = session ? new Date(session.start_time).getTime() : 0;
  const endTs = session ? new Date(session.end_time).getTime() : 0;
  const currentTs = session ? new Date(session.current_position).getTime() : 0;
  
  const progressPercent = endTs > startTs 
    ? ((currentTs - startTs) / (endTs - startTs)) * 100 
    : 0;

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!session) return;
    const seekTime = new Date(parseInt(e.target.value, 10)).toISOString();
    onControl('seek', seekTime);
  };

  const fmtTime = (isoString?: string) => {
    if (!isoString) return '--:--:--';
    return new Date(isoString).toISOString().replace('T', ' ').substring(0, 19);
  };

  return (
    <div className="glass-panel" style={{ height: '100%', padding: '12px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid var(--border-color)', paddingBottom: '8px', marginBottom: '10px' }}>
        <span style={{ fontFamily: 'var(--font-display)', fontSize: '13px', fontWeight: '700', letterSpacing: '1px', textTransform: 'uppercase' }}>
          Mission Timeline Controls
        </span>
        <div style={{ display: 'flex', gap: '6px' }}>
          <button
            onClick={() => onToggleMode('LIVE')}
            style={{
              padding: '4px 10px',
              borderRadius: '4px',
              border: mode === 'LIVE' ? '1px solid var(--status-green)' : '1px solid var(--border-color)',
              background: mode === 'LIVE' ? 'rgba(16, 185, 129, 0.15)' : 'transparent',
              color: mode === 'LIVE' ? 'var(--status-green)' : 'var(--text-secondary)',
              fontFamily: 'var(--font-display)',
              fontSize: '10px',
              fontWeight: '700',
              cursor: 'pointer',
              textShadow: mode === 'LIVE' ? '0 0 5px var(--status-green-glow)' : 'none',
            }}
          >
            LIVE STREAM
          </button>
          <button
            onClick={() => onToggleMode('REPLAY')}
            style={{
              padding: '4px 10px',
              borderRadius: '4px',
              border: mode === 'REPLAY' ? '1px solid var(--status-blue)' : '1px solid var(--border-color)',
              background: mode === 'REPLAY' ? 'rgba(14, 165, 233, 0.15)' : 'transparent',
              color: mode === 'REPLAY' ? 'var(--status-blue)' : 'var(--text-secondary)',
              fontFamily: 'var(--font-display)',
              fontSize: '10px',
              fontWeight: '700',
              cursor: 'pointer',
              textShadow: mode === 'REPLAY' ? '0 0 5px var(--status-blue-glow)' : 'none',
            }}
          >
            HISTORICAL REPLAY
          </button>
        </div>
      </div>

      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center' }}>
        {mode === 'LIVE' ? (
          <div style={{ textAlign: 'center', padding: '10px' }}>
            <div style={{ display: 'inline-block', position: 'relative', width: '12px', height: '12px', background: 'var(--status-green)', borderRadius: '50%', boxShadow: '0 0 8px var(--status-green)', marginRight: '8px', top: '1px' }} />
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: '13px', color: 'var(--text-primary)', fontWeight: '600' }}>
              STREAMING REAL-TIME MISSION UPDATES
            </span>
            <p style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '6px' }}>
              Connected to ingest websocket. Evaluating subsystem telemetries as they arrive.
            </p>
          </div>
        ) : (
          <div>
            {!session ? (
              /* Replay Creation Form */
              <form onSubmit={handleCreate} style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 100px', gap: '8px', alignItems: 'end' }}>
                <div style={formGroupStyle}>
                  <label style={labelStyle}>Target Satellite</label>
                  <select
                    value={targetSat}
                    onChange={e => setTargetSat(e.target.value)}
                    style={inputStyle}
                  >
                    <option value="ALL">All Satellites</option>
                    {satellites.map(sat => (
                      <option key={sat.id} value={sat.id}>{sat.id}</option>
                    ))}
                  </select>
                </div>
                <div style={formGroupStyle}>
                  <label style={labelStyle}>Start Time (UTC)</label>
                  <input
                    type="datetime-local"
                    value={startTime}
                    onChange={e => setStartTime(e.target.value)}
                    style={inputStyle}
                    required
                  />
                </div>
                <div style={formGroupStyle}>
                  <label style={labelStyle}>End Time (UTC)</label>
                  <input
                    type="datetime-local"
                    value={endTime}
                    onChange={e => setEndTime(e.target.value)}
                    style={inputStyle}
                    required
                  />
                </div>
                <button
                  type="submit"
                  style={{
                    height: '28px',
                    borderRadius: '4px',
                    border: '1px solid var(--status-blue)',
                    background: 'rgba(14, 165, 233, 0.1)',
                    color: 'var(--status-blue)',
                    fontFamily: 'var(--font-display)',
                    fontSize: '11px',
                    fontWeight: '700',
                    cursor: 'pointer',
                  }}
                >
                  INITIALIZE
                </button>
              </form>
            ) : (
              /* Playback Controller */
              <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                  <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                    <span className={`status-indicator ${session.status === 'PLAYING' ? 'blue' : 'yellow'}`} />
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: '11px', fontWeight: '700', textTransform: 'uppercase' }}>
                      Status: {session.status} | Speed: {session.speed}x
                    </span>
                  </div>
                  <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)' }}>
                    {fmtTime(session.current_position)}
                  </div>
                </div>

                {/* Timeline slider scrubbing */}
                <div style={{ position: 'relative', width: '100%', marginBottom: '12px' }}>
                  <input
                    type="range"
                    min={startTs}
                    max={endTs}
                    value={currentTs}
                    onChange={handleSliderChange}
                    style={{
                      width: '100%',
                      accentColor: 'var(--status-blue)',
                      background: `linear-gradient(to right, var(--status-blue) 0%, var(--status-blue) ${progressPercent}%, var(--bg-tertiary) ${progressPercent}%, var(--bg-tertiary) 100%)`,
                      height: '6px',
                      borderRadius: '3px',
                      cursor: 'pointer',
                      WebkitAppearance: 'none',
                    }}
                  />
                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '9px', color: 'var(--text-muted)', marginTop: '4px', fontFamily: 'var(--font-mono)' }}>
                    <span>{fmtTime(session.start_time)}</span>
                    <span>{fmtTime(session.end_time)}</span>
                  </div>
                </div>

                {/* Action Controls & Speeds */}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div style={{ display: 'flex', gap: '6px' }}>
                    <button
                      onClick={() => onControl('play')}
                      disabled={session.status === 'PLAYING'}
                      style={session.status === 'PLAYING' ? activeControlStyle : controlBtnStyle}
                    >
                      ▶ PLAY
                    </button>
                    <button
                      onClick={() => onControl('pause')}
                      disabled={session.status === 'PAUSED'}
                      style={session.status === 'PAUSED' ? activeControlStyle : controlBtnStyle}
                    >
                      ▮▮ PAUSE
                    </button>
                    <button
                      onClick={() => onControl('stop')}
                      style={controlBtnStyle}
                    >
                      ◼ RESET
                    </button>
                  </div>

                  <div style={{ display: 'flex', gap: '4px', alignItems: 'center' }}>
                    <span style={{ fontSize: '10px', color: 'var(--text-muted)', textTransform: 'uppercase', marginRight: '4px', fontFamily: 'var(--font-display)' }}>SPEED:</span>
                    {[1, 5, 10, 50, 100].map(s => {
                      const isCurrent = Math.abs(session.speed - s) < 0.1;
                      return (
                        <button
                          key={s}
                          onClick={() => onControl('speed', undefined, s)}
                          style={{
                            padding: '2px 6px',
                            border: isCurrent ? '1px solid var(--status-blue)' : '1px solid var(--border-color)',
                            background: isCurrent ? 'rgba(14, 165, 233, 0.15)' : 'transparent',
                            color: isCurrent ? 'var(--status-blue)' : 'var(--text-secondary)',
                            borderRadius: '3px',
                            fontSize: '9px',
                            fontFamily: 'var(--font-mono)',
                            cursor: 'pointer',
                            fontWeight: '700',
                          }}
                        >
                          {s}x
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

const formGroupStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};

const labelStyle: React.CSSProperties = {
  fontSize: '9px',
  color: 'var(--text-muted)',
  textTransform: 'uppercase',
  fontFamily: 'var(--font-display)',
};

const inputStyle: React.CSSProperties = {
  background: 'rgba(26, 36, 56, 0.5)',
  border: '1px solid var(--border-color)',
  borderRadius: '4px',
  color: 'var(--text-primary)',
  height: '28px',
  padding: '0 8px',
  fontSize: '11px',
  outline: 'none',
  fontFamily: 'var(--font-sans)',
};

const controlBtnStyle: React.CSSProperties = {
  background: 'rgba(26, 36, 56, 0.4)',
  border: '1px solid var(--border-color)',
  color: 'var(--text-primary)',
  padding: '4px 10px',
  borderRadius: '4px',
  fontSize: '10px',
  fontFamily: 'var(--font-display)',
  fontWeight: '700',
  cursor: 'pointer',
};

const activeControlStyle: React.CSSProperties = {
  ...controlBtnStyle,
  background: 'rgba(56, 189, 248, 0.12)',
  border: '1px solid rgba(56, 189, 248, 0.4)',
  color: '#38BDF8',
};
