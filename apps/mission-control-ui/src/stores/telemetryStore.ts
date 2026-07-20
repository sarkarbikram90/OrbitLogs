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

import { create } from 'zustand';
import type { TelemetryRecord, SatelliteHealth, EventLog } from '../types';

interface TelemetryState {
  telemetry: TelemetryRecord | null;
  health: SatelliteHealth | null;
  events: EventLog[];
  setTelemetry: (telemetry: TelemetryRecord | null) => void;
  setHealth: (health: SatelliteHealth | null) => void;
  setEvents: (events: EventLog[] | ((prev: EventLog[]) => EventLog[])) => void;
  clearTelemetry: () => void;
}

export const useTelemetryStore = create<TelemetryState>((set) => ({
  telemetry: null,
  health: null,
  events: [],

  setTelemetry: (telemetry) => set({ telemetry }),
  setHealth: (health) => set({ health }),
  setEvents: (events) =>
    set((state) => ({
      events: typeof events === 'function' ? events(state.events) : events,
    })),

  clearTelemetry: () => set({ telemetry: null, health: null, events: [] }),
}));
