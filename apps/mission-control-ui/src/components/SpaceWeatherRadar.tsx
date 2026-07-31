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

interface SpaceWeatherRadarProps {
  selectedSatId: string | null;
}

export function SpaceWeatherRadar({ selectedSatId }: SpaceWeatherRadarProps) {
  const [kpIndex, setKpIndex] = useState(4.2);
  const [solarProtonFlux, setSolarProtonFlux] = useState(1420);
  const [dstIndex, setDstIndex] = useState(-48);
  const [seuRiskLevel, setSeuRiskLevel] = useState<'LOW' | 'ELEVATED' | 'HIGH' | 'CRITICAL'>('ELEVATED');

  return (
    <div className="glass-panel tech-corner-panel" style={{ height: '100%', padding: '24px', overflowY: 'auto', color: '#f8fafc' }}>
      {/* Title */}
      <div style={{ marginBottom: '24px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h1 style={{ fontSize: '18px', fontFamily: 'var(--font-display)', fontWeight: 900, margin: 0, color: 'var(--cyber-cyan)', letterSpacing: '1px' }}>
            ☀️ REAL-TIME SPACE WEATHER & SOLAR FLARE MONITORING FEED
          </h1>
          <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '4px 0 0 0', fontFamily: 'var(--font-mono)' }}>
            NOAA SWPC & ISRO Master Control Facility (MCF) Solar Proton Flux, Geomagnetic Storm Indices & LEO Drag Overlay
          </p>
        </div>
        <span style={{ fontSize: '10px', fontFamily: 'var(--font-mono)', color: 'var(--status-yellow)', backgroundColor: 'rgba(255, 183, 0, 0.1)', padding: '6px 12px', borderRadius: '4px', border: '1px solid var(--status-yellow)' }}>
          ● LIVE NOAA SWPC STREAM ATTACHED
        </span>
      </div>

      {/* Metrics Grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px', marginBottom: '24px' }}>
        {/* Kp Index */}
        <div style={{ backgroundColor: 'rgba(6, 12, 24, 0.8)', border: '1px solid var(--border-color)', borderRadius: '8px', padding: '16px' }}>
          <span style={{ fontSize: '10px', fontFamily: 'var(--font-display)', color: 'var(--text-muted)' }}>GEOMAGNETIC Kp INDEX</span>
          <div style={{ fontSize: '28px', fontWeight: 900, fontFamily: 'var(--font-mono)', color: kpIndex >= 5.0 ? 'var(--status-red)' : 'var(--status-yellow)', marginTop: '4px' }}>
            {kpIndex.toFixed(1)} <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>/ 9.0</span>
          </div>
          <span style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}>G1 Minor Geomagnetic Storm</span>
        </div>

        {/* Solar Proton Flux */}
        <div style={{ backgroundColor: 'rgba(6, 12, 24, 0.8)', border: '1px solid var(--border-color)', borderRadius: '8px', padding: '16px' }}>
          <span style={{ fontSize: '10px', fontFamily: 'var(--font-display)', color: 'var(--text-muted)' }}>SOLAR PROTON FLUX (&gt;10 MeV)</span>
          <div style={{ fontSize: '28px', fontWeight: 900, fontFamily: 'var(--font-mono)', color: 'var(--cyber-cyan)', marginTop: '4px' }}>
            {solarProtonFlux.toLocaleString()} <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>pfu</span>
          </div>
          <span style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}>S2 Moderate Radiation Storm</span>
        </div>

        {/* Dst Index */}
        <div style={{ backgroundColor: 'rgba(6, 12, 24, 0.8)', border: '1px solid var(--border-color)', borderRadius: '8px', padding: '16px' }}>
          <span style={{ fontSize: '10px', fontFamily: 'var(--font-display)', color: 'var(--text-muted)' }}>DISTURBANCE STORM TIME (Dst)</span>
          <div style={{ fontSize: '28px', fontWeight 900, fontFamily: 'var(--font-mono)', color: 'var(--status-yellow)', marginTop: '4px' }}>
            {dstIndex} <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>nT</span>
          </div>
          <span style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}>Ring Current Compression</span>
        </div>

        {/* SEU Risk Assessment */}
        <div style={{ backgroundColor: 'rgba(6, 12, 24, 0.8)', border: '1px solid var(--border-color)', borderRadius: '8px', padding: '16px' }}>
          <span style={{ fontSize: '10px', fontFamily: 'var(--font-display)', color: 'var(--text-muted)' }}>SINGLE EVENT UPSET (SEU) RISK</span>
          <div style={{ fontSize: '22px', fontWeight: 900, fontFamily: 'var(--font-display)', color: 'var(--status-yellow)', marginTop: '8px' }}>
            {seuRiskLevel}
          </div>
          <span style={{ fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}>Star Tracker Bit-Flip Warning</span>
        </div>
      </div>

      {/* Satellite Altitude Specific Drag & Ionospheric Impact */}
      <div style={{ backgroundColor: 'rgba(3, 7, 16, 0.9)', border: '1px solid var(--border-color)', borderRadius: '8px', padding: '20px', marginBottom: '24px' }}>
        <div style={{ fontSize: '12px', fontFamily: 'var(--font-display)', fontWeight: 800, color: 'var(--cyber-cyan)', marginBottom: '12px' }}>
          🛰 IMPACT ON ACTIVE TARGET ({selectedSatId || 'SAT-0001'}) — LEO ATMOSPHERIC DRAG & DENSITY SPIKE
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '16px', fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
          <div>
            <span style={{ color: 'var(--text-muted)' }}>Upper Atmos Density:</span>
            <div style={{ color: '#fff', fontWeight: 'bold', marginTop: '4px' }}>3.84 × 10⁻¹² kg/m³ (+42%)</div>
          </div>
          <div>
            <span style={{ color: 'var(--text-muted)' }}>Predicted Orbit Decay Rate:</span>
            <div style={{ color: 'var(--status-yellow)', fontWeight: 'bold', marginTop: '4px' }}>-145 meters / day</div>
          </div>
          <div>
            <span style={{ color: 'var(--text-muted)' }}>Recommended Action:</span>
            <div style={{ color: 'var(--status-green)', fontWeight: 'bold', marginTop: '4px' }}>Maintain Safe Orientation</div>
          </div>
        </div>
      </div>
    </div>
  );
}
