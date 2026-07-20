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
    '[SYSTEM] Booting secure link protocol...',
    '[SYSTEM] Awaiting console credentials...',
  ]);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);

    if (loginMode === 'REGISTER') {
      if (!tenantName || !tenantSlug || !email || !password) {
        setStatusLogs((prev) => [...prev, '[ERROR] All fields are required for registration.']);
        setIsSubmitting(false);
        return;
      }
      setStatusLogs((prev) => [...prev, `[SYSTEM] Registering organization '${tenantName}'...`]);

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
            '[SYSTEM] Tenant registration successful!',
            `[SYSTEM] Organization ID: ${data.tenant_id}`,
            '[SYSTEM] Navigating to Mission Control...',
          ]);
          setAuth('', data.token, data.tenant_id, data.tenant_slug, 'TenantAdmin');
          setTimeout(() => navigate('/'), 800);
        } else {
          const err = (await resp.json()) as { message?: string };
          setStatusLogs((prev) => [...prev, `[ERROR] Registration failed: ${err.message || 'Error creating tenant'}`]);
          setIsSubmitting(false);
        }
      } catch {
        setStatusLogs((prev) => [...prev, '[ERROR] Command service offline.']);
        setIsSubmitting(false);
      }
      return;
    }

    if (loginMode === 'ACCOUNT') {
      if (!email || !password) {
        setStatusLogs((prev) => [...prev, '[ERROR] Email and password are required.']);
        setIsSubmitting(false);
        return;
      }
      setStatusLogs((prev) => [...prev, `[SYSTEM] Authenticating user '${email}'...`]);

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
        setStatusLogs((prev) => [...prev, '[ERROR] Command service offline.']);
        setIsSubmitting(false);
      }
      return;
    }

    // API_KEY mode
    if (!keyInput.trim()) {
      setStatusLogs((prev) => [...prev, '[ERROR] Access key cannot be empty.']);
      setIsSubmitting(false);
      return;
    }

    setStatusLogs((prev) => [...prev, `[SYSTEM] Connecting to Mission Command Service...`, `[SYSTEM] Authenticating as role ${role}...`]);

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
      setStatusLogs((prev) => [...prev, '[ERROR] Auth service offline. Ensure command service is running on port 8085.']);
      setIsSubmitting(false);
    }
  };

  return (
    <div style={loginBgStyle}>
      <div style={loginCardStyle}>
        <div style={loginHeaderStyle}>
          <span style={{ fontSize: '28px' }}>🛰</span>
          <h2 style={loginTitleStyle}>ORBITLOGS • FLIGHT OPERATIONS</h2>
          <span style={loginSubTitleStyle}>ORBITLOGS CORE ACCESS CONTROL</span>
        </div>

        {/* Mode Selector */}
        <div style={{ display: 'flex', gap: '8px', borderBottom: '1px solid rgba(56, 189, 248, 0.2)', paddingBottom: '8px' }}>
          {(['ACCOUNT', 'REGISTER', 'API_KEY'] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setLoginMode(m)}
              style={{
                flex: 1,
                padding: '6px',
                fontSize: '10px',
                fontFamily: 'var(--font-display)',
                fontWeight: 'bold',
                backgroundColor: loginMode === m ? 'rgba(56, 189, 248, 0.2)' : 'transparent',
                color: loginMode === m ? '#38bdf8' : '#64748b',
                border: loginMode === m ? '1px solid #38bdf8' : '1px solid transparent',
                borderRadius: '4px',
                cursor: 'pointer',
              }}
            >
              {m === 'ACCOUNT' ? 'LOGIN' : m === 'REGISTER' ? 'REGISTER ORG' : 'API KEY'}
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
                  placeholder="e.g. Pixxel Space"
                  style={loginInputStyle}
                  disabled={isSubmitting}
                />
              </div>
              <div style={loginFormGroupStyle}>
                <label style={loginLabelStyle}>ORGANIZATION SLUG</label>
                <input
                  type="text"
                  value={tenantSlug}
                  onChange={(e) => setTenantSlug(e.target.value)}
                  placeholder="e.g. pixxel"
                  style={loginInputStyle}
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
                  placeholder="operator@company.com"
                  style={loginInputStyle}
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
                  style={loginInputStyle}
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
                  style={loginInputStyle}
                  disabled={isSubmitting}
                />
              </div>

              <div style={loginFormGroupStyle}>
                <label style={loginLabelStyle}>CONSOLE ROLE CONTEXT</label>
                <select
                  value={role}
                  onChange={(e) => setRole(e.target.value as any)}
                  style={loginSelectStyle}
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

          <button type="submit" style={loginBtnStyle} disabled={isSubmitting}>
            {isSubmitting
              ? 'PROCESSING...'
              : loginMode === 'REGISTER'
              ? 'CREATE TENANT ACCOUNT'
              : 'INITIALIZE CONSOLE UPLINK'}
          </button>
        </form>

        <div style={logBoxStyle}>
          <div style={logHeaderStyle}>TELEMETRY CONSOLE DIAGNOSTICS:</div>
          <div style={logContentStyle}>
            {statusLogs.map((log, index) => (
              <div
                key={index}
                style={{
                  color: log.startsWith('[ERROR]')
                    ? '#ef4444'
                    : log.includes('successful') || log.includes('verified')
                    ? '#10b981'
                    : '#38bdf8',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '11px',
                  marginBottom: '4px',
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
  backgroundColor: '#020408',
  backgroundImage: 'radial-gradient(circle at center, #0f172a 0%, #020408 100%)',
  position: 'fixed',
  top: 0,
  left: 0,
  zIndex: 9999,
};

const loginCardStyle: React.CSSProperties = {
  width: '420px',
  padding: '32px',
  backgroundColor: 'rgba(8, 11, 20, 0.85)',
  backdropFilter: 'blur(16px)',
  WebkitBackdropFilter: 'blur(16px)',
  border: '1px solid rgba(56, 189, 248, 0.3)',
  borderRadius: '12px',
  boxShadow: '0 0 30px rgba(56, 189, 248, 0.15)',
  display: 'flex',
  flexDirection: 'column',
  gap: '20px',
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
  fontSize: '18px',
  fontWeight: '900',
  letterSpacing: '2px',
  color: '#38bdf8',
  textShadow: '0 0 10px rgba(56, 189, 248, 0.3)',
};

const loginSubTitleStyle: React.CSSProperties = {
  fontSize: '10px',
  fontFamily: 'var(--font-display)',
  letterSpacing: '1.5px',
  color: '#64748b',
  fontWeight: 'bold',
};

const loginFormStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
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
  color: '#94a3b8',
  fontWeight: 'bold',
};

const loginInputStyle: React.CSSProperties = {
  backgroundColor: '#07090e',
  color: '#f8fafc',
  border: '1px solid rgba(56, 189, 248, 0.2)',
  borderRadius: '6px',
  padding: '12px',
  fontSize: '13px',
  fontFamily: 'var(--font-mono)',
  outline: 'none',
  transition: 'border-color 0.2s ease, box-shadow 0.2s ease',
};

const loginSelectStyle: React.CSSProperties = {
  backgroundColor: '#07090e',
  color: '#f8fafc',
  border: '1px solid rgba(56, 189, 248, 0.2)',
  borderRadius: '6px',
  padding: '12px',
  fontSize: '13px',
  outline: 'none',
  cursor: 'pointer',
};

const loginBtnStyle: React.CSSProperties = {
  backgroundColor: '#38bdf8',
  color: '#020408',
  border: 'none',
  borderRadius: '6px',
  padding: '12px',
  fontSize: '12px',
  fontFamily: 'var(--font-display)',
  fontWeight: '900',
  letterSpacing: '1px',
  cursor: 'pointer',
  transition: 'all 0.2s ease',
  marginTop: '8px',
  boxShadow: '0 0 15px rgba(56, 189, 248, 0.3)',
};

const logBoxStyle: React.CSSProperties = {
  backgroundColor: '#04060a',
  border: '1px solid rgba(56, 189, 248, 0.1)',
  borderRadius: '6px',
  padding: '12px',
  height: '100px',
  overflowY: 'auto',
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};

const logHeaderStyle: React.CSSProperties = {
  fontSize: '9px',
  fontFamily: 'var(--font-display)',
  color: '#64748b',
  letterSpacing: '0.5px',
  marginBottom: '6px',
  fontWeight: 'bold',
};

const logContentStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};
