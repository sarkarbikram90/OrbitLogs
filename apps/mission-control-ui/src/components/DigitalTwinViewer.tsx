import React, { useState, useEffect } from 'react';
import type { TwinHealthProjection, TelemetryRecord } from '../types';

const BACKEND_TWIN_URL = 'http://localhost:8086';

interface DigitalTwinViewerProps {
  selectedSatId: string | null;
}

export const DigitalTwinViewer: React.FC<DigitalTwinViewerProps> = ({
  selectedSatId,
}) => {
  const [twin, setTwin] = useState<TelemetryRecord | null>(null);
  const [projection, setProjection] = useState<TwinHealthProjection | null>(null);
  const [injecting, setInjecting] = useState(false);

  const fetchTwinData = async () => {
    if (!selectedSatId) return;

    try {
      // 1. Fetch current sync state
      const tResp = await fetch(`${BACKEND_TWIN_URL}/api/twin/${selectedSatId}`);
      if (tResp.ok) {
        const tData = await tResp.json();
        setTwin(tData);
      }

      // 2. Fetch health projection
      const pResp = await fetch(`${BACKEND_TWIN_URL}/api/twin/${selectedSatId}/health`);
      if (pResp.ok) {
        const pData = await pResp.json();
        setProjection(pData);
      }
    } catch (err) {
      console.warn('Failed to fetch twin data. Digital twin service offline.');
    }
  };

  useEffect(() => {
    fetchTwinData();
    const interval = setInterval(fetchTwinData, 1500);
    return () => clearInterval(interval);
  }, [selectedSatId]);

  const handleSimulateFailure = async (subsystem: string) => {
    if (!selectedSatId) return;
    setInjecting(true);
    try {
      const resp = await fetch(
        `${BACKEND_TWIN_URL}/api/twin/${selectedSatId}/simulate-failure`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ subsystem }),
        }
      );

      if (resp.ok) {
        fetchTwinData();
      } else {
        alert('Failed to inject simulated failure.');
      }
    } catch (err) {
      alert('Error contacting digital twin engine.');
    } finally {
      setInjecting(false);
    }
  };

  if (!selectedSatId) {
    return (
      <div style={emptyContainerStyle}>
        <span>Select a spacecraft from the Fleet Overview to spin up its Digital Twin virtual model.</span>
      </div>
    );
  }

  return (
    <div style={containerStyle}>
      {/* Header */}
      <div style={headerStyle}>
        <span>♊️ DIGITAL TWIN DIAGNOSTICS</span>
        <span style={subHeaderStyle}>Status: synchronized</span>
      </div>

      <div style={contentLayoutStyle}>
        {/* Left Column: Health Projection & Recommendation Engine */}
        <div style={leftColStyle}>
          <div style={cardStyle}>
            <h3 style={cardTitleStyle}>VIRTUAL STATE HARNESS</h3>
            {projection ? (
              <div style={projectionContainerStyle}>
                <div style={metricRowStyle}>
                  <span>Telemetry Sync:</span>
                  <span style={syncBadgeStyle}>ACTIVE</span>
                </div>
                <div style={metricRowStyle}>
                  <span>Constellation Vector:</span>
                  <span>LEO Circular (Inc: 53°)</span>
                </div>
                <div style={metricRowStyle}>
                  <span>Projected Mission Life:</span>
                  <span style={lifeStyle}>{projection.life_expectancy_days.toFixed(0)} Days</span>
                </div>

                <h4 style={subSectionHeaderStyle}>AI PREDICTIVE REMEDIATIONS</h4>
                {projection.recommendations.length === 0 ? (
                  <div style={noRecStyle}>No predictive degradation anomalies flagged. Subsystems nominal.</div>
                ) : (
                  <ul style={recListStyle}>
                    {projection.recommendations.map((rec, idx) => (
                      <li key={idx} style={recItemStyle}>
                        ⚠️ {rec}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            ) : (
              <div style={loadingStyle}>Calibrating health projection matrix...</div>
            )}
          </div>

          {/* Fault Injector Panel */}
          <div style={cardStyle}>
            <h3 style={cardTitleStyle}>ANOMALY / FAULT HARNESS INJECTOR</h3>
            <div style={gridBtnStyle}>
              <button
                disabled={injecting}
                onClick={() => handleSimulateFailure('power')}
                style={failBtnStyle}
              >
                SOLAR ARRAY LOCK
              </button>
              <button
                disabled={injecting}
                onClick={() => handleSimulateFailure('thermal')}
                style={failBtnStyle}
              >
                RADIATOR FLUID BLOCK
              </button>
              <button
                disabled={injecting}
                onClick={() => handleSimulateFailure('adcs')}
                style={failBtnStyle}
              >
                GUIDANCE DRIFT
              </button>
              <button
                disabled={injecting}
                onClick={() => handleSimulateFailure('communications')}
                style={failBtnStyle}
              >
                ANTENNA DECAY
              </button>
              <button
                disabled={injecting}
                onClick={() => handleSimulateFailure('propulsion')}
                style={failBtnStyle}
              >
                FUEL VENT LEAK
              </button>
            </div>
          </div>
        </div>

        {/* Right Column: Subsystem telemetry grid */}
        <div style={rightColStyle}>
          <div style={cardStyle}>
            <h3 style={cardTitleStyle}>VIRTUALIZED SUBSYSTEM DIAGNOSTICS</h3>
            {projection && twin ? (
              <div style={subsystemsGridStyle}>
                {/* Power */}
                <div style={subsystemBlockStyle}>
                  <div style={subHeaderRowStyle}>
                    <span>🔋 EPS (Electrical Power)</span>
                    <span
                      style={{
                        ...statusStyle,
                        color:
                          projection.subsystems.power.status === 'NOMINAL'
                            ? '#059669'
                            : '#d97706',
                      }}
                    >
                      {projection.subsystems.power.status}
                    </span>
                  </div>
                  <div style={subValueStyle}>
                    V_bat: {twin.battery_voltage?.toFixed(2) || '0.00'} V
                  </div>
                  <div style={subDetailsStyle}>{projection.subsystems.power.details}</div>
                </div>

                {/* Thermal */}
                <div style={subsystemBlockStyle}>
                  <div style={subHeaderRowStyle}>
                    <span>🌡 TCS (Thermal Control)</span>
                    <span
                      style={{
                        ...statusStyle,
                        color:
                          projection.subsystems.thermal.status === 'NOMINAL'
                            ? '#059669'
                            : '#dc2626',
                      }}
                    >
                      {projection.subsystems.thermal.status}
                    </span>
                  </div>
                  <div style={subValueStyle}>
                    Temp: {twin.temperature?.toFixed(1) || '0.0'} °C
                  </div>
                  <div style={subDetailsStyle}>{projection.subsystems.thermal.details}</div>
                </div>

                {/* ADCS */}
                <div style={subsystemBlockStyle}>
                  <div style={subHeaderRowStyle}>
                    <span>🧭 ADCS (Attitude Control)</span>
                    <span
                      style={{
                        ...statusStyle,
                        color:
                          projection.subsystems.adcs.status === 'NOMINAL'
                            ? '#059669'
                            : '#d97706',
                      }}
                    >
                      {projection.subsystems.adcs.status}
                    </span>
                  </div>
                  <div style={subValueStyle}>
                    Pitch: {twin.orientation_pitch?.toFixed(1) || '0.0'}°
                  </div>
                  <div style={subDetailsStyle}>{projection.subsystems.adcs.details}</div>
                </div>

                {/* Communications */}
                <div style={subsystemBlockStyle}>
                  <div style={subHeaderRowStyle}>
                    <span>📡 COMMS (RF Link)</span>
                    <span
                      style={{
                        ...statusStyle,
                        color:
                          projection.subsystems.communications.status === 'NOMINAL'
                            ? '#059669'
                            : '#d97706',
                      }}
                    >
                      {projection.subsystems.communications.status}
                    </span>
                  </div>
                  <div style={subValueStyle}>
                    PLoss: {twin.packet_loss?.toFixed(1) || '0.0'}%
                  </div>
                  <div style={subDetailsStyle}>{projection.subsystems.communications.details}</div>
                </div>
              </div>
            ) : (
              <div style={loadingStyle}>Connecting virtual spacecraft telemetry stream...</div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

const emptyContainerStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  height: '100%',
  backgroundColor: '#020617',
  border: '1px solid #1e293b',
  borderRadius: '8px',
  color: '#64748b',
  fontSize: '13px',
  padding: '20px',
  textAlign: 'center',
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

const subHeaderStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#059669',
};

const contentLayoutStyle: React.CSSProperties = {
  display: 'flex',
  flex: 1,
  padding: '16px',
  gap: '16px',
  overflow: 'hidden',
};

const leftColStyle: React.CSSProperties = {
  flex: 1,
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
  overflowY: 'auto',
};

const rightColStyle: React.CSSProperties = {
  flex: 1.2,
  display: 'flex',
  flexDirection: 'column',
  overflowY: 'auto',
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

const projectionContainerStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  fontSize: '12px',
};

const metricRowStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  color: '#f8fafc',
};

const syncBadgeStyle: React.CSSProperties = {
  color: '#059669',
  fontWeight: 'bold',
};

const lifeStyle: React.CSSProperties = {
  color: '#3b82f6',
  fontWeight: 'bold',
};

const subSectionHeaderStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#64748b',
  marginTop: '16px',
  marginBottom: '8px',
  fontWeight: 'bold',
  letterSpacing: '0.5px',
};

const noRecStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#475569',
};

const recListStyle: React.CSSProperties = {
  margin: 0,
  paddingLeft: '16px',
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
};

const recItemStyle: React.CSSProperties = {
  color: '#f59e0b',
  fontSize: '11px',
  lineHeight: '1.4',
};

const gridBtnStyle: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gap: '8px',
};

const failBtnStyle: React.CSSProperties = {
  padding: '10px 8px',
  backgroundColor: '#1e293b',
  border: '1px solid #dc2626',
  borderRadius: '4px',
  color: '#dc2626',
  fontSize: '10px',
  fontWeight: 'bold',
  cursor: 'pointer',
};

const loadingStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#475569',
  fontStyle: 'italic',
};

const subsystemsGridStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
};

const subsystemBlockStyle: React.CSSProperties = {
  backgroundColor: '#020617',
  border: '1px solid #1e293b',
  borderRadius: '6px',
  padding: '12px',
};

const subHeaderRowStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  fontSize: '11px',
  fontWeight: 'bold',
  color: '#94a3b8',
};

const statusStyle: React.CSSProperties = {
  fontSize: '10px',
};

const subValueStyle: React.CSSProperties = {
  fontSize: '18px',
  fontWeight: 'bold',
  color: '#f8fafc',
  margin: '8px 0',
  fontFamily: 'monospace',
};

const subDetailsStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#64748b',
  lineHeight: '1.4',
};
