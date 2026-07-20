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
import type { Satellite, GroundStation } from '../types';

interface FleetState {
  satellites: Satellite[];
  groundStations: GroundStation[];
  selectedSatId: string | null;
  mode: 'LIVE' | 'REPLAY';
  dashboardMode: 'OPERATOR' | 'EXECUTIVE';
  activeTab: string;
  setSatellites: (satellites: Satellite[] | ((prev: Satellite[]) => Satellite[])) => void;
  setGroundStations: (stations: GroundStation[] | ((prev: GroundStation[]) => GroundStation[])) => void;
  setSelectedSatId: (id: string | null) => void;
  setMode: (mode: 'LIVE' | 'REPLAY') => void;
  setDashboardMode: (mode: 'OPERATOR' | 'EXECUTIVE') => void;
  setActiveTab: (tab: string) => void;
}

const defaultSatellites: Satellite[] = Array.from({ length: 10 }).map((_, idx) => ({
  id: `SAT-${String(idx + 1).padStart(4, '0')}`,
  state: 'GREEN',
  score: 100,
}));

export const useFleetStore = create<FleetState>((set) => ({
  satellites: defaultSatellites,
  groundStations: [],
  selectedSatId: 'SAT-0001',
  mode: 'LIVE',
  dashboardMode: 'OPERATOR',
  activeTab: 'DASHBOARD',

  setSatellites: (satellites) =>
    set((state) => ({
      satellites: typeof satellites === 'function' ? satellites(state.satellites) : satellites,
    })),

  setGroundStations: (groundStations) =>
    set((state) => ({
      groundStations: typeof groundStations === 'function' ? groundStations(state.groundStations) : groundStations,
    })),

  setSelectedSatId: (selectedSatId) => set({ selectedSatId }),
  setMode: (mode) => set({ mode }),
  setDashboardMode: (dashboardMode) => set({ dashboardMode }),
  setActiveTab: (activeTab) => set({ activeTab }),
}));
