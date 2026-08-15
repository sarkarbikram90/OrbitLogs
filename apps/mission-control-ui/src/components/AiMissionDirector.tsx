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

interface Recommendation {
  category: string;
  title: string;
  description: string;
  reference_runbook: string;
  proposed_commands: string[];
}

interface DirectorStatus {
  satellite_id: string;
  operational_status: string;
  recommendations: Recommendation[];
}

interface PostmortemReport {
  satellite_id: string;
  generated_at: string;
  root_cause: string;
  impacted_subsystems: string[];
  commands_executed: string[];
  markdown_document: string;
}

interface AiMissionDirectorProps {
  selectedSatId: string | null;
}

const getBackendOpsUrl = () => {
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:8095`;
  }
  return `http://localhost:8095`;
};

export function AiMissionDirector({ selectedSatId }: AiMissionDirectorProps) {
  const [status, setStatus] = useState<DirectorStatus | null>(null);
  const [postmortem, setPostmortem] = useState<PostmortemReport | null>(null);
  const [loadingPostmortem, setLoadingPostmortem] = useState(false);
  const BACKEND_OPS_URL = getBackendOpsUrl();

  useEffect(() => {
    let isMounted = true;
    const fetchDirector = async () => {
      if (!selectedSatId) return;
      try {
        const resp = await fetch(`${BACKEND_OPS_URL}/api/director/${selectedSatId}`);
        if (resp.ok && isMounted) {
          const data = await resp.json();
          setStatus(data);
        }
      } catch (err) {
        console.error('Failed to fetch AI Director status:', err);
      }
    };

    fetchDirector();
    const interval = setInterval(fetchDirector, 5000);
    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, [selectedSatId, BACKEND_OPS_URL]);

  const handleGeneratePostmortem = async () => {
    if (!selectedSatId) return;
    setLoadingPostmortem(true);
    try {
      const resp = await fetch(`${BACKEND_OPS_URL}/api/postmortem/${selectedSatId}`);
      if (resp.ok) {
        const data = await resp.json();
        setPostmortem(data);
      }
    } catch (err) {
      console.error('Failed to compile postmortem:', err);
    } finally {
      setLoadingPostmortem(false);
    }
  };

  const handleDownload = () => {
    if (!postmortem) return;
    const blob = new Blob([postmortem.markdown_document], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `POSTMORTEM_${selectedSatId}_${new Date().toISOString().substring(0,10)}.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="glass-panel" style={{ height: '100%', padding: '20px', overflowY: 'auto' }}>
      <div className="panel-header" style={{ marginBottom: '20px', background: 'transparent', padding: 0 }}>
        <span>AI Mission Director & Incident Postmortem Compiler</span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '24px' }}>
        {/* Left: Advisory Panel */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8' }}>
              Real-Time RAG Advisory Feed
            </span>
            <span style={{
              fontSize: '11px',
              fontFamily: 'var(--font-mono)',
              fontWeight: 'bold',
              color: status?.operational_status === 'NOMINAL' ? '#10b981' : '#ef4444'
            }}>
              STATUS: {status?.operational_status || 'NOMINAL'}
            </span>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            {status?.recommendations.map((rec, i) => (
              <div key={i} style={{
                padding: '16px',
                backgroundColor: 'rgba(255, 255, 255, 0.01)',
                border: `1px solid ${rec.category === 'CRITICAL' ? 'rgba(239, 68, 68, 0.25)' : 'rgba(245, 158, 11, 0.25)'}`,
                borderRadius: '6px'
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
                  <span style={{ fontWeight: 'bold', fontSize: '12px', color: rec.category === 'CRITICAL' ? '#ef4444' : '#f59e0b' }}>
                    {rec.title}
                  </span>
                  <span style={{ fontSize: '9px', opacity: 0.6, fontFamily: 'var(--font-mono)' }}>{rec.category}</span>
                </div>
                <p style={{ fontSize: '11px', color: 'var(--text-secondary)', margin: '0 0 10px 0', lineHeight: '1.5' }}>
                  {rec.description}
                </p>
                <div style={{ fontSize: '10px', color: '#a855f7', fontFamily: 'var(--font-mono)', marginBottom: '8px' }}>
                  Runbook: {rec.reference_runbook}
                </div>
                <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                  {rec.proposed_commands.map((cmd) => (
                    <code key={cmd} style={{
                      backgroundColor: '#020408',
                      border: '1px solid rgba(255,255,255,0.05)',
                      padding: '2px 6px',
                      borderRadius: '4px',
                      fontSize: '10px',
                      color: '#c084fc'
                    }}>{cmd}</code>
                  ))}
                </div>
              </div>
            ))}
            {status?.recommendations.length === 0 && (
              <div style={{ color: 'var(--text-muted)', fontSize: '12px', padding: '20px 0', textAlign: 'center' }}>
                All systems verified nominal. No operational warnings active.
              </div>
            )}
          </div>
        </div>

        {/* Right: Postmortem Panel */}
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '16px',
          paddingLeft: '24px',
          borderLeft: '1px solid var(--border-color)'
        }}>
          <span style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8' }}>
            Incident Postmortem Generator
          </span>
          <p style={{ fontSize: '11px', color: 'var(--text-secondary)', lineHeight: '1.5', margin: 0 }}>
            Queries database history logs, telemetry outliers, and commands to compile a OrbitLogs Flight Operations standard report.
          </p>

          <button onClick={handleGeneratePostmortem} style={buttonStyle} disabled={loadingPostmortem}>
            {loadingPostmortem ? 'COMPILING TELEMETRY...' : 'GENERATE POSTMORTEM REPORT'}
          </button>

          {postmortem && (
            <div style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '12px',
              backgroundColor: 'rgba(255, 255, 255, 0.01)',
              border: '1px solid var(--border-color)',
              borderRadius: '6px',
              padding: '16px'
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ fontWeight: 'bold', fontSize: '12px', color: '#10b981' }}>Postmortem Compiled</span>
                <button onClick={handleDownload} style={downloadBtnStyle}>DOWNLOAD MARKDOWN</button>
              </div>

              <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <div>Root Cause: <span style={{ color: 'var(--text-secondary)' }}>{postmortem.root_cause}</span></div>
                <div>Systems Impacted: <span style={{ color: '#ef4444' }}>{postmortem.impacted_subsystems.join(', ')}</span></div>
                <div>Commands Executed: <span style={{ color: '#c084fc' }}>{postmortem.commands_executed.length} total</span></div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

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
  alignSelf: 'flex-start'
};

const downloadBtnStyle = {
  backgroundColor: 'transparent',
  color: '#10b981',
  border: '1px solid #10b981',
  borderRadius: '4px',
  padding: '4px 8px',
  fontSize: '10px',
  cursor: 'pointer',
  transition: 'all 0.2s ease'
};
