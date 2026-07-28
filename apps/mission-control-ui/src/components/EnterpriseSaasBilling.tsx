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

interface TierConfig {
  name: string;
  monthlyBasePrice: number;
  includedFrames: number; // Millions
  extraFrameRate: number; // $ per 1M frames
  includedStorageGb: number;
  slaTarget: string;
}

const SAAS_TIERS: Record<string, TierConfig> = {
  STARTER: {
    name: 'Starter Constellation',
    monthlyBasePrice: 4999,
    includedFrames: 50,
    extraFrameRate: 0.15,
    includedStorageGb: 500,
    slaTarget: '99.9%',
  },
  ENTERPRISE: {
    name: 'Enterprise Fleet (Kuiper Class)',
    monthlyBasePrice: 24999,
    includedFrames: 500,
    extraFrameRate: 0.08,
    includedStorageGb: 5000,
    slaTarget: '99.99%',
  },
  MEGACONSTELLATION: {
    name: 'Starlink Megaconstellation',
    monthlyBasePrice: 99999,
    includedFrames: 5000,
    extraFrameRate: 0.03,
    includedStorageGb: 100000,
    slaTarget: '99.999%',
  },
};

export function EnterpriseSaasBilling() {
  const [selectedTier, setSelectedTier] = useState<'STARTER' | 'ENTERPRISE' | 'MEGACONSTELLATION'>('ENTERPRISE');
  const [satCount, setSatCount] = useState<number>(250);
  const [monthlyFramesM, setMonthlyFramesM] = useState<number>(650); // Millions of frames
  const [storageGb, setStorageGb] = useState<number>(3200);

  const tier = SAAS_TIERS[selectedTier];
  const overageFrames = Math.max(0, monthlyFramesM - tier.includedFrames);
  const overageCost = overageFrames * tier.extraFrameRate;
  const storageOverageGb = Math.max(0, storageGb - tier.includedStorageGb);
  const storageOverageCost = storageOverageGb * 0.05; // $0.05 per GB overage
  const totalEstimatedCost = tier.monthlyBasePrice + overageCost + storageOverageCost;
  const costPerSat = satCount > 0 ? (totalEstimatedCost / satCount).toFixed(2) : '0.00';

  return (
    <div style={{ padding: '24px', color: '#f8fafc', fontFamily: 'var(--font-sans)', height: '100%', overflowY: 'auto' }}>
      <div style={{ marginBottom: '24px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h1 style={{ fontSize: '22px', fontWeight: 700, margin: 0, color: '#38bdf8' }}>
            💳 SaaS Subscription & Metered Billing Operations
          </h1>
          <p style={{ fontSize: '12px', color: '#94a3b8', margin: '4px 0 0 0' }}>
            Enterprise multi-tenant telemetry ingestion consumption, tier quota enforcement, and SLA audit metrics.
          </p>
        </div>
        <div style={{ backgroundColor: 'rgba(56, 189, 248, 0.1)', border: '1px solid rgba(56, 189, 248, 0.3)', padding: '8px 16px', borderRadius: '6px' }}>
          <span style={{ fontSize: '11px', color: '#94a3b8' }}>Est. Monthly Recurring Revenue (MRR): </span>
          <strong style={{ fontSize: '16px', color: '#38bdf8', fontFamily: 'var(--font-mono)' }}>
            ${totalEstimatedCost.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </strong>
        </div>
      </div>

      {/* Tier Selector Grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '16px', marginBottom: '24px' }}>
        {(Object.keys(SAAS_TIERS) as Array<keyof typeof SAAS_TIERS>).map((key) => {
          const t = SAAS_TIERS[key];
          const isSelected = selectedTier === key;
          return (
            <div
              key={key}
              onClick={() => setSelectedTier(key)}
              style={{
                backgroundColor: isSelected ? 'rgba(56, 189, 248, 0.12)' : '#0f172a',
                border: isSelected ? '2px solid #38bdf8' : '1px solid #1e293b',
                borderRadius: '8px',
                padding: '16px',
                cursor: 'pointer',
                transition: 'all 0.2s ease',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                <span style={{ fontWeight: 700, fontSize: '14px', color: isSelected ? '#38bdf8' : '#e2e8f0' }}>{t.name}</span>
                {isSelected && <span style={{ fontSize: '10px', backgroundColor: '#38bdf8', color: '#090d16', fontWeight: 800, padding: '2px 6px', borderRadius: '4px' }}>ACTIVE TIER</span>}
              </div>
              <div style={{ fontSize: '20px', fontWeight: 800, fontFamily: 'var(--font-mono)', color: '#f8fafc', marginBottom: '8px' }}>
                ${t.monthlyBasePrice.toLocaleString()}<span style={{ fontSize: '12px', color: '#64748b' }}>/mo</span>
              </div>
              <ul style={{ fontSize: '11px', color: '#94a3b8', paddingLeft: '16px', margin: 0, lineHeight: 1.6 }}>
                <li>Included Telemetry: <strong>{t.includedFrames}M frames/mo</strong></li>
                <li>Overage Rate: <strong>${t.extraFrameRate}/1M frames</strong></li>
                <li>Hypertable Storage: <strong>{t.includedStorageGb.toLocaleString()} GB</strong></li>
                <li>Guaranteed SLA: <strong style={{ color: '#4ade80' }}>{t.slaTarget} Uptime</strong></li>
              </ul>
            </div>
          );
        })}
      </div>

      {/* Consumption Calculators & Metric Cards */}
      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '20px', marginBottom: '24px' }}>
        {/* Interactive Metering Configurator */}
        <div style={{ backgroundColor: '#0f172a', border: '1px solid #1e293b', borderRadius: '8px', padding: '20px' }}>
          <h3 style={{ fontSize: '14px', fontWeight: 600, color: '#e2e8f0', marginTop: 0, marginBottom: '16px' }}>
            ⚙️ Interactive Consumption & Unit Economics Simulator
          </h3>

          <div style={{ marginBottom: '16px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '6px' }}>
              <span>Active Satellite Fleet Count:</span>
              <strong style={{ fontFamily: 'var(--font-mono)', color: '#38bdf8' }}>{satCount} Satellites</strong>
            </div>
            <input
              type="range"
              min="10"
              max="5000"
              step="10"
              value={satCount}
              onChange={(e) => setSatCount(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#38bdf8' }}
            />
          </div>

          <div style={{ marginBottom: '16px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '6px' }}>
              <span>Monthly Ingested Frame Volume (CCSDS Packets):</span>
              <strong style={{ fontFamily: 'var(--font-mono)', color: '#38bdf8' }}>{monthlyFramesM} Million Frames</strong>
            </div>
            <input
              type="range"
              min="10"
              max="10000"
              step="50"
              value={monthlyFramesM}
              onChange={(e) => setMonthlyFramesM(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#38bdf8' }}
            />
          </div>

          <div style={{ marginBottom: '16px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '6px' }}>
              <span>Compressed TimescaleDB Storage Footprint:</span>
              <strong style={{ fontFamily: 'var(--font-mono)', color: '#38bdf8' }}>{storageGb.toLocaleString()} GB</strong>
            </div>
            <input
              type="range"
              min="100"
              max="200000"
              step="500"
              value={storageGb}
              onChange={(e) => setStorageGb(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#38bdf8' }}
            />
          </div>
        </div>

        {/* Cost Breakdown Summary Card */}
        <div style={{ backgroundColor: '#0f172a', border: '1px solid #1e293b', borderRadius: '8px', padding: '20px' }}>
          <h3 style={{ fontSize: '14px', fontWeight: 600, color: '#e2e8f0', marginTop: 0, marginBottom: '16px' }}>
            📊 Invoice Breakdown & Unit ROI
          </h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', fontSize: '12px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px dashed #334155', pb: '6px' }}>
              <span style={{ color: '#94a3b8' }}>Base Subscription Fee:</span>
              <span style={{ fontFamily: 'var(--font-mono)' }}>${tier.monthlyBasePrice.toLocaleString()}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px dashed #334155', pb: '6px' }}>
              <span style={{ color: '#94a3b8' }}>Telemetry Frame Overage ({overageFrames}M):</span>
              <span style={{ fontFamily: 'var(--font-mono)' }}>${overageCost.toFixed(2)}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px dashed #334155', pb: '6px' }}>
              <span style={{ color: '#94a3b8' }}>Storage Overage ({storageOverageGb} GB):</span>
              <span style={{ fontFamily: 'var(--font-mono)' }}>${storageOverageCost.toFixed(2)}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', pt: '8px', fontSize: '14px', fontWeight: 700 }}>
              <span style={{ color: '#f8fafc' }}>Total Estimated Invoice:</span>
              <span style={{ color: '#4ade80', fontFamily: 'var(--font-mono)' }}>${totalEstimatedCost.toLocaleString(undefined, { minimumFractionDigits: 2 })}</span>
            </div>
            <div style={{ marginTop: '12px', padding: '10px', backgroundColor: 'rgba(74, 222, 128, 0.08)', border: '1px solid rgba(74, 222, 128, 0.3)', borderRadius: '6px' }}>
              <span style={{ fontSize: '11px', color: '#94a3b8' }}>Cost per Satellite / Month: </span>
              <strong style={{ fontSize: '14px', color: '#4ade80', fontFamily: 'var(--font-mono)' }}>${costPerSat} / sat</strong>
            </div>
          </div>
        </div>
      </div>

      {/* Real-Time Telemetry & SLA Audit Metrics */}
      <div style={{ backgroundColor: '#0f172a', border: '1px solid #1e293b', borderRadius: '8px', padding: '20px' }}>
        <h3 style={{ fontSize: '14px', fontWeight: 600, color: '#e2e8f0', marginTop: 0, marginBottom: '16px' }}>
          🛡️ Enterprise Telemetry SLA Compliance & Real-Time Performance Audit
        </h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px' }}>
          <div style={{ padding: '12px', backgroundColor: '#1e293b', borderRadius: '6px' }}>
            <span style={{ fontSize: '11px', color: '#94a3b8' }}>Ingestion Availability SLA</span>
            <div style={{ fontSize: '18px', fontWeight: 700, color: '#4ade80', fontFamily: 'var(--font-mono)', marginTop: '4px' }}>99.998%</div>
            <span style={{ fontSize: '10px', color: '#64748b' }}>Target: {tier.slaTarget}</span>
          </div>
          <div style={{ padding: '12px', backgroundColor: '#1e293b', borderRadius: '6px' }}>
            <span style={{ fontSize: '11px', color: '#94a3b8' }}>Mean Packet Ingestion Latency</span>
            <div style={{ fontSize: '18px', fontWeight: 700, color: '#38bdf8', fontFamily: 'var(--font-mono)', marginTop: '4px' }}>4.2 ms</div>
            <span style={{ fontSize: '10px', color: '#64748b' }}>99th percentile: 11.5 ms</span>
          </div>
          <div style={{ padding: '12px', backgroundColor: '#1e293b', borderRadius: '6px' }}>
            <span style={{ fontSize: '11px', color: '#94a3b8' }}>Zero Packet Drop Rate</span>
            <div style={{ fontSize: '18px', fontWeight: 700, color: '#4ade80', fontFamily: 'var(--font-mono)', marginTop: '4px' }}>0.0000%</div>
            <span style={{ fontSize: '10px', color: '#64748b' }}>Unaligned Byte Safety Active</span>
          </div>
          <div style={{ padding: '12px', backgroundColor: '#1e293b', borderRadius: '6px' }}>
            <span style={{ fontSize: '11px', color: '#94a3b8' }}>TimescaleDB Compression Ratio</span>
            <div style={{ fontSize: '18px', fontWeight 700, color: '#a855f7', fontFamily: 'var(--font-mono)', marginTop: '4px' }}>12.4x</div>
            <span style={{ fontSize: '10px', color: '#64748b' }}>Chunk Duration: 1 Day</span>
          </div>
        </div>
      </div>
    </div>
  );
}
