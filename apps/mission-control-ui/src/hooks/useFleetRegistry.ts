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
import { useState, useEffect, useCallback } from 'react';
import { useAuthStore } from '../stores/authStore';
import { API_CONFIG } from '../config';
import { RegisteredSat, RegisteredStation } from '../components/fleet/types';

export function useFleetRegistry() {
  const { jwtToken, apiKey } = useAuthStore();
  const [sats, setSats] = useState<RegisteredSat[]>([]);
  const [stations, setStations] = useState<RegisteredStation[]>([]);
  const [message, setMessage] = useState<{ text: string; isError: boolean } | null>(null);

  const tokenToUse = jwtToken || apiKey;

  const fetchFleetData = useCallback(async () => {
    try {
      const satsResp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/satellites`, {
        headers: { Authorization: `Bearer ${tokenToUse}` },
      });
      if (satsResp.ok) {
        const data = await satsResp.json();
        setSats(data.satellites || []);
      }

      const stnResp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/ground-stations`, {
        headers: { Authorization: `Bearer ${tokenToUse}` },
      });
      if (stnResp.ok) {
        const data = await stnResp.json();
        setStations(data.ground_stations || []);
      }
    } catch (err) {
      console.error('Failed to fetch registered fleet:', err);
    }
  }, [tokenToUse]);

  useEffect(() => {
    fetchFleetData();
  }, [fetchFleetData]);

  const addSatellite = async (payload: {
    satellite_id: string;
    satellite_name: string;
    norad_id?: number;
    orbit_type: string;
    tle_line1?: string;
    tle_line2?: string;
  }) => {
    setMessage(null);
    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/satellites`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${tokenToUse}`,
        },
        body: JSON.stringify(payload),
      });

      if (resp.ok) {
        setMessage({ text: `Satellite ${payload.satellite_id} registered successfully!`, isError: false });
        fetchFleetData();
        return true;
      } else {
        const err = await resp.json();
        setMessage({ text: err.message || 'Failed to register satellite', isError: true });
        return false;
      }
    } catch {
      setMessage({ text: 'Command service unreachable', isError: true });
      return false;
    }
  };

  const addStation = async (payload: {
    station_id: string;
    station_name: string;
    latitude: number;
    longitude: number;
    altitude_m: number;
  }) => {
    setMessage(null);
    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/ground-stations`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${tokenToUse}`,
        },
        body: JSON.stringify(payload),
      });

      if (resp.ok) {
        setMessage({ text: `Ground station ${payload.station_id} registered successfully!`, isError: false });
        fetchFleetData();
        return true;
      } else {
        const err = await resp.json();
        setMessage({ text: err.message || 'Failed to register station', isError: true });
        return false;
      }
    } catch {
      setMessage({ text: 'Command service unreachable', isError: true });
      return false;
    }
  };

  const deleteSatellite = async (id: string) => {
    if (!confirm(`Decommission satellite ${id}?`)) return;
    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/fleet/satellites/${id}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${tokenToUse}` },
      });
      if (resp.ok) {
        fetchFleetData();
      }
    } catch (err) {
      console.error(err);
    }
  };

  return {
    sats,
    stations,
    message,
    setMessage,
    addSatellite,
    addStation,
    deleteSatellite,
    fetchFleetData,
  };
}
