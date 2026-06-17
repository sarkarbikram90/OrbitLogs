import { useState, useEffect, useRef } from 'react';
import { FleetOverview } from './components/FleetOverview';
import { OrbitViewer } from './components/OrbitViewer';
import { SatelliteDetail } from './components/SatelliteDetail';
import { GroundStationStatus } from './components/GroundStationStatus';
import { ReplayControls } from './components/ReplayControls';
import { IncidentPanel } from './components/IncidentPanel';
import { CommandCenter } from './components/CommandCenter';
import { MissionTimelineView } from './components/MissionTimelineView';
import { DigitalTwinViewer } from './components/DigitalTwinViewer';
import { AiCopilot } from './components/AiCopilot';
import { MissionPlanner } from './components/MissionPlanner';
import { GroundOperations } from './components/GroundOperations';
import { PassSchedule } from './components/PassSchedule';
import { SsaDashboard } from './components/SsaDashboard';
import { FlightDynamics } from './components/FlightDynamics';
import { TrainingSimulator } from './components/TrainingSimulator';
import { AiMissionDirector } from './components/AiMissionDirector';
import type { Satellite, GroundStation, EventLog, ReplaySession, SatelliteHealth, TelemetryRecord } from './types';

const getBackendUrl = (envVal: string | undefined, defaultPort: string) => {
  if (envVal) return envVal;
  if (typeof window !== 'undefined') {
    const { protocol, hostname } = window.location;
    return `${protocol}//${hostname}:${defaultPort}`;
  }
  return `http://localhost:${defaultPort}`;
};

const BACKEND_REPLAY_URL = getBackendUrl(import.meta.env.VITE_REPLAY_API_URL, '8084');
const BACKEND_COLLECTOR_URL = getBackendUrl(import.meta.env.VITE_COLLECTOR_API_URL, '8080');
const BACKEND_CMD_URL = getBackendUrl(import.meta.env.VITE_COMMAND_API_URL, '8085');

const getInitialApiKey = () => {
  return import.meta.env.VITE_API_KEY || sessionStorage.getItem('VITE_API_KEY') || '';
};

const getInitialJwtToken = () => {
  return sessionStorage.getItem('JWT_TOKEN') || '';
};

