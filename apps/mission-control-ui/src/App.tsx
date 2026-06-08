import { useState, useEffect, useRef } from 'react';
import { FleetOverview } from './components/FleetOverview';
import { OrbitViewer } from './components/OrbitViewer';
import { SatelliteDetail } from './components/SatelliteDetail';
import { GroundStationStatus } from './components/GroundStationStatus';
import { ReplayControls } from './components/ReplayControls';
import { IncidentPanel } from './components/IncidentPanel';
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
const API_KEY = import.meta.env.VITE_API_KEY || 'secret-spaceops-key-2026';

export default function App() {
  const [mode, setMode] = useState<'LIVE' | 'REPLAY'>('LIVE');
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
      cleanupConnections();
      const wsUrl = BACKEND_COLLECTOR_URL.replace(/^http/, 'ws') + `/ws?token=${encodeURIComponent(API_KEY)}`;
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
  }, [mode, selectedSatId]);

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
    const sse = new EventSource(`${BACKEND_REPLAY_URL}/api/replays/${sessionId}/stream?token=${encodeURIComponent(API_KEY)}`);

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
  }, [mode, sessionId, selectedSatId]);

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
          'Authorization': `Bearer ${API_KEY}`
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
            'Authorization': `Bearer ${API_KEY}`
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

  return (
    <div className="app-container">
      {/* Header bar */}
      <header className="header">
        <div className="header-title-container">
          <span className="logo-icon">🛰</span>
          <span className="header-title">Open SpaceOps - Mission Control</span>
        </div>
        <div className="header-status">
          <span>UTC: {utcTime}</span>
          <div className={`mode-badge ${mode === 'LIVE' ? 'live' : 'replay'}`}>
            {mode === 'LIVE' ? '● LIVE CONSOLE' : '▼ REPLAY CONSOLE'}
          </div>
        </div>
      </header>

      {/* Main Grid Workspace */}
      <main className="main-content">
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
      </main>
    </div>
  );
}
