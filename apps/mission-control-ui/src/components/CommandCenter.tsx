import React, { useState, useEffect } from 'react';
import type { Command } from '../types';

const BACKEND_CMD_URL = 'http://localhost:8085';

interface CommandCenterProps {
  selectedSatId: string | null;
  satellites: { id: string }[];
}

export const CommandCenter: React.FC<CommandCenterProps> = ({
  selectedSatId,
}) => {
  const [commands, setCommands] = useState<Command[]>([]);
  const [currentRole, setCurrentRole] = useState<'OPERATOR' | 'FLIGHT_DIRECTOR' | 'MISSION_MANAGER' | 'ADMINISTRATOR'>('OPERATOR');
  const [commandType, setCommandType] = useState('REBOOT_FC');
  const [shouldFail, setShouldFail] = useState(false);
  const [loading, setLoading] = useState(false);

  const fetchCommands = async () => {
    try {
      const resp = await fetch(`${BACKEND_CMD_URL}/api/commands`);
      if (resp.ok) {
        const data = await resp.json();
        setCommands(data);
      }
    } catch (err) {
      console.warn('Failed to fetch commands from command service');
    }
  };

  useEffect(() => {
    fetchCommands();
    const interval = setInterval(fetchCommands, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleCreateCommand = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedSatId) return;

    setLoading(true);
    try {
      const resp = await fetch(`${BACKEND_CMD_URL}/api/commands`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-User-Role': currentRole,
        },
        body: JSON.stringify({
          satellite_id: selectedSatId,
          command_type: commandType,
          parameters: { should_fail: shouldFail },
        }),
      });

      if (resp.ok) {
        fetchCommands();
      } else {
        const err = await resp.json();
        alert(`Failed to create command: ${err.error}`);
      }
    } catch (err) {
      alert('Error creating command. Service offline.');
    } finally {
      setLoading(false);
    }
  };

  const handleAction = async (id: string, action: 'approve' | 'queue' | 'execute') => {
    try {
      const resp = await fetch(`${BACKEND_CMD_URL}/api/commands/${id}/${action}`, {
        method: 'POST',
        headers: {
          'X-User-Role': currentRole,
        },
      });

      if (resp.ok) {
        fetchCommands();
      } else {
        const err = await resp.json();
        alert(`Action failed: ${err.error}`);
      }
    } catch (err) {
      alert('Action failed. Service offline.');
    }
  };

  return (
    <div style={containerStyle}>
      {/* Top Bar: RBAC Selector */}
      <div style={headerStyle}>
        <span>🛰 COMMAND CENTER</span>
        <div style={roleContainerStyle}>
          <span style={roleLabelStyle}>OPERATOR ROLE:</span>
          <select
            value={currentRole}
            onChange={(e) => setCurrentRole(e.target.value as any)}
            style={selectStyle}
          >
            <option value="OPERATOR">Operator</option>
            <option value="FLIGHT_DIRECTOR">Flight Director (Operations Approval)</option>
            <option value="MISSION_MANAGER">Mission Manager</option>
            <option value="ADMINISTRATOR">Administrator</option>
          </select>
        </div>
      </div>

      <div style={contentGridStyle}>
        {/* Left Side: Create Command Form */}
        <div style={cardStyle}>
          <h3 style={cardTitleStyle}>UPLINK COMMAND CREATOR</h3>
          <form onSubmit={handleCreateCommand} style={formStyle}>
            <div style={formGroupStyle}>
              <label style={labelStyle}>Target Spacecraft:</label>
              <input
                type="text"
                value={selectedSatId || 'Select a satellite from overview'}
                disabled
                style={inputStyle}
              />
            </div>

            <div style={formGroupStyle}>
              <label style={labelStyle}>Command Vector:</label>
              <select
                value={commandType}
                onChange={(e) => setCommandType(e.target.value)}
                style={inputStyle}
              >
                <option value="REBOOT_FC">REBOOT_FC (Reboot Flight Computer)</option>
                <option value="ALIGN_ADCS">ALIGN_ADCS (Align Inertial Guidance Stars)</option>
                <option value="DUMP_MOMENTUM">DUMP_MOMENTUM (Magnetic Desaturation of Wheels)</option>
                <option value="SAFE_MODE">SAFE_MODE (Enter Thermal Protection Safe Mode)</option>
                <option value="DEPLOY_SOLAR_ARRAYS">DEPLOY_SOLAR_ARRAYS (Deploy Primary Solar Arrays)</option>
              </select>
            </div>

            <div style={checkboxContainerStyle}>
              <input
                type="checkbox"
                id="should_fail"
                checked={shouldFail}
                onChange={(e) => setShouldFail(e.target.checked)}
                style={checkboxStyle}
              />
              <label htmlFor="should_fail" style={checkboxLabelStyle}>
                Simulate execution failure (exceed limits)
              </label>
            </div>

            <button
              type="submit"
              disabled={loading || !selectedSatId}
              style={{
                ...submitBtnStyle,
                opacity: selectedSatId ? 1 : 0.5,
              }}
            >
              REGISTER COMMAND
            </button>
          </form>
        </div>

        {/* Right Side: Command History & Actions */}
        <div style={{ ...cardStyle, flex: 2, overflowY: 'auto' }}>
          <h3 style={cardTitleStyle}>COMMUNICATION LINK AUDIT LOG</h3>
          <table style={tableStyle}>
            <thead>
              <tr style={tableHeaderRowStyle}>
                <th style={thStyle}>ID</th>
                <th style={thStyle}>SPACECRAFT</th>
                <th style={thStyle}>COMMAND VECTOR</th>
                <th style={thStyle}>STATUS</th>
                <th style={thStyle}>CREATED BY</th>
                <th style={thStyle}>ACTIONS</th>
              </tr>
            </thead>
            <tbody>
              {commands.length === 0 ? (
                <tr>
                  <td colSpan={6} style={emptyTdStyle}>
                    No registered telemetry uplink sequences found.
                  </td>
                </tr>
              ) : (
                commands.map((cmd) => (
                  <tr key={cmd.id} style={trStyle}>
                    <td style={tdIdStyle}>{cmd.id.substring(0, 8)}</td>
                    <td style={tdStyle}>{cmd.satellite_id}</td>
                    <td style={tdStyle}>
                      <code style={codeStyle}>{cmd.command_type}</code>
                    </td>
                    <td style={tdStyle}>
                      <span
                        style={{
                          ...badgeStyle,
                          backgroundColor:
                            cmd.status === 'VERIFIED'
                              ? '#059669'
                              : cmd.status === 'FAILED'
                              ? '#dc2626'
                              : cmd.status === 'TRANSMITTED' || cmd.status === 'RECEIVED'
                              ? '#2563eb'
                              : '#d97706',
                        }}
                      >
                        {cmd.status}
                      </span>
                    </td>
                    <td style={tdStyle}>{cmd.created_by}</td>
                    <td style={tdStyle}>
                      <div style={actionsContainerStyle}>
                        {cmd.status === 'CREATED' && (
                          <button
                            onClick={() => handleAction(cmd.id, 'approve')}
                            style={actionBtnStyle}
                          >
                            APPROVE
                          </button>
                        )}
                        {cmd.status === 'APPROVED' && (
                          <button
                            onClick={() => handleAction(cmd.id, 'queue')}
                            style={actionBtnStyle}
                          >
                            QUEUE
                          </button>
                        )}
                        {cmd.status === 'QUEUED' && (
                          <button
                            onClick={() => handleAction(cmd.id, 'execute')}
                            style={{ ...actionBtnStyle, backgroundColor: '#2563eb' }}
                          >
                            TRANSMIT
                          </button>
                        )}
                        {['TRANSMITTED', 'RECEIVED'].includes(cmd.status) && (
                          <span style={runningStyle}>EXECUTING...</span>
                        )}
                        {['VERIFIED', 'FAILED'].includes(cmd.status) && (
                          <span style={finishedStyle}>COMPLETE</span>
                        )}
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};

const containerStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  backgroundColor: '#020617',
  border: '1px solid #1e293b',
  borderRadius: '8px',
  overflow: 'hidden',
};

const headerStyle: React.CSSProperties = {
  padding: '12px 16px',
  backgroundColor: '#0b0f19',
  borderBottom: '1px solid #1e293b',
  fontSize: '14px',
  fontWeight: 'bold',
  letterSpacing: '1px',
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
};

const roleContainerStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
};

const roleLabelStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#94a3b8',
};

const selectStyle: React.CSSProperties = {
  backgroundColor: '#0f172a',
  color: '#f8fafc',
  border: '1px solid #3b82f6',
  borderRadius: '4px',
  padding: '4px 8px',
  fontSize: '11px',
  outline: 'none',
};

const contentGridStyle: React.CSSProperties = {
  display: 'flex',
  flex: 1,
  padding: '16px',
  gap: '16px',
  overflow: 'hidden',
};

const cardStyle: React.CSSProperties = {
  backgroundColor: '#0b0f19',
  border: '1px solid #1e293b',
  borderRadius: '6px',
  padding: '16px',
  display: 'flex',
  flexDirection: 'column',
};

const cardTitleStyle: React.CSSProperties = {
  fontSize: '12px',
  fontWeight: 'bold',
  letterSpacing: '1px',
  color: '#94a3b8',
  marginBottom: '16px',
  borderBottom: '1px solid #1e293b',
  paddingBottom: '8px',
};

const formStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
};

const formGroupStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
};

const labelStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#94a3b8',
};

const inputStyle: React.CSSProperties = {
  backgroundColor: '#020617',
  color: '#f8fafc',
  border: '1px solid #1e293b',
  borderRadius: '4px',
  padding: '8px',
  fontSize: '12px',
};

const checkboxContainerStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  marginTop: '4px',
};

const checkboxStyle: React.CSSProperties = {
  cursor: 'pointer',
};

const checkboxLabelStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#94a3b8',
  cursor: 'pointer',
};

const submitBtnStyle: React.CSSProperties = {
  marginTop: '12px',
  padding: '10px',
  backgroundColor: '#3b82f6',
  color: '#ffffff',
  border: 'none',
  borderRadius: '4px',
  fontWeight: 'bold',
  cursor: 'pointer',
  fontSize: '11px',
};

const tableStyle: React.CSSProperties = {
  width: '100%',
  borderCollapse: 'collapse',
  fontSize: '12px',
};

const tableHeaderRowStyle: React.CSSProperties = {
  borderBottom: '1px solid #1e293b',
};

const thStyle: React.CSSProperties = {
  textAlign: 'left',
  padding: '8px',
  color: '#64748b',
  fontWeight: 'bold',
};

const trStyle: React.CSSProperties = {
  borderBottom: '1px solid #0f172a',
};

