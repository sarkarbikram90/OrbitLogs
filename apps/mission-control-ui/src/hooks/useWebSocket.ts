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
import { API_CONFIG } from '../config';
import type { TelemetryRecord } from '../types';

export function useWebSocket() {
  const { apiKey, jwtToken } = useAuthStore();
  const { mode, selectedSatId } = useFleetStore();
  const { setTelemetry } = useTelemetryStore();
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    if (mode !== 'LIVE') {
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      return;
    }

    const tokenToUse = apiKey || jwtToken;
    if (!tokenToUse) return;

    const connectWs = () => {
      if (wsRef.current) {
        wsRef.current.close();
      }

      const wsUrl = API_CONFIG.COLLECTOR_URL.replace(/^http/, 'ws') + `/ws?token=${encodeURIComponent(tokenToUse)}`;
      const ws = new WebSocket(wsUrl);

      ws.onmessage = (event) => {
        try {
          const rec = JSON.parse(event.data) as TelemetryRecord;
          if (rec.satellite_id === selectedSatId) {
            setTelemetry(rec);
          }
        } catch (e) {
          console.error('WebSocket telemetry JSON parse error:', e);
        }
      };

      ws.onerror = (e) => {
        console.error('WebSocket error:', e);
      };

      ws.onclose = () => {
        wsRef.current = null;
      };

      wsRef.current = ws;
    };

    connectWs();

    return () => {
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [mode, selectedSatId, apiKey, jwtToken, setTelemetry]);
}