export default function App() {
  const [apiKey, setApiKey] = useState<string>(getInitialApiKey());
  const [jwtToken, setJwtToken] = useState<string>(getInitialJwtToken());
  const [mode, setMode] = useState<'LIVE' | 'REPLAY'>('LIVE');
  const [activeTab, setActiveTab] = useState<'DASHBOARD' | 'COMMANDS' | 'PLANNER' | 'GROUND_OPS' | 'PASSES' | 'SSA' | 'FLIGHT_DYNAMICS' | 'TWIN' | 'TRAINING' | 'AI_DIRECTOR' | 'TIMELINE' | 'AI'>('DASHBOARD');
  const [dashboardMode, setDashboardMode] = useState<'OPERATOR' | 'EXECUTIVE'>('OPERATOR');
  const [selectedSatId, setSelectedSatId] = useState<string | null>('SAT-0001');
  const [utcTime, setUtcTime] = useState(new Date().toISOString().replace('T', ' ').substring(0, 19));

  useEffect(() => {
    const timer = setInterval(() => {
      setUtcTime(new Date().toISOString().replace('T', ' ').substring(0, 19));
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // Initialize fleet list with default objects
  const [satellites, setSatellites] = useState<Satellite[]>(
    Array.from({ length: 10 }).map((_, idx) => ({
      id: `SAT-${String(idx + 1).padStart(4, '0')}`,
      state: 'GREEN',
      score: 100,
    }))
  );

  // Initialize ground stations list
  const [groundStations, setGroundStations] = useState<GroundStation[]>([]);

  // Incidents log
  const [events, setEvents] = useState<EventLog[]>([]);

  // Telemetry details for selected satellite
  const [telemetry, setTelemetry] = useState<TelemetryRecord | null>(null);
  const [health, setHealth] = useState<SatelliteHealth | null>(null);

  // Active Replay session
  const [replaySession, setReplaySession] = useState<ReplaySession | null>(null);

  // Refs for managing connections
  const wsRef = useRef<WebSocket | null>(null);
  const sseRef = useRef<EventSource | null>(null);

  // Handle switching modes
  const handleToggleMode = (newMode: 'LIVE' | 'REPLAY') => {
    if (newMode === mode) return;

    // Clean up connections
    cleanupConnections();
    
    // Reset state relative to the mode
    setTelemetry(null);
    setHealth(null);
    setEvents([]);
    
    if (newMode === 'LIVE') {
      setReplaySession(null);
    }
    
    setMode(newMode);
  };

  const cleanupConnections = () => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    if (sseRef.current) {
      sseRef.current.close();
      sseRef.current = null;
    }
  };

  // Cleanup on unmount
  useEffect(() => {
    return () => cleanupConnections();
  }, []);

  // --- 1. LIVE MODE OPERATIONS ---
  useEffect(() => {
    if (mode !== 'LIVE') return;

    // A. Query initial state from replay engine (TimescaleDB reader)
    const fetchLiveDiagnostics = async () => {
      try {
        const satsResp = await fetch(`${BACKEND_REPLAY_URL}/api/satellites`);
        if (satsResp.ok) {
          const satsData = await satsResp.json() as SatelliteHealth[];
          // Update satellites health assessments
          if (satsData.length > 0) {
            setSatellites(prev =>
              prev.map(s => {
                const updated = satsData.find((d: SatelliteHealth) => d.satellite_id === s.id);
                return updated
                  ? { id: s.id, state: updated.overall_state as 'GREEN' | 'YELLOW' | 'RED', score: updated.overall_score }
                  : s;
              })
            );
          }
        }

        const gsResp = await fetch(`${BACKEND_REPLAY_URL}/api/ground-stations`);
        if (gsResp.ok) {
          const gsData = await gsResp.json() as GroundStation[];
          setGroundStations(gsData);
        }

        const evResp = await fetch(`${BACKEND_REPLAY_URL}/api/events`);
        if (evResp.ok) {
          const evData = await evResp.json() as EventLog[];
          setEvents(evData);
        }
      } catch (err) {
        console.error('Failed to poll initial live database state:', err);
      }
    };

    // Initial fetch
    fetchLiveDiagnostics();

    // Polling diagnostics states every 1.5 seconds
    const pollInterval = setInterval(fetchLiveDiagnostics, 1500);

    // B. WebSocket connection for real-time telemetry updates
    const connectWs = () => {
      if (!apiKey) return;
      cleanupConnections();
      const wsUrl = BACKEND_COLLECTOR_URL.replace(/^http/, 'ws') + `/ws?token=${encodeURIComponent(apiKey)}`;
      const ws = new WebSocket(wsUrl);
      
      ws.onmessage = (event) => {
        try {
          const rec = JSON.parse(event.data) as TelemetryRecord;
          // If the telemetry matches the selected satellite, update the telemetry panel
          if (rec.satellite_id === selectedSatId) {
            setTelemetry(rec);
          }
        } catch (e) {
          console.error('Failed to parse WebSocket telemetry JSON:', e);
        }
      };

      ws.onerror = (e) => {
        console.error('WebSocket error:', e);
      };

      ws.onclose = () => {
        console.log('WebSocket closed. Reconnecting in 3 seconds...');
        wsRef.current = null;
        if (mode === 'LIVE') {
          setTimeout(connectWs, 3000);
        }
      };

      wsRef.current = ws;
    };

    connectWs();

    return () => {
      clearInterval(pollInterval);
      cleanupConnections();
    };
  }, [mode, selectedSatId, apiKey]);

  // When selected satellite changes, update its telemetry from the live diagnostics
  useEffect(() => {
    if (mode !== 'LIVE' || !selectedSatId) return;
    
    // Fetch latest evaluated health record for the selected satellite
    const fetchSelectedHealth = async () => {
      try {
        const satsResp = await fetch(`${BACKEND_REPLAY_URL}/api/satellites`);
        if (satsResp.ok) {
          const satsData: SatelliteHealth[] = await satsResp.json();
          const target = satsData.find((d: SatelliteHealth) => d.satellite_id === selectedSatId);
          if (target) {
            setHealth(target);
          } else {
            // Default healthy
            setHealth({
              timestamp: new Date().toISOString(),
              satellite_id: selectedSatId || 'UNKNOWN',
              mission_id: 'NOMINAL',
              overall_state: 'GREEN',
              overall_score: 100,
              explanation: 'All subsystems operating within normal parameters.',
              power: { state: 'GREEN', score: 100, explanation: '' },
              thermal: { state: 'GREEN', score: 100, explanation: '' },
              communications: { state: 'GREEN', score: 100, explanation: '' },
              propulsion: { state: 'GREEN', score: 100, explanation: '' },
              adcs: { state: 'GREEN', score: 100, explanation: '' },
              payload: { state: 'GREEN', score: 100, explanation: '' },
              flight_computer: { state: 'GREEN', score: 100, explanation: '' },
            });
          }
        }
      } catch (err) {
        console.error('Error fetching satellite health:', err);
      }
    };

    fetchSelectedHealth();
    const interval = setInterval(fetchSelectedHealth, 1500);
    return () => clearInterval(interval);
  }, [mode, selectedSatId]);


  // --- 2. REPLAY MODE OPERATIONS ---

  const sessionId = replaySession?.session_id;

  // Connect to SSE stream when a replay session is active
  useEffect(() => {
    if (mode !== 'REPLAY' || !sessionId) return;

    if (sseRef.current) {
      sseRef.current.close();
    }

    console.log(`Connecting to Replay SSE stream for session ${sessionId}...`);
    const sse = new EventSource(`${BACKEND_REPLAY_URL}/api/replays/${sessionId}/stream?token=${encodeURIComponent(apiKey)}`);

    sse.addEventListener('state', (e: MessageEvent) => {
      try {
        const sessionState = JSON.parse(e.data) as ReplaySession;
        setReplaySession(sessionState);
      } catch (err) {
        console.error('Error parsing session state SSE:', err);
      }
    });

    sse.addEventListener('telemetry', (e: MessageEvent) => {
      try {
        const rec = JSON.parse(e.data) as TelemetryRecord;
        if (rec.satellite_id === selectedSatId) {
          setTelemetry(rec);
        }
      } catch (err) {
        console.error('Error parsing telemetry SSE:', err);
      }
    });

    sse.addEventListener('satellite_health', (e: MessageEvent) => {
      try {
        const hRec = JSON.parse(e.data) as SatelliteHealth;
        
        // Update satellites array
        setSatellites(prev =>
          prev.map(s => 
            s.id === hRec.satellite_id 
              ? { id: s.id, state: hRec.overall_state as 'GREEN' | 'YELLOW' | 'RED', score: hRec.overall_score }
              : s
          )
        );

        // Update selected satellite health
        if (hRec.satellite_id === selectedSatId) {
          setHealth(hRec);
        }
      } catch (err) {
        console.error('Error parsing health SSE:', err);
      }
    });

    sse.addEventListener('ground_station_health', (e: MessageEvent) => {
      try {
        const gsRec = JSON.parse(e.data) as GroundStation;
        setGroundStations(prev => {
          const index = prev.findIndex(g => g.station_id === gsRec.station_id);
          if (index >= 0) {
            const copy = [...prev];
            copy[index] = gsRec;
            return copy;
          } else {
            return [...prev, gsRec];
          }
        });
      } catch (err) {
        console.error('Error parsing ground station SSE:', err);
      }
    });

    sse.addEventListener('event', (e: MessageEvent) => {
      try {
        const evRec = JSON.parse(e.data) as EventLog;
        setEvents(prev => [evRec, ...prev].slice(0, 100)); // Cap at 100 elements
      } catch (err) {
        console.error('Error parsing event SSE:', err);
      }
    });

    sse.onerror = (err) => {
      console.warn('SSE disconnected or error occurred:', err);
    };

    sseRef.current = sse;

    return () => {
      sse.close();
      sseRef.current = null;
    };
  }, [mode, sessionId, selectedSatId, apiKey]);

  // Create Replay Session
  const handleCreateSession = async (
    satId: string | null,
    start: string,
    end: string,
    speed: number
  ) => {
    try {
      const resp = await fetch(`${BACKEND_REPLAY_URL}/api/replays`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`
        },
        body: JSON.stringify({
          satellite_id: satId,
          start_time: start,
          end_time: end,
          speed,
        }),
      });

      if (resp.ok) {
        const session = await resp.json() as ReplaySession;
        setReplaySession(session);
        setEvents([]); // Reset logs
        console.log('Successfully initialized replay session:', session.session_id);
      } else {
        const err = await resp.json();
        alert(`Failed to create session: ${err.error || 'Unknown error'}`);
      }
    } catch (err) {
      console.error('Failed to create replay session:', err);
      alert('Backend replay engine unreachable.');
    }
  };

  // Issue Replay Playback Control command
  const handleControl = async (action: string, position?: string, speed?: number) => {
    if (!replaySession) return;

    try {
      const resp = await fetch(
        `${BACKEND_REPLAY_URL}/api/replays/${replaySession.session_id}/control`,
        {
          method: 'POST',
          headers: { 
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${apiKey}`
          },
          body: JSON.stringify({ action, position, speed }),
        }
      );

      if (resp.ok) {
        const updated = await resp.json() as ReplaySession;
        setReplaySession(updated);
        if (action === 'stop') {
          // If reset, clear diagnostic screens
          setTelemetry(null);
          setHealth(null);
          setEvents([]);
        }
      } else {
        const err = await resp.json();
        console.warn('Control action failed:', err.error);
      }
    } catch (err) {
      console.error('Failed to control playback:', err);
    }
  };
  if (!apiKey || !jwtToken) {
    return (
      <LoginScreen
        onLoginSuccess={(key, token) => {
          setApiKey(key);
          setJwtToken(token);
        }}
      />
    );
  }

  return (
    <div className="app-container">
      {/* Header bar */}
      <header className="header">
        <div className="header-title-container">
          <span className="logo-icon">🛰</span>
          <span className="header-title">OrbitLogs - Mission Control</span>
        </div>

        {/* Tab Navigation */}
        <div style={{ display: 'flex', gap: '4px', margin: '0 16px', overflowX: 'auto', maxWidth: 'calc(100% - 600px)' }}>
          {([
            ['DASHBOARD', 'DASHBOARD'],
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
          ] as const).map(([tabId, label]) => (
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
                whiteSpace: 'nowrap'
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
              onChange={(e) => setDashboardMode(e.target.value as any)}
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
                fontWeight: 'bold'
              }}
            >
              <option value="OPERATOR">OPERATOR VIEW</option>
              <option value="EXECUTIVE">EXECUTIVE VIEW</option>
            </select>
            <div className={`mode-badge ${mode === 'LIVE' ? 'live' : 'replay'}`}>
              {mode === 'LIVE' ? '● LIVE' : '▼ REPLAY'}
            </div>
            <button
              onClick={() => {
                sessionStorage.clear();
                window.location.reload();
              }}
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

      {/* Main Workspace */}
      <main 
        className="main-content" 
        style={{
          display: activeTab === 'DASHBOARD' && dashboardMode === 'OPERATOR' ? 'grid' : 'block',
          height: '100%',
        }}
      >
        {activeTab === 'DASHBOARD' && dashboardMode === 'EXECUTIVE' && (
          <div style={{ display: 'grid', gridTemplateRows: 'auto 1fr', gap: '20px', height: '100%', width: '100%', color: '#f8fafc', padding: '20px', overflowY: 'auto' }} className="glass-panel">
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '16px' }}>
              <div style={{ padding: '16px', border: '1px solid var(--border-color)', borderRadius: '6px', backgroundColor: 'rgba(255,255,255,0.01)' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-display)' }}>CONSTELLATION READINESS</div>
                <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#10b981', marginTop: '8px', fontFamily: 'var(--font-mono)' }}>
                  {((satellites.filter(s => s.state === 'GREEN').length / satellites.length) * 100).toFixed(0)}%
                </div>
              </div>
              <div style={{ padding: '16px', border: '1px solid var(--border-color)', borderRadius: '6px', backgroundColor: 'rgba(255,255,255,0.01)' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-display)' }}>ACTIVE ORBITAL ASSETS</div>
                <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#0ea5e9', marginTop: '8px', fontFamily: 'var(--font-mono)' }}>
                  {satellites.length}
                </div>
              </div>
              <div style={{ padding: '16px', border: '1px solid var(--border-color)', borderRadius: '6px', backgroundColor: 'rgba(255,255,255,0.01)' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-display)' }}>ACTIVE GROUND STATIONS</div>
                <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#f59e0b', marginTop: '8px', fontFamily: 'var(--font-mono)' }}>
                  {groundStations.length}
                </div>
              </div>
              <div style={{ padding: '16px', border: '1px solid var(--border-color)', borderRadius: '6px', backgroundColor: 'rgba(255,255,255,0.01)' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-display)' }}>FLEET RISK SCORE</div>
                <div style={{ fontSize: '24px', fontWeight: 'bold', color: satellites.some(s => s.state === 'RED') ? '#ef4444' : '#10b981', marginTop: '8px', fontFamily: 'var(--font-mono)' }}>
                  {satellites.filter(s => s.state === 'RED').length > 0 ? 'HIGH' : satellites.filter(s => s.state === 'YELLOW').length > 0 ? 'ELEVATED' : 'LOW'}
                </div>
              </div>
            </div>
            <div style={{ marginTop: '20px' }}>
              <div style={{ fontWeight: 'bold', fontSize: '13px', fontFamily: 'var(--font-display)', color: '#38bdf8', marginBottom: '12px' }}>CONSTELLATION FLEET INVENTORY</div>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid var(--border-color)', color: '#38bdf8', textAlign: 'left' }}>
                    <th style={{ padding: '8px' }}>Spacecraft ID</th>
                    <th style={{ padding: '8px' }}>Operational Mode</th>
                    <th style={{ padding: '8px' }}>Subsystem Health Score</th>
                    <th style={{ padding: '8px' }}>Telemetry Link Status</th>
                  </tr>
                </thead>
                <tbody>
                  {satellites.map(sat => (
                    <tr key={sat.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                      <td style={{ padding: '12px 8px', fontWeight: 'bold' }}>{sat.id}</td>
                      <td style={{ padding: '12px 8px' }}>{sat.state === 'GREEN' ? 'NOMINAL' : sat.state === 'YELLOW' ? 'DEGRADED' : 'SAFE MODE'}</td>
                      <td style={{ padding: '12px 8px', color: sat.state === 'GREEN' ? '#10b981' : sat.state === 'YELLOW' ? '#f59e0b' : '#ef4444' }}>
                        {sat.score}/100
                      </td>
                      <td style={{ padding: '12px 8px' }}>
                        <span className={`status-indicator ${sat.state === 'GREEN' ? 'green' : sat.state === 'YELLOW' ? 'yellow' : 'red'}`} style={{ marginRight: '6px' }} />
                        {sat.state === 'GREEN' ? 'Telemetry Locked' : 'Degraded Signal'}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {activeTab === 'DASHBOARD' && dashboardMode === 'OPERATOR' && (
          <>
            {/* Left Column: Fleet Listing & Ground Stations */}
            <div style={{ display: 'grid', gridTemplateRows: '1fr 260px', gap: '16px', height: '100%', overflow: 'hidden' }}>
              <FleetOverview
                satellites={satellites}
                selectedSatId={selectedSatId}
                onSelectSat={(id) => setSelectedSatId(id)}
              />
              <GroundStationStatus groundStations={groundStations} />
            </div>

            {/* Center Column: 3D Visualization & Playback Timelines */}
            <div className="center-column">
              <OrbitViewer
                satellites={satellites}
                groundStations={groundStations}
                selectedSatId={selectedSatId}
              />
              <ReplayControls
                mode={mode}
                session={replaySession}
                satellites={satellites}
                onToggleMode={handleToggleMode}
                onCreateSession={handleCreateSession}
                onControl={handleControl}
              />
            </div>

            {/* Right Column: Diagnostic Cards & Events Log */}
            <div style={{ display: 'grid', gridTemplateRows: '1fr 280px', gap: '16px', height: '100%', overflow: 'hidden' }}>
              <SatelliteDetail
                satelliteId={selectedSatId}
                health={health}
                telemetry={telemetry}
              />
              <IncidentPanel events={events} />
            </div>
          </>
        )}

        {activeTab === 'COMMANDS' && (
          <div style={{ height: '100%' }}>
            <CommandCenter selectedSatId={selectedSatId} satellites={satellites} apiKey={apiKey} jwtToken={jwtToken} onJwtTokenChange={setJwtToken} />
          </div>
        )}

        {activeTab === 'PLANNER' && (
          <div style={{ height: '100%' }}>
            <MissionPlanner />
          </div>
        )}

        {activeTab === 'GROUND_OPS' && (
          <div style={{ height: '100%' }}>
            <GroundOperations />
          </div>
        )}

        {activeTab === 'PASSES' && (
          <div style={{ height: '100%' }}>
            <PassSchedule selectedSatId={selectedSatId} />
          </div>
        )}

        {activeTab === 'SSA' && (
          <div style={{ height: '100%' }}>
            <SsaDashboard selectedSatId={selectedSatId} />
          </div>
        )}

        {activeTab === 'FLIGHT_DYNAMICS' && (
          <div style={{ height: '100%' }}>
            <FlightDynamics selectedSatId={selectedSatId} />
          </div>
        )}

        {activeTab === 'TWIN' && (
          <div style={{ height: '100%' }}>
            <DigitalTwinViewer selectedSatId={selectedSatId} apiKey={apiKey} />
          </div>
        )}

        {activeTab === 'TRAINING' && (
          <div style={{ height: '100%' }}>
            <TrainingSimulator selectedSatId={selectedSatId} />
          </div>
        )}

        {activeTab === 'AI_DIRECTOR' && (
          <div style={{ height: '100%' }}>
            <AiMissionDirector selectedSatId={selectedSatId} />
          </div>
        )}

        {activeTab === 'TIMELINE' && (
          <div style={{ height: '100%' }}>
            <MissionTimelineView selectedSatId={selectedSatId} />
          </div>
        )}

        {activeTab === 'AI' && (
          <div style={{ height: '100%' }}>
            <AiCopilot selectedSatId={selectedSatId} apiKey={apiKey} />
          </div>
        )}
      </main>
    </div>
  );
}

// ==========================================
// SPACEX LOGIN CONTROL SCREEN & STYLES
// ==========================================

interface LoginScreenProps {
  onLoginSuccess: (apiKey: string, jwtToken: string) => void;
}

function LoginScreen({ onLoginSuccess }: LoginScreenProps) {
  const [keyInput, setKeyInput] = useState(import.meta.env.VITE_API_KEY || sessionStorage.getItem('VITE_API_KEY') || '');
  const [role, setRole] = useState<'OPERATOR' | 'FLIGHT_DIRECTOR' | 'MISSION_MANAGER' | 'ADMINISTRATOR'>('OPERATOR');
  const [statusLogs, setStatusLogs] = useState<string[]>([
    '[SYSTEM] Booting secure link protocol...',
    '[SYSTEM] Awaiting console credentials...'
  ]);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!keyInput.trim()) {
      setStatusLogs(prev => [...prev, '[ERROR] Access key cannot be empty.']);
      return;
    }

    setIsSubmitting(true);
    setStatusLogs(prev => [
      ...prev,
      `[SYSTEM] Connecting to Mission Command Service (8085)...`,
      `[SYSTEM] Authenticating as role ${role}...`
    ]);

    try {
      const resp = await fetch(`${BACKEND_CMD_URL}/api/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ api_key: keyInput, role }),
      });

      if (resp.ok) {
        const data = await resp.json() as { token: string };
        setStatusLogs(prev => [
          ...prev,
          `[SYSTEM] Cryptographic key verification successful.`,
          `[SYSTEM] JWT Token received. Session initialized.`,
          `[SYSTEM] Loading Flight Control Dashboard...`
        ]);
        
        sessionStorage.setItem('VITE_API_KEY', keyInput);
        sessionStorage.setItem('JWT_TOKEN', data.token);
        sessionStorage.setItem('CURRENT_ROLE', role);

        setTimeout(() => {
          onLoginSuccess(keyInput, data.token);
        }, 800);
      } else {
        const err = await resp.json() as { error?: string };
        setStatusLogs(prev => [
          ...prev,
          `[ERROR] Authentication failed: ${err.error || 'Invalid API Key specified.'}`
        ]);
        setIsSubmitting(false);
      }
    } catch (err) {
      setStatusLogs(prev => [
        ...prev,
        `[ERROR] Auth service offline. Ensure command service is running on port 8085.`
      ]);
      setIsSubmitting(false);
    }
  };

  return (
    <div style={loginBgStyle}>
      <div style={loginCardStyle}>
        <div style={loginHeaderStyle}>
          <span style={{ fontSize: '28px' }}>🛰</span>
          <h2 style={loginTitleStyle}>SPACEX • FLIGHT OPERATIONS</h2>
          <span style={loginSubTitleStyle}>ORBITLOGS CORE ACCESS CONTROL</span>
        </div>

        <form onSubmit={handleSubmit} style={loginFormStyle}>
          <div style={loginFormGroupStyle}>
            <label style={loginLabelStyle}>OPERATOR PASSKEY</label>
            <input
              type="password"
              value={keyInput}
              onChange={(e) => setKeyInput(e.target.value)}
              placeholder="ENTER SECURE CONSOLE API KEY"
              style={loginInputStyle}
              disabled={isSubmitting}
            />
          </div>

          <div style={loginFormGroupStyle}>
            <label style={loginLabelStyle}>CONSOLE ROLE CONTEXT</label>
            <select
              value={role}
              onChange={(e) => setRole(e.target.value as any)}
              style={loginSelectStyle}
              disabled={isSubmitting}
            >
              <option value="OPERATOR">Operator (Telemetry Uplink)</option>
              <option value="FLIGHT_DIRECTOR">Flight Director (Operations Approval)</option>
              <option value="MISSION_MANAGER">Mission Manager</option>
              <option value="ADMINISTRATOR">Administrator</option>
            </select>
          </div>

          <button type="submit" style={loginBtnStyle} disabled={isSubmitting}>
            {isSubmitting ? 'ESTABLISHING LINK...' : 'INITIALIZE CONSOLE UPLINK'}
          </button>
        </form>

        <div style={logBoxStyle}>
          <div style={logHeaderStyle}>TELEMETRY CONSOLE DIAGNOSTICS:</div>
          <div style={logContentStyle}>
            {statusLogs.map((log, index) => (
              <div
                key={index}
                style={{
                  color: log.startsWith('[ERROR]') ? '#ef4444' : log.includes('successful') || log.includes('Token') ? '#10b981' : '#38bdf8',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '11px',
                  marginBottom: '4px',
                }}
              >
                {log}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

const loginBgStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '100vw',
  height: '100vh',
  backgroundColor: '#020408',
  backgroundImage: 'radial-gradient(circle at center, #0f172a 0%, #020408 100%)',
  position: 'fixed',
  top: 0,
  left: 0,
  zIndex: 9999,
};

const loginCardStyle: React.CSSProperties = {
  width: '420px',
  padding: '32px',
  backgroundColor: 'rgba(8, 11, 20, 0.85)',
  backdropFilter: 'blur(16px)',
  WebkitBackdropFilter: 'blur(16px)',
  border: '1px solid rgba(56, 189, 248, 0.3)',
  borderRadius: '12px',
  boxShadow: '0 0 30px rgba(56, 189, 248, 0.15)',
  display: 'flex',
  flexDirection: 'column',
  gap: '24px',
};

const loginHeaderStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: '8px',
  textAlign: 'center',
};

const loginTitleStyle: React.CSSProperties = {
  margin: 0,
  fontFamily: 'var(--font-display)',
  fontSize: '18px',
  fontWeight: '900',
  letterSpacing: '2px',
  color: '#38bdf8',
  textShadow: '0 0 10px rgba(56, 189, 248, 0.3)',
};

const loginSubTitleStyle: React.CSSProperties = {
  fontSize: '10px',
  fontFamily: 'var(--font-display)',
  letterSpacing: '1.5px',
  color: '#64748b',
  fontWeight: 'bold',
};

const loginFormStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
};

const loginFormGroupStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
};

const loginLabelStyle: React.CSSProperties = {
  fontSize: '10px',
  fontFamily: 'var(--font-display)',
  letterSpacing: '1px',
  color: '#94a3b8',
  fontWeight: 'bold',
};

const loginInputStyle: React.CSSProperties = {
  backgroundColor: '#07090e',
  color: '#f8fafc',
  border: '1px solid rgba(56, 189, 248, 0.2)',
  borderRadius: '6px',
  padding: '12px',
  fontSize: '13px',
  fontFamily: 'var(--font-mono)',
  outline: 'none',
  transition: 'border-color 0.2s ease, box-shadow 0.2s ease',
};

const loginSelectStyle: React.CSSProperties = {
  backgroundColor: '#07090e',
  color: '#f8fafc',
  border: '1px solid rgba(56, 189, 248, 0.2)',
  borderRadius: '6px',
  padding: '12px',
  fontSize: '13px',
  outline: 'none',
  cursor: 'pointer',
};

const loginBtnStyle: React.CSSProperties = {
  backgroundColor: '#38bdf8',
  color: '#020408',
  border: 'none',
  borderRadius: '6px',
  padding: '12px',
  fontSize: '12px',
  fontFamily: 'var(--font-display)',
  fontWeight: '900',
  letterSpacing: '1px',
  cursor: 'pointer',
  transition: 'all 0.2s ease',
  marginTop: '8px',
  boxShadow: '0 0 15px rgba(56, 189, 248, 0.3)',
};

const logBoxStyle: React.CSSProperties = {
  backgroundColor: '#04060a',
  border: '1px solid rgba(56, 189, 248, 0.1)',
  borderRadius: '6px',
  padding: '12px',
  height: '120px',
  overflowY: 'auto',
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};

const logHeaderStyle: React.CSSProperties = {
  fontSize: '9px',
  fontFamily: 'var(--font-display)',
  color: '#64748b',
  letterSpacing: '0.5px',
  marginBottom: '6px',
  fontWeight: 'bold',
};

const logContentStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};