const tdStyle: React.CSSProperties = {
  padding: '8px',
  color: '#f1f5f9',
};

const tdIdStyle: React.CSSProperties = {
  padding: '8px',
  color: '#94a3b8',
  fontFamily: 'monospace',
};

const codeStyle: React.CSSProperties = {
  fontFamily: 'monospace',
  color: '#3b82f6',
};

const badgeStyle: React.CSSProperties = {
  padding: '2px 6px',
  borderRadius: '4px',
  fontSize: '10px',
  fontWeight: 'bold',
  color: '#ffffff',
};

const emptyTdStyle: React.CSSProperties = {
  padding: '20px',
  textAlign: 'center',
  color: '#475569',
};

const actionsContainerStyle: React.CSSProperties = {
  display: 'flex',
  gap: '8px',
};

const actionBtnStyle: React.CSSProperties = {
  backgroundColor: '#d97706',
  color: '#ffffff',
  border: 'none',
  borderRadius: '4px',
  padding: '4px 8px',
  fontWeight: 'bold',
  fontSize: '10px',
  cursor: 'pointer',
};

const runningStyle: React.CSSProperties = {
  color: '#3b82f6',
  fontSize: '11px',
  fontStyle: 'italic',
};

const finishedStyle: React.CSSProperties = {
  color: '#475569',
  fontSize: '11px',
};
