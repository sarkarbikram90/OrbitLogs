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

import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { FleetOverview } from '../FleetOverview';
import type { Satellite } from '../../types';

const mockSatellites: Satellite[] = [
  { id: 'SAT-0001', state: 'GREEN', score: 100 },
  { id: 'SAT-0002', state: 'YELLOW', score: 82 },
  { id: 'SAT-0003', state: 'RED', score: 45 },
];

describe('FleetOverview Component', () => {
  it('renders satellite fleet overview list with state badges', () => {
    render(<FleetOverview satellites={mockSatellites} selectedSatId="SAT-0001" onSelectSat={() => {}} />);
    
    expect(screen.getByText('SAT-0001')).toBeInTheDocument();
    expect(screen.getByText('SAT-0002')).toBeInTheDocument();
    expect(screen.getByText('SAT-0003')).toBeInTheDocument();
    expect(screen.getByText('100%')).toBeInTheDocument();
    expect(screen.getByText('45%')).toBeInTheDocument();
  });

  it('triggers onSelectSat callback when clicking a satellite card', () => {
    const handleSelect = vi.fn();
    render(<FleetOverview satellites={mockSatellites} selectedSatId="SAT-0001" onSelectSat={handleSelect} />);

    const satCard = screen.getByText('SAT-0002');
    fireEvent.click(satCard);

    expect(handleSelect).toHaveBeenCalledWith('SAT-0002');
  });
});
