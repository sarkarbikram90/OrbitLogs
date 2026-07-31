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
  monthlyBasePriceInr: number;
  includedFramesM: number; // Millions
  extraFrameRateInr: number; // ₹ per 1K frames
  includedStorageGb: number;
  slaTarget: string;
}

const SAAS_TIERS: Record<string, TierConfig> = {
  ACADEMIC: {
    name: 'Academic / Student CubeSat',
    monthlyBasePriceInr: 0,
    includedFramesM: 10,
    extraFrameRateInr: 0.50,
    includedStorageGb: 100,
    slaTarget: 'Best Effort',
  },
  STARTER: {
    name: 'ISRO Startup / SmallSat Tier',
    monthlyBasePriceInr: 4999,
    includedFramesM: 50,
    extraFrameRateInr: 0.50,
    includedStorageGb: 500,
    slaTarget: '99.9%',
  },
  ENTERPRISE: {
    name: 'Enterprise Fleet (Kuiper Class)',
    monthlyBasePriceInr: 24999,
    includedFramesM: 500,
    extraFrameRateInr: 0.35,
    includedStorageGb: 5000,
    slaTarget: '99.99%',
  },
  MEGACONSTELLATION: {
    name: 'Starlink Megaconstellation',
    monthlyBasePriceInr: 99999,
    includedFramesM: 5000,
    extraFrameRateInr: 0.20,
    includedStorageGb: 100000,
    slaTarget: '99.999%',
  },
};

