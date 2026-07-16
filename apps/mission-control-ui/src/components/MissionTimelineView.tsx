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
import React, { useState, useEffect } from 'react';
import type { TimelineEvent } from '../types';

const BACKEND_TIMELINE_URL = 'http://localhost:8087';

interface MissionTimelineViewProps {
  selectedSatId: string | null;
}

export const MissionTimelineView: React.FC<MissionTimelineViewProps> = ({
  selectedSatId,
}) => {
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [sourceFilter, setSourceFilter] = useState<string>('');
  const [severityFilter, setSeverityFilter] = useState<string>('');
  const [searchQuery, setSearchQuery] = useState('');

  const fetchTimeline = async () => {
    try {
      let url = `${BACKEND_TIMELINE_URL}/api/timeline?`;
      if (selectedSatId) {
        url += `satellite_id=${selectedSatId}&`;
      }
      if (sourceFilter) {
        url += `source=${sourceFilter}&`;
      }
      if (severityFilter) {
        url += `severity=${severityFilter}&`;
      }
      const resp = await fetch(url);
      if (resp.ok) {
        const data = await resp.json();
        setEvents(data);
      }
    } catch (err) {
      console.warn('Failed to fetch timeline from timeline service');
    }
  };

  useEffect(() => {
    fetchTimeline();
    const interval = setInterval(fetchTimeline, 2000);
    return () => clearInterval(interval);
  }, [selectedSatId, sourceFilter, severityFilter]);

  const filteredEvents = events.filter((e) =>
    e.message.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div style={containerStyle}>
      <div style={headerStyle}>
        <span>🗂 MISSION EVENTS TIMELINE</span>
        <span style={subHeaderStyle}>{selectedSatId ? `Scope: ${selectedSatId}` : 'Constellation Scope'}</span>
      </div>

      {/* Filter Controls Bar */}
      <div style={filterBarStyle}>
        <div style={filterGroupStyle}>
          <span style={labelStyle}>Source:</span>
          <select
            value={sourceFilter}
            onChange={(e) => setSourceFilter(e.target.value)}
            style={selectStyle}
          >
            <option value="">ALL SOURCES</option>
            <option value="TELEMETRY">Telemetry</option>
            <option value="COMMAND">Command System</option>
            <option value="ALERT">Alerts</option>
            <option value="INCIDENT">Incidents</option>
            <option value="GROUND_STATION">Ground Stations</option>
            <option value="DIGITAL_TWIN">Digital Twin</option>
          </select>
        </div>

        <div style={filterGroupStyle}>
          <span style={labelStyle}>Severity:</span>
          <select
            value={severityFilter}
            onChange={(e) => setSeverityFilter(e.target.value)}
            style={selectStyle}
          >
            <option value="">ALL SEVERITIES</option>
            <option value="INFO">Info</option>
            <option value="WARNING">Warning</option>
            <option value="CRITICAL">Critical</option>
          </select>
        </div>

        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search timeline message text..."
          style={searchStyle}
        />
      </div>

      {/* Timeline Scroll Area */}
      <div style={timelineAreaStyle}>
        {filteredEvents.length === 0 ? (
          <div style={emptyStyle}>No timeline telemetry events recorded.</div>
        ) : (
          filteredEvents.map((evt) => (
            <div key={evt.event_id} style={eventRowStyle}>
              {/* Timestamp */}
              <div style={timeStyle}>
                {new Date(evt.timestamp).toISOString().substring(11, 19)}
              </div>

              {/* Source Indicator */}
              <div style={sourceIndicatorStyle}>
                <span
                  style={{
                    ...sourceBadgeStyle,
                    backgroundColor:
                      evt.source === 'COMMAND'
                        ? '#2563eb'
                        : evt.source === 'TELEMETRY'
                        ? '#059669'
                        : evt.source === 'DIGITAL_TWIN'
                        ? '#7c3aed'
                        : '#475569',
                  }}
                >
                  {evt.source}
                </span>
              </div>

              {/* Severity dot */}
              <div
                style={{
                  ...dotStyle,
                  backgroundColor:
                    evt.severity === 'CRITICAL'
                      ? '#dc2626'
                      : evt.severity === 'WARNING'
                      ? '#d97706'
                      : '#059669',
                }}
              />

              {/* Event Details */}
              <div style={messageStyle}>
                <span style={textStyle}>{evt.message}</span>
                {evt.details && (
                  <div style={detailsStyle}>
                    <pre style={preStyle}>{JSON.stringify(evt.details, null, 2)}</pre>
                  </div>
                )}
              </div>
            </div>
          ))
        )}
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

const subHeaderStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#3b82f6',
};

const filterBarStyle: React.CSSProperties = {
  padding: '8px 16px',
  backgroundColor: '#0b0f19',
  borderBottom: '1px solid #1e293b',
  display: 'flex',
  alignItems: 'center',
  gap: '16px',
};

const filterGroupStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
};

const labelStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#94a3b8',
};

const selectStyle: React.CSSProperties = {
  backgroundColor: '#0f172a',
  color: '#f8fafc',
  border: '1px solid #1e293b',
  borderRadius: '4px',
  padding: '4px 8px',
  fontSize: '11px',
  outline: 'none',
};

const searchStyle: React.CSSProperties = {
  flex: 1,
  backgroundColor: '#0f172a',
  color: '#f8fafc',
  border: '1px solid #1e293b',
  borderRadius: '4px',
  padding: '4px 8px',
  fontSize: '11px',
  outline: 'none',
};

const timelineAreaStyle: React.CSSProperties = {
  flex: 1,
  padding: '16px',
  overflowY: 'auto',
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
};

const emptyStyle: React.CSSProperties = {
  textAlign: 'center',
  color: '#475569',
  padding: '20px',
  fontSize: '12px',
};

const eventRowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'flex-start',
  padding: '8px',
  borderBottom: '1px solid #0f172a',
};

const timeStyle: React.CSSProperties = {
  width: '70px',
  fontSize: '11px',
  color: '#94a3b8',
  fontFamily: 'monospace',
  paddingTop: '2px',
};

const sourceIndicatorStyle: React.CSSProperties = {
  width: '100px',
  paddingTop: '1px',
};

const sourceBadgeStyle: React.CSSProperties = {
  fontSize: '9px',
  fontWeight: 'bold',
  color: '#ffffff',
  padding: '2px 6px',
  borderRadius: '4px',
};

const dotStyle: React.CSSProperties = {
  width: '8px',
  height: '8px',
  borderRadius: '50%',
  marginTop: '5px',
  marginRight: '12px',
  flexShrink: 0,
};

const messageStyle: React.CSSProperties = {
  flex: 1,
};

const textStyle: React.CSSProperties = {
  fontSize: '12.5px',
  color: '#f8fafc',
  lineHeight: '1.4',
};

const detailsStyle: React.CSSProperties = {
  marginTop: '6px',
  backgroundColor: '#0f172a',
  borderRadius: '4px',
  padding: '6px',
  border: '1px solid #1e293b',
  overflowX: 'auto',
};

const preStyle: React.CSSProperties = {
  margin: 0,
  fontSize: '10px',
  fontFamily: 'monospace',
  color: '#94a3b8',
};
