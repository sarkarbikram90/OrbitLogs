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

import { useEffect, useRef } from 'react';
import { useAuthStore } from '../stores/authStore';
import { useFleetStore } from '../stores/fleetStore';
import { useTelemetryStore } from '../stores/telemetryStore';
import { useReplayStore } from '../stores/replayStore';
import { API_CONFIG } from '../config';
import type { ReplaySession, TelemetryRecord, SatelliteHealth, GroundStation, EventLog } from '../types';

export function useSSE() {
  const { apiKey, jwtToken } = useAuthStore();
  const { mode, selectedSatId, setSatellites, setGroundStations } = useFleetStore();
  const { setTelemetry, setHealth, setEvents } = useTelemetryStore();
  const { replaySession, setReplaySession } = useReplayStore();
  const sseRef = useRef<EventSource | null>(null);

  const sessionId = replaySession?.session_id;

  useEffect(() => {
    if (mode !== 'REPLAY' || !sessionId) {
      if (sseRef.current) {
        sseRef.current.close();
        sseRef.current = null;
      }
      return;
    }

    const tokenToUse = apiKey || jwtToken;
    const sse = new EventSource(`${API_CONFIG.REPLAY_URL}/api/replays/${sessionId}/stream?token=${encodeURIComponent(tokenToUse)}`);

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
        setSatellites((prev) =>
          prev.map((s) => (s.id === hRec.satellite_id ? { id: s.id, state: hRec.overall_state as 'GREEN' | 'YELLOW' | 'RED', score: hRec.overall_score } : s))
        );

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
        setGroundStations((prev) => {
          const index = prev.findIndex((g) => g.station_id === gsRec.station_id);
          if (index >= 0) {
            const copy = [...prev];
            copy[index] = gsRec;
            return copy;
          }
          return [...prev, gsRec];
        });
      } catch (err) {
        console.error('Error parsing ground station SSE:', err);
      }
    });

    sse.addEventListener('event', (e: MessageEvent) => {
      try {
        const evRec = JSON.parse(e.data) as EventLog;
        setEvents((prev) => [evRec, ...prev].slice(0, 100));
      } catch (err) {
        console.error('Error parsing event SSE:', err);
      }
    });

    sseRef.current = sse;

    return () => {
      sse.close();
      sseRef.current = null;
    };
  }, [mode, sessionId, selectedSatId, apiKey, jwtToken, setSatellites, setGroundStations, setTelemetry, setHealth, setEvents, setReplaySession]);
}
