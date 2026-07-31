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

import { useEffect } from 'react';
import { useAuthStore } from './stores/authStore';
import { useFleetStore } from './stores/fleetStore';
import { useTelemetryStore } from './stores/telemetryStore';
import { useReplayStore } from './stores/replayStore';
import { useWebSocket } from './hooks/useWebSocket';
import { useSSE } from './hooks/useSSE';
import { API_CONFIG } from './config';

import { Header } from './components/layout/Header';
import { SpaceBackdrop } from './components/SpaceBackdrop';
import { TelemetryHud } from './components/TelemetryHud';
import { DashboardPage } from './pages/DashboardPage';
import { FleetManagementPage } from './pages/FleetManagementPage';
import { CommandCenter } from './components/CommandCenter';
import { MissionPlanner } from './components/MissionPlanner';
import { GroundOperations } from './components/GroundOperations';
import { PassSchedule } from './components/PassSchedule';
import { SsaDashboard } from './components/SsaDashboard';
import { FlightDynamics } from './components/FlightDynamics';
import { DigitalTwinViewer } from './components/DigitalTwinViewer';
import { TrainingSimulator } from './components/TrainingSimulator';
import { AiMissionDirector } from './components/AiMissionDirector';
import { MissionTimelineView } from './components/MissionTimelineView';
import { AiCopilot } from './components/AiCopilot';
import { EnterpriseSaasBilling } from './components/EnterpriseSaasBilling';
import { ConjunctionRadar } from './components/ConjunctionRadar';
import { SpaceWeatherRadar } from './components/SpaceWeatherRadar';
import type { SatelliteHealth, GroundStation, EventLog, ReplaySession } from './types';

export default function App() {
  const { apiKey, jwtToken } = useAuthStore();
  const { mode, selectedSatId, activeTab, setMode, setSatellites, setGroundStations } = useFleetStore();
  const { setEvents, setHealth, clearTelemetry } = useTelemetryStore();
  const { replaySession, setReplaySession } = useReplayStore();

  useWebSocket();
  useSSE();

  // --- LIVE MODE DIAGNOSTICS POLLING ---
  useEffect(() => {
    if (mode !== 'LIVE') return;

    const fetchLiveDiagnostics = async () => {
      try {
        const satsResp = await fetch(`${API_CONFIG.REPLAY_URL}/api/satellites`);
        if (satsResp.ok) {
          const satsData = (await satsResp.json()) as SatelliteHealth[];
          if (satsData.length > 0) {
            setSatellites((prev) =>
              prev.map((s) => {
                const updated = satsData.find((d: SatelliteHealth) => d.satellite_id === s.id);
                return updated ? { id: s.id, state: updated.overall_state as 'GREEN' | 'YELLOW' | 'RED', score: updated.overall_score } : s;
              })
            );
          }
        }

        const gsResp = await fetch(`${API_CONFIG.REPLAY_URL}/api/ground-stations`);
        if (gsResp.ok) {
          const gsData = (await gsResp.json()) as GroundStation[];
          setGroundStations(gsData);
        }

        const evResp = await fetch(`${API_CONFIG.REPLAY_URL}/api/events`);
        if (evResp.ok) {
          const evData = (await evResp.json()) as EventLog[];
          setEvents(evData);
        }
      } catch (err) {
        console.error('Failed to poll live diagnostics:', err);
      }
    };

    fetchLiveDiagnostics();
    const interval = setInterval(fetchLiveDiagnostics, 1500);
    return () => clearInterval(interval);
  }, [mode, setSatellites, setGroundStations, setEvents]);

  // Handle switching modes
  const handleToggleMode = (newMode: 'LIVE' | 'REPLAY') => {
    if (newMode === mode) return;
    clearTelemetry();
    if (newMode === 'LIVE') setReplaySession(null);
    setMode(newMode);
  };

  // Create Replay Session
  const handleCreateSession = async (satId: string | null, start: string, end: string, speed: number) => {
    try {
      const resp = await fetch(`${API_CONFIG.REPLAY_URL}/api/replays`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey || jwtToken}`,
        },
        body: JSON.stringify({ satellite_id: satId, start_time: start, end_time: end, speed }),
      });

      if (resp.ok) {
        const session = (await resp.json()) as ReplaySession;
        setReplaySession(session);
        setEvents([]);
      } else {
        const err = await resp.json();
        alert(`Failed to create session: ${err.error || 'Unknown error'}`);
      }
    } catch {
      alert('Backend replay engine unreachable.');
    }
  };

  // Issue Replay Playback Control command
  const handleControl = async (action: string, position?: string, speed?: number) => {
    if (!replaySession) return;
    try {
      const resp = await fetch(`${API_CONFIG.REPLAY_URL}/api/replays/${replaySession.session_id}/control`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey || jwtToken}`,
        },
        body: JSON.stringify({ action, position, speed }),
      });

      if (resp.ok) {
        const updated = (await resp.json()) as ReplaySession;
        setReplaySession(updated);
        if (action === 'stop') {
          clearTelemetry();
        }
      }
    } catch (err) {
      console.error('Failed to control playback:', err);
    }
  };

  return (
    <div className="app-container">
      <SpaceBackdrop />
      <Header />
      <main className="main-content" style={{ display: 'block', height: '100%', position: 'relative' }}>
        <TelemetryHud />
        {activeTab === 'DASHBOARD' && (
          <DashboardPage onToggleMode={handleToggleMode} onCreateSession={handleCreateSession} onControl={handleControl} />
        )}
        {activeTab === 'FLEET' && <FleetManagementPage />}
        {activeTab === 'COMMANDS' && (
          <CommandCenter selectedSatId={selectedSatId} satellites={[]} apiKey={apiKey} jwtToken={jwtToken} onJwtTokenChange={() => {}} />
        )}
        {activeTab === 'PLANNER' && <MissionPlanner />}
        {activeTab === 'GROUND_OPS' && <GroundOperations />}
        {activeTab === 'PASSES' && <PassSchedule selectedSatId={selectedSatId} />}
        {activeTab === 'SSA' && <SsaDashboard selectedSatId={selectedSatId} />}
        {activeTab === 'CONJUNCTION' && <ConjunctionRadar selectedSatId={selectedSatId} />}
        {activeTab === 'SPACE_WX' && <SpaceWeatherRadar selectedSatId={selectedSatId} />}
        {activeTab === 'FLIGHT_DYNAMICS' && <FlightDynamics selectedSatId={selectedSatId} />}
        {activeTab === 'TWIN' && <DigitalTwinViewer selectedSatId={selectedSatId} apiKey={apiKey} />}
        {activeTab === 'TRAINING' && <TrainingSimulator selectedSatId={selectedSatId} />}
        {activeTab === 'AI_DIRECTOR' && <AiMissionDirector selectedSatId={selectedSatId} />}
        {activeTab === 'TIMELINE' && <MissionTimelineView selectedSatId={selectedSatId} />}
        {activeTab === 'AI' && <AiCopilot selectedSatId={selectedSatId} apiKey={apiKey} />}
        {activeTab === 'BILLING' && <EnterpriseSaasBilling />}
      </main>
    </div>
  );
}