export function EnterpriseSaasBilling() {
  const [selectedTier, setSelectedTier] = useState<'ACADEMIC' | 'STARTER' | 'ENTERPRISE' | 'MEGACONSTELLATION'>('STARTER');
  const [satCount, setSatCount] = useState<number>(25);
  const [monthlyFramesM, setMonthlyFramesM] = useState<number>(120); // Millions of frames
  const [storageGb, setStorageGb] = useState<number>(850);
  const [currency, setCurrency] = useState<'INR' | 'USD'>('INR');
  const [gstin, setGstin] = useState<string>('27AAAAA0000A1Z5');


  const tier = SAAS_TIERS[selectedTier];
  const overageFramesM = Math.max(0, monthlyFramesM - tier.includedFramesM);
  const overageCostInr = overageFramesM * 1000 * tier.extraFrameRateInr;
  const storageOverageGb = Math.max(0, storageGb - tier.includedStorageGb);
  const storageOverageCostInr = storageOverageGb * 15.0; // ₹15/GB overage
  const subtotalInr = tier.monthlyBasePriceInr + overageCostInr + storageOverageCostInr;
  const gstTaxInr = subtotalInr * 0.18; // 18% GST SAC 998314
  const totalInr = subtotalInr + gstTaxInr;

  const inrToUsdRate = 0.012;
  const formatAmount = (valInr: number) => {
    if (currency === 'USD') {
      return `$${(valInr * inrToUsdRate).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
    }
    return `₹${valInr.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  };

  return (
    <div style={{ padding: '24px', color: '#f8fafc', fontFamily: 'var(--font-sans)', height: '100%', overflowY: 'auto' }} className="glass-panel tech-corner-panel">
      <div style={{ marginBottom: '24px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h1 style={{ fontSize: '20px', fontFamily: 'var(--font-display)', fontWeight: 900, margin: 0, color: 'var(--cyber-cyan)', letterSpacing: '1px' }}>
            💳 SAAS SUBSCRIPTION & METERED BILLING
          </h1>
          <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '4px 0 0 0', fontFamily: 'var(--font-mono)' }}>
            ISRO / IN-SPACe Compliant Multi-Tenant Telemetry Ingestion Consumption & 18% GST Invoicing (SAC 998314).
          </p>
        </div>
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
          <div style={{ backgroundColor: 'rgba(3, 7, 16, 0.8)', border: '1px solid var(--border-color)', borderRadius: '6px', padding: '4px' }}>
            {(['INR', 'USD'] as const).map((curr) => (
              <button
                key={curr}
                onClick={() => setCurrency(curr)}
                style={{
                  padding: '4px 10px',
                  fontSize: '10px',
                  fontFamily: 'var(--font-display)',
                  fontWeight: 700,
                  backgroundColor: currency === curr ? 'var(--cyber-cyan)' : 'transparent',
                  color: currency === curr ? '#020409' : 'var(--text-muted)',
                  border: 'none',
                  borderRadius: '4px',
                  cursor: 'pointer',
                }}
              >
                {curr === 'INR' ? '🇮🇳 INR (₹)' : '🇺🇸 USD ($)'}
              </button>
            ))}
          </div>

          <div style={{ backgroundColor: 'rgba(0, 240, 255, 0.1)', border: '1px solid var(--cyber-cyan)', padding: '8px 16px', borderRadius: '6px' }}>
            <span style={{ fontSize: '10px', color: 'var(--text-muted)', fontFamily: 'var(--font-display)' }}>EST. MONTHLY RECURRING REVENUE (MRR): </span>
            <strong style={{ fontSize: '18px', color: 'var(--cyber-cyan)', fontFamily: 'var(--font-mono)' }}>
              {formatAmount(totalInr)}
            </strong>
          </div>
        </div>
      </div>

      {/* Tier Selector Grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px', marginBottom: '24px' }}>

        {(Object.keys(SAAS_TIERS) as Array<keyof typeof SAAS_TIERS>).map((key) => {
          const t = SAAS_TIERS[key];
          const isSelected = selectedTier === key;
          return (
            <div
              key={key}
              onClick={() => setSelectedTier(key as any)}
              style={{
                backgroundColor: isSelected ? 'rgba(0, 240, 255, 0.12)' : 'rgba(6, 12, 24, 0.7)',
                border: isSelected ? '2px solid var(--cyber-cyan)' : '1px solid var(--border-color)',
                borderRadius: '8px',
                padding: '20px',
                cursor: 'pointer',
                transition: 'all 0.2s ease',
                boxShadow: isSelected ? '0 0 20px rgba(0, 240, 255, 0.2)' : 'none',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '12px' }}>
                <span style={{ fontSize: '12px', fontFamily: 'var(--font-display)', fontWeight: 800, color: isSelected ? 'var(--cyber-cyan)' : '#fff' }}>
                  {t.name}
                </span>
                <span style={{ fontSize: '9px', fontFamily: 'var(--font-mono)', color: 'var(--status-green)', backgroundColor: 'rgba(0, 255, 157, 0.1)', padding: '2px 6px', borderRadius: '3px' }}>
                  SLA: {t.slaTarget}
                </span>
              </div>
              <div style={{ fontSize: '24px', fontWeight: 900, fontFamily: 'var(--font-mono)', color: '#fff', marginBottom: '16px' }}>
                {formatAmount(t.monthlyBasePriceInr)}
                <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>/mo</span>
              </div>
              <ul style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)', paddingLeft: '16px', margin: 0, lineHeight: 1.8 }}>
                <li>Included Telemetry: {t.includedFramesM}M frames/mo</li>
                <li>Overage Rate: ₹{t.extraFrameRateInr}/1K frames</li>
                <li>TimescaleDB Storage: {t.includedStorageGb} GB</li>
                <li>24/7 Ground Control Support</li>
              </ul>
            </div>
          );
        })}
      </div>

      {/* Metered Calculator Controls */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '20px', backgroundColor: 'rgba(6, 12, 24, 0.8)', border: '1px solid var(--border-color)', padding: '20px', borderRadius: '8px', marginBottom: '24px' }}>
        <div>
          <label style={{ fontSize: '10px', fontFamily: 'var(--font-display)', color: 'var(--text-secondary)', fontWeight: 700 }}>ACTIVE SATELLITE COUNT ({satCount})</label>
          <input
            type="range"
            min={1}
            max={500}
            value={satCount}
            onChange={(e) => setSatCount(Number(e.target.value))}
            style={{ width: '100%', marginTop: '10px', accentColor: 'var(--cyber-cyan)' }}
          />
        </div>
        <div>
          <label style={{ fontSize: '10px', fontFamily: 'var(--font-display)', color: 'var(--text-secondary)', fontWeight: 700 }}>MONTHLY TELEMETRY FRAMES ({monthlyFramesM} Million)</label>
          <input
            type="range"
            min={10}
            max={2000}
            value={monthlyFramesM}
            onChange={(e) => setMonthlyFramesM(Number(e.target.value))}
            style={{ width: '100%', marginTop: '10px', accentColor: 'var(--cyber-cyan)' }}
          />
        </div>
        <div>
          <label style={{ fontSize: '10px', fontFamily: 'var(--font-display)', color: 'var(--text-secondary)', fontWeight: 700 }}>TIMESCALEDB STORAGE ({storageGb} GB)</label>
          <input
            type="range"
            min={100}
            max={10000}
            value={storageGb}
            onChange={(e) => setStorageGb(Number(e.target.value))}
            style={{ width: '100%', marginTop: '10px', accentColor: 'var(--cyber-cyan)' }}
          />
        </div>
      </div>

      {/* Tax & Invoicing Summary Card */}
      <div style={{ backgroundColor: 'rgba(3, 7, 16, 0.9)', border: '1px solid var(--border-color)', padding: '20px', borderRadius: '8px' }}>
        <div style={{ fontSize: '12px', fontFamily: 'var(--font-display)', fontWeight: 800, color: 'var(--cyber-cyan)', marginBottom: '12px' }}>
          📄 GST INVOICE BREAKDOWN (SAC CODE: 998314 - SAAS TELEMETRY OBSERVABILITY)
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px', fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
          <div>
            <span style={{ color: 'var(--text-muted)' }}>Base Subscription Fee:</span>
            <div style={{ color: '#fff', fontWeight: 'bold', marginTop: '4px' }}>{formatAmount(tier.monthlyBasePriceInr)}</div>
          </div>
          <div>
            <span style={{ color: 'var(--text-muted)' }}>Volume Overage Charges:</span>
            <div style={{ color: '#fff', fontWeight: 'bold', marginTop: '4px' }}>{formatAmount(overageCostInr + storageOverageCostInr)}</div>
          </div>
          <div>
            <span style={{ color: 'var(--text-muted)' }}>GST (18% SAC 998314):</span>
            <div style={{ color: 'var(--status-yellow)', fontWeight: 'bold', marginTop: '4px' }}>{formatAmount(gstTaxInr)}</div>
          </div>
          <div>
            <span style={{ color: 'var(--text-muted)' }}>Total Amount Payable:</span>
            <div style={{ color: 'var(--status-green)', fontWeight: 'bold', marginTop: '4px', fontSize: '14px' }}>{formatAmount(totalInr)}</div>
          </div>
        </div>
      </div>
    </div>
  );
}
