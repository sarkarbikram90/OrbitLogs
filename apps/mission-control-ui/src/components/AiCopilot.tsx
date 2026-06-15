import React, { useState } from 'react';

const BACKEND_AI_URL = 'http://localhost:8088';

interface AiCopilotProps {
  selectedSatId: string | null;
}

interface Message {
  sender: 'OPERATOR' | 'AI';
  text: string;
  sources?: string[];
}

export const AiCopilot: React.FC<AiCopilotProps> = ({ selectedSatId }) => {
  const [query, setQuery] = useState('');
  const [messages, setMessages] = useState<Message[]>([
    {
      sender: 'AI',
      text: 'Good day, Operator. I am your Space Operations AI Copilot. I can assist with subsystem diagnostic correlation, anomaly root cause analyses, and runbook retrieval. Ask me about SAT status or last 24h event logs.',
    },
  ]);
  const [loading, setLoading] = useState(false);

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;

    const userMsg = query;
    setMessages((prev) => [...prev, { sender: 'OPERATOR', text: userMsg }]);
    setQuery('');
    setLoading(true);

    try {
      const resp = await fetch(`${BACKEND_AI_URL}/api/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: userMsg,
          satellite_id: selectedSatId,
        }),
      });

      if (resp.ok) {
        const data = await resp.json();
        setMessages((prev) => [
          ...prev,
          { sender: 'AI', text: data.response, sources: data.sources },
        ]);
      } else {
        setMessages((prev) => [
          ...prev,
          { sender: 'AI', text: 'Error contacting AI Incident Commander.' },
        ]);
      }
    } catch (err) {
      setMessages((prev) => [
        ...prev,
        { sender: 'AI', text: 'AI Incident Commander service offline.' },
      ]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={panelStyle}>
      <div style={headerStyle}>
        <span>🤖 AI COPILOT CHAT</span>
        <span style={subHeaderStyle}>{selectedSatId ? `Focus: ${selectedSatId}` : 'Constellation Scope'}</span>
      </div>

      <div style={chatAreaStyle}>
        {messages.map((msg, idx) => (
          <div
            key={idx}
            style={{
              ...msgBubbleContainerStyle,
              justifyContent: msg.sender === 'OPERATOR' ? 'flex-end' : 'flex-start',
            }}
          >
            <div
              style={{
                ...msgBubbleStyle,
                backgroundColor: msg.sender === 'OPERATOR' ? '#1e293b' : '#0f172a',
                border: msg.sender === 'OPERATOR' ? '1px solid #3b82f6' : '1px solid #1e293b',
              }}
            >
              <div style={senderLabelStyle}>{msg.sender === 'OPERATOR' ? 'OPERATOR' : 'FLIGHT_COPILOT'}</div>
              <div style={textStyle}>{msg.text}</div>
              {msg.sources && msg.sources.length > 0 && (
                <div style={sourcesStyle}>
                  <strong>Sources:</strong> {msg.sources.join(', ')}
                </div>
              )}
            </div>
          </div>
        ))}
        {loading && <div style={loadingStyle}>Analyzing telemetry telemetry pools...</div>}
      </div>

      <form onSubmit={handleSend} style={inputFormStyle}>
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Ask AI Incident Commander (e.g. 'Why is satellite unhealthy?')"
          style={inputStyle}
          disabled={loading}
        />
        <button type="submit" style={btnStyle} disabled={loading}>
          SEND
        </button>
      </form>
    </div>
  );
};

const panelStyle: React.CSSProperties = {
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
  color: '#3b82f6',
};

const chatAreaStyle: React.CSSProperties = {
  flex: 1,
  padding: '16px',
  overflowY: 'auto',
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
};

const msgBubbleContainerStyle: React.CSSProperties = {
  display: 'flex',
  width: '100%',
};

const msgBubbleStyle: React.CSSProperties = {
  maxWidth: '80%',
  padding: '12px',
  borderRadius: '8px',
};

const senderLabelStyle: React.CSSProperties = {
  fontSize: '9px',
  fontWeight: 'bold',
  color: '#94a3b8',
  marginBottom: '4px',
};

const textStyle: React.CSSProperties = {
  fontSize: '13px',
  lineHeight: '1.5',
  whiteSpace: 'pre-wrap',
  color: '#f1f5f9',
};

const sourcesStyle: React.CSSProperties = {
  marginTop: '8px',
  fontSize: '10px',
  color: '#3b82f6',
  borderTop: '1px solid #334155',
  paddingTop: '4px',
};

const loadingStyle: React.CSSProperties = {
  fontSize: '12px',
  color: '#94a3b8',
  fontStyle: 'italic',
};

const inputFormStyle: React.CSSProperties = {
  display: 'flex',
  borderTop: '1px solid #1e293b',
};

const inputStyle: React.CSSProperties = {
  flex: 1,
  padding: '14px',
  backgroundColor: '#0b0f19',
  color: '#f8fafc',
  border: 'none',
  fontSize: '13px',
  outline: 'none',
};

const btnStyle: React.CSSProperties = {
  padding: '0 20px',
  backgroundColor: '#3b82f6',
  color: '#ffffff',
  border: 'none',
  fontWeight: 'bold',
  cursor: 'pointer',
  fontSize: '12px',
};
