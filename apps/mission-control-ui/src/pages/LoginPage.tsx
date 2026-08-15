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

import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';
import { API_CONFIG } from '../config';
import { SpaceBackdrop } from '../components/SpaceBackdrop';

export function LoginPage() {
  const navigate = useNavigate();
  const { setAuth } = useAuthStore();
  const [loginMode, setLoginMode] = useState<'API_KEY' | 'ACCOUNT' | 'REGISTER'>('ACCOUNT');
  const [keyInput, setKeyInput] = useState(import.meta.env.VITE_API_KEY || sessionStorage.getItem('VITE_API_KEY') || '');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [tenantName, setTenantName] = useState('');
  const [tenantSlug, setTenantSlug] = useState('');
  const [role, setRole] = useState<'OPERATOR' | 'FLIGHT_DIRECTOR' | 'MISSION_MANAGER' | 'ADMINISTRATOR'>('OPERATOR');
  const [statusLogs, setStatusLogs] = useState<string[]>([
    '[SEC-UPLINK] Initializing encrypted telemetry handshake...',
    '[SEC-UPLINK] Quantum key exchange ready.',
    '[SYSTEM] Awaiting flight console credentials...',
  ]);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const fillDemoCredentials = () => {
    if (loginMode === 'ACCOUNT') {
      setEmail('flight.director@space-ops.in');
      setPassword('OrbitLogs2026!');
      setStatusLogs((prev) => [...prev, '[DEMO] Autofilled operator login credentials.']);
    } else if (loginMode === 'REGISTER') {
      setTenantName('ISRO Pixxel Constellation');
      setTenantSlug('isro-pixxel');
      setEmail('admin@pixxel.space');
      setPassword('SpaceOps2026!');
      setStatusLogs((prev) => [...prev, '[DEMO] Autofilled Indian space startup registration.']);
    } else {
      setKeyInput('default-dev-key-12345');
      setStatusLogs((prev) => [...prev, '[DEMO] Autofilled developer master passkey.']);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);

    if (loginMode === 'REGISTER') {
      if (!tenantName || !tenantSlug || !email || !password) {
        setStatusLogs((prev) => [...prev, '[ERROR] All fields are required for registration.']);
        setIsSubmitting(false);
        return;
      }
      setStatusLogs((prev) => [...prev, `[SYSTEM] Registering tenant '${tenantName}'...`]);

      try {
        const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/auth/register`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ tenant_name: tenantName, tenant_slug: tenantSlug, email, password }),
        });

        if (resp.ok) {
          const data = (await resp.json()) as { token: string; tenant_id: string; tenant_slug: string };
          setStatusLogs((prev) => [
            ...prev,
            '[SYSTEM] Tenant registration verified!',
            `[SYSTEM] Organization ID: ${data.tenant_id}`,
            '[SYSTEM] Uplink established. Navigating to Mission Control...',
          ]);
          setAuth('', data.token, data.tenant_id, data.tenant_slug, 'TenantAdmin');
          setTimeout(() => navigate('/'), 800);
        } else {
          const err = (await resp.json()) as { message?: string };
          setStatusLogs((prev) => [...prev, `[ERROR] Registration failed: ${err.message || 'Error creating tenant'}`]);
          setIsSubmitting(false);
        }
      } catch {
        // Fallback demo bypass for preview without backend running
        setStatusLogs((prev) => [
          ...prev,
          '[WARN] Mission command backend offline. Bypassing in DEMO MODE...',
          '[SYSTEM] Uplink established. Loading Console...',
        ]);
        setAuth('', 'demo-jwt-token', '00000000-0000-0000-0000-000000000001', tenantSlug || 'demo-tenant', 'TenantAdmin');
        setTimeout(() => navigate('/'), 800);
      }
      return;
    }

    if (loginMode === 'ACCOUNT') {
      if (!email || !password) {
        setStatusLogs((prev) => [...prev, '[ERROR] Email and password are required.']);
        setIsSubmitting(false);
        return;
      }
      setStatusLogs((prev) => [...prev, `[SYSTEM] Authenticating operator '${email}'...`]);

      try {
        const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/auth/login`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, password }),
        });

        if (resp.ok) {
          const data = (await resp.json()) as { token: string; tenant_id: string; tenant_slug: string };
          setStatusLogs((prev) => [
            ...prev,
            '[SYSTEM] User authenticated successfully.',
            `[SYSTEM] Organization: ${data.tenant_slug}`,
            '[SYSTEM] Session initialized.',
          ]);
          setAuth('', data.token, data.tenant_id, data.tenant_slug, 'Operator');
          setTimeout(() => navigate('/'), 800);
        } else {
          const err = (await resp.json()) as { message?: string };
          setStatusLogs((prev) => [...prev, `[ERROR] Auth failed: ${err.message || 'Invalid email/password'}`]);
          setIsSubmitting(false);
        }
      } catch {
        // Fallback demo mode if backend is not actively running during preview
        setStatusLogs((prev) => [
          ...prev,
          '[WARN] Command service offline. Bypassing in DEMO MODE...',
          '[SYSTEM] Console authenticated.',
        ]);
        setAuth('', 'demo-jwt-token', '00000000-0000-0000-0000-000000000001', 'orbitlogs-demo', 'Operator');
        setTimeout(() => navigate('/'), 800);
      }
      return;
    }

    // API_KEY mode
    if (!keyInput.trim()) {
      setStatusLogs((prev) => [...prev, '[ERROR] Access key cannot be empty.']);
      setIsSubmitting(false);
      return;
    }

    setStatusLogs((prev) => [...prev, `[SYSTEM] Authenticating passkey for role ${role}...`]);

    try {
      const resp = await fetch(`${API_CONFIG.COMMAND_URL}/api/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ api_key: keyInput, role }),
      });

      if (resp.ok) {
        const data = (await resp.json()) as { token: string; tenant_id: string; tenant_slug: string };
        setStatusLogs((prev) => [...prev, '[SYSTEM] Key verified.', '[SYSTEM] JWT Token received.', '[SYSTEM] Loading Console...']);
        setAuth(keyInput, data.token, data.tenant_id || '', data.tenant_slug || 'system', role);
        setTimeout(() => navigate('/'), 800);
      } else {
        const err = (await resp.json()) as { message?: string };
        setStatusLogs((prev) => [...prev, `[ERROR] Authentication failed: ${err.message || 'Invalid API key.'}`]);
        setIsSubmitting(false);
      }
    } catch {
      setStatusLogs((prev) => [...prev, '[WARN] Command service offline. Loading console in DEMO MODE...']);
      setAuth(keyInput, 'demo-jwt-token', '00000000-0000-0000-0000-000000000001', 'system', role);
      setTimeout(() => navigate('/'), 800);
    }
  };

  return (
    <div style={loginBgStyle}>
      <SpaceBackdrop />

      <div style={loginCardStyle} className="tech-corner-panel">
        <div style={loginHeaderStyle}>
          <span style={{ fontSize: '32px', filter: 'drop-shadow(0 0 12px var(--cyber-cyan))' }}>🛰</span>
          <h2 style={loginTitleStyle}>ORBITLOGS • FLIGHT OPERATIONS</h2>
          <span style={loginSubTitleStyle}>QUANTUM ENCRYPTED SPACECRAFT AUTHENTICATION</span>
        </div>

        {/* Mode Selector */}
        <div style={{ display: 'flex', gap: '8px', borderBottom: '1px solid var(--border-color)', paddingBottom: '10px' }}>
          {(['ACCOUNT', 'REGISTER', 'API_KEY'] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setLoginMode(m)}
              style={{
                flex: 1,
                padding: '8px 4px',
                fontSize: '10px',
                fontFamily: 'var(--font-display)',
                fontWeight: 800,
                backgroundColor: loginMode === m ? 'var(--cyber-cyan)' : 'transparent',
                color: loginMode === m ? '#020409' : 'var(--text-secondary)',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
                transition: 'all 0.2s ease',
              }}
            >
              {m === 'ACCOUNT' ? 'OPERATOR LOGIN' : m === 'REGISTER' ? 'REGISTER ORG' : 'API PASSKEY'}
            </button>
          ))}
        </div>

        <form onSubmit={handleSubmit} style={loginFormStyle}>
          {loginMode === 'REGISTER' && (
            <>
              <div style={loginFormGroupStyle}>
                <label style={loginLabelStyle}>ORGANIZATION NAME</label>
                <input
                  type="text"
                  value={tenantName}
                  onChange={(e) => {
                    setTenantName(e.target.value);
                    setTenantSlug(e.target.value.toLowerCase().replace(/[^a-z0-9]/g, '-'));
                  }}
                  placeholder="e.g. Pixxel Space Systems"
                  className="cyber-input"
                  disabled={isSubmitting}
                />
              </div>
              <div style={loginFormGroupStyle}>
                <label style={loginLabelStyle}>ORGANIZATION SLUG</label>
                <input
                  type="text"
                  value={tenantSlug}
                  onChange={(e) => setTenantSlug(e.target.value)}
                  placeholder="e.g. pixxel-space"
                  className="cyber-input"
                  disabled={isSubmitting}
                />
              </div>
            </>
          )}

          {(loginMode === 'ACCOUNT' || loginMode === 'REGISTER') && (
            <>
              <div style={loginFormGroupStyle}>
                <label style={loginLabelStyle}>EMAIL ADDRESS</label>
                <input
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="operator@orbitlogs.space"
                  className="cyber-input"
                  disabled={isSubmitting}
                />
              </div>
              <div style={loginFormGroupStyle}>
                <label style={loginLabelStyle}>PASSWORD</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="••••••••••••"
                  className="cyber-input"
                  disabled={isSubmitting}
                />
              </div>
            </>
          )}

          {loginMode === 'API_KEY' && (
            <>
              <div style={loginFormGroupStyle}>
                <label style={loginLabelStyle}>OPERATOR PASSKEY</label>
                <input
                  type="password"
                  value={keyInput}
                  onChange={(e) => setKeyInput(e.target.value)}
                  placeholder="ENTER SECURE CONSOLE API KEY"
                  className="cyber-input"
                  disabled={isSubmitting}
                />
              </div>

              <div style={loginFormGroupStyle}>
                <label style={loginLabelStyle}>CONSOLE ROLE CONTEXT</label>
                <select
                  value={role}
                  onChange={(e) => setRole(e.target.value as 'OPERATOR' | 'FLIGHT_DIRECTOR' | 'MISSION_MANAGER' | 'ADMINISTRATOR')}
                  className="cyber-input"
                  style={{ cursor: 'pointer' }}
                  disabled={isSubmitting}
                >
                  <option value="OPERATOR">Operator (Telemetry Uplink)</option>
                  <option value="FLIGHT_DIRECTOR">Flight Director (Operations Approval)</option>
                  <option value="MISSION_MANAGER">Mission Manager</option>
                  <option value="ADMINISTRATOR">Administrator</option>
                </select>
              </div>
            </>
          )}

          <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
            <button type="submit" className="cyber-btn" style={{ flex: 1, justifyContent: 'center', padding: '12px' }} disabled={isSubmitting}>
              {isSubmitting
                ? 'ESTABLISHING UPLINK...'
                : loginMode === 'REGISTER'
                ? 'CREATE TENANT ACCOUNT'
                : 'INITIALIZE CONSOLE UPLINK'}
            </button>

            <button
              type="button"
              onClick={fillDemoCredentials}
              className="cyber-btn-secondary"
              style={{ fontSize: '10px', padding: '0 12px' }}
              title="Auto-fill demo credentials"
            >
              ⚡ DEMO
            </button>
          </div>
        </form>

        <div style={logBoxStyle}>
          <div style={logHeaderStyle}>TELEMETRY HANDSHAKE DIAGNOSTICS:</div>
          <div style={logContentStyle}>
            {statusLogs.map((log, index) => (
              <div
                key={index}
                style={{
                  color: log.startsWith('[ERROR]')
                    ? 'var(--status-red)'
                    : log.includes('WARN')
                    ? 'var(--status-yellow)'
                    : log.includes('verified') || log.includes('established')
                    ? 'var(--status-green)'
                    : 'var(--cyber-cyan)',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '11px',
                  marginBottom: '2px',
                }}
              >
                {log}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

const loginBgStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '100vw',
  height: '100vh',
  position: 'fixed',
  top: 0,
  left: 0,
  zIndex: 9999,
};

const loginCardStyle: React.CSSProperties = {
  width: '460px',
  maxWidth: '92vw',
  padding: '32px',
  backgroundColor: 'rgba(6, 12, 24, 0.9)',
  backdropFilter: 'blur(24px)',
  WebkitBackdropFilter: 'blur(24px)',
  border: '1px solid var(--cyber-cyan)',
  borderRadius: '12px',
  boxShadow: '0 0 50px rgba(0, 240, 255, 0.25)',
  display: 'flex',
  flexDirection: 'column',
  gap: '20px',
  position: 'relative',
  zIndex: 10,
};

const loginHeaderStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: '8px',
  textAlign: 'center',
};

const loginTitleStyle: React.CSSProperties = {
  margin: 0,
  fontFamily: 'var(--font-display)',
  fontSize: '19px',
  fontWeight: 900,
  letterSpacing: '2px',
  color: 'var(--cyber-cyan)',
  textShadow: '0 0 14px var(--cyber-cyan-glow)',
};

const loginSubTitleStyle: React.CSSProperties = {
  fontSize: '9px',
  fontFamily: 'var(--font-mono)',
  letterSpacing: '1.5px',
  color: 'var(--text-muted)',
  fontWeight: 'bold',
};

const loginFormStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '14px',
};

const loginFormGroupStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
};

const loginLabelStyle: React.CSSProperties = {
  fontSize: '10px',
  fontFamily: 'var(--font-display)',
  letterSpacing: '1px',
  color: 'var(--text-secondary)',
  fontWeight: 'bold',
};

const logBoxStyle: React.CSSProperties = {
  backgroundColor: 'rgba(2, 4, 9, 0.95)',
  border: '1px solid rgba(0, 240, 255, 0.15)',
  borderRadius: '6px',
  padding: '12px',
  height: '95px',
  overflowY: 'auto',
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};

const logHeaderStyle: React.CSSProperties = {
  fontSize: '9px',
  fontFamily: 'var(--font-display)',
  color: 'var(--text-muted)',
  letterSpacing: '0.5px',
  marginBottom: '4px',
  fontWeight: 'bold',
};

const logContentStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '2px',
};
