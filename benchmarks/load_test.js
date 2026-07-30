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
import http from 'k6/http';
import { check } from 'k6';
import { uuidv4 } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

// Get target rate from environment or default to 1000 events/sec
const TARGET_RATE = __ENV.TARGET_RATE ? parseInt(__ENV.TARGET_RATE) : 1000;
const DURATION = __ENV.DURATION ? __ENV.DURATION : '30s';
const API_KEY = __ENV.API_KEY || 'default-dev-key-12345';

export const options = {
  scenarios: {
    telemetry_ingestion: {
      executor: 'constant-arrival-rate',
      rate: TARGET_RATE,
      timeUnit: '1s',
      duration: DURATION,
      preAllocatedVUs: Math.max(50, Math.floor(TARGET_RATE / 20)),
      maxVUs: Math.max(200, Math.floor(TARGET_RATE / 5)),
    },
  },
  thresholds: {
    http_req_failed: ['rate<0.01'], // less than 1% errors
    http_req_duration: ['p(95)<100'], // 95% of requests must complete below 100ms
  },
};

export default function () {
  const satId = `SAT-${String(Math.floor(Math.random() * 50) + 1).padStart(4, '0')}`;
  
  const payload = JSON.stringify({
    satellite_id: satId,
    timestamp: new Date().toISOString(),
    battery_voltage: 13.5 + Math.random() * 1.0,
    battery_current: 2.0 - Math.random() * 4.0,
    solar_panel_output: 100.0 + Math.random() * 40.0,
    cpu_usage: 20.0 + Math.random() * 30.0,
    memory_usage: 40.0 + Math.random() * 10.0,
    temperature: 25.0 + Math.random() * 15.0,
    orientation_pitch: Math.random() * 10.0 - 5.0,
    orientation_roll: Math.random() * 10.0 - 5.0,
    orientation_yaw: Math.random() * 10.0 - 5.0,
    velocity: 7.67,
    altitude: 520.0 + Math.random() * 10.0,
    fuel_remaining: 85.0 - Math.random() * 0.05,
    communication_latency: 30.0 + Math.random() * 20.0,
    packet_loss: Math.random() * 0.5,
    signal_strength: -70.0 + Math.random() * 10.0,
    event_id: uuidv4(),
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
    },
  };

  const res = http.post('http://localhost:8080/telemetry', payload, params);

  check(res, {
    'status is 202': (r) => r.status === 202,
    'has event_id': (r) => {
      const body = JSON.parse(r.body);
      return body && body.event_id !== undefined;
    },
  });
}
