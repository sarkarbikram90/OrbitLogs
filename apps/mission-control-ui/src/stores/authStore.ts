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
import { API_CONFIG } from '../config';

interface AuthState {
  apiKey: string;
  jwtToken: string;
  tenantId: string;
  tenantSlug: string;
  role: string;
  isAuthenticated: boolean;
  setAuth: (apiKey: string, jwtToken: string, tenantId?: string, tenantSlug?: string, role?: string) => void;
  login: (apiKey: string, role: string) => Promise<boolean>;
  loginWithPassword: (email: string, pass: string) => Promise<boolean>;
  logout: () => void;
}

const getInitialApiKey = () => import.meta.env.VITE_API_KEY || sessionStorage.getItem('VITE_API_KEY') || '';
const getInitialJwtToken = () => sessionStorage.getItem('JWT_TOKEN') || '';
const getInitialTenantId = () => sessionStorage.getItem('TENANT_ID') || '';
const getInitialTenantSlug = () => sessionStorage.getItem('TENANT_SLUG') || '';
const getInitialRole = () => sessionStorage.getItem('CURRENT_ROLE') || 'OPERATOR';

export const useAuthStore = create<AuthState>((set) => ({
  apiKey: getInitialApiKey(),
  jwtToken: getInitialJwtToken(),
  tenantId: getInitialTenantId(),
  tenantSlug: getInitialTenantSlug(),
  role: getInitialRole(),
  isAuthenticated: !!(getInitialApiKey() && getInitialJwtToken()),

  setAuth: (apiKey, jwtToken, tenantId = '', tenantSlug = '', role = 'OPERATOR') => {
    sessionStorage.setItem('VITE_API_KEY', apiKey);
    sessionStorage.setItem('JWT_TOKEN', jwtToken);
    sessionStorage.setItem('TENANT_ID', tenantId);
    sessionStorage.setItem('TENANT_SLUG', tenantSlug);
    sessionStorage.setItem('CURRENT_ROLE', role);
    set({ apiKey, jwtToken, tenantId, tenantSlug, role, isAuthenticated: true });
  },

  login: async (apiKey, role) => {
    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ api_key: apiKey, role }),
      });

      if (resp.ok) {
        const data = await resp.json() as { token: string; tenant_id?: string; tenant_slug?: string };
        const tenantId = data.tenant_id || '';
        const tenantSlug = data.tenant_slug || '';
        sessionStorage.setItem('VITE_API_KEY', apiKey);
        sessionStorage.setItem('JWT_TOKEN', data.token);
        sessionStorage.setItem('TENANT_ID', tenantId);
        sessionStorage.setItem('TENANT_SLUG', tenantSlug);
        sessionStorage.setItem('CURRENT_ROLE', role);
        set({ apiKey, jwtToken: data.token, tenantId, tenantSlug, role, isAuthenticated: true });
        return true;
      }
      return false;
    } catch {
      return false;
    }
  },

  loginWithPassword: async (email, password) => {
    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password }),
      });

      if (resp.ok) {
        const data = await resp.json() as { token: string; tenant_id: string; tenant_slug: string };
        sessionStorage.setItem('JWT_TOKEN', data.token);
        sessionStorage.setItem('TENANT_ID', data.tenant_id);
        sessionStorage.setItem('TENANT_SLUG', data.tenant_slug);
        sessionStorage.setItem('CURRENT_ROLE', 'TenantAdmin');
        set({ apiKey: '', jwtToken: data.token, tenantId: data.tenant_id, tenantSlug: data.tenant_slug, role: 'TenantAdmin', isAuthenticated: true });
        return true;
      }
      return false;
    } catch {
      return false;
    }
  },

  logout: () => {
    sessionStorage.clear();
    set({ apiKey: '', jwtToken: '', tenantId: '', tenantSlug: '', role: '', isAuthenticated: false });
  },
}));
