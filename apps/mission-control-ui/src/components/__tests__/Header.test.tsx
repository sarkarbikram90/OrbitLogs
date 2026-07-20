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
import { describe, it, expect } from 'vitest';
import { Header } from '../layout/Header';
import { useAuthStore } from '../../stores/authStore';

describe('Header Component', () => {
  it('renders title and navigation tabs', () => {
    render(<Header />);
    expect(screen.getByText(/OrbitLogs - Mission Control/i)).toBeInTheDocument();
    expect(screen.getByText('DASHBOARD')).toBeInTheDocument();
    expect(screen.getByText('FLEET MGMT')).toBeInTheDocument();
  });

  it('allows switching view mode between OPERATOR and EXECUTIVE', () => {
    render(<Header />);
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'EXECUTIVE' } });
    expect(select).toHaveValue('EXECUTIVE');
  });

  it('triggers logout on DISCONNECT button click', () => {
    useAuthStore.setState({ isAuthenticated: true, apiKey: 'test', jwtToken: 'token' });
    render(<Header />);
    const disconnectBtn = screen.getByText('DISCONNECT');
    fireEvent.click(disconnectBtn);
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
  });
});
