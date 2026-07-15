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
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fleet_health::{FleetHealthEngine, SatelliteHealthSummary};
use orbitlogs_models::HealthState;

fn generate_mock_fleet(size: usize) -> Vec<SatelliteHealthSummary> {
    (0..size)
        .map(|i| SatelliteHealthSummary {
            id: format!("SAT-{:04}", i),
            overall_state: if i % 10 == 0 {
                HealthState::Red
            } else if i % 5 == 0 {
                HealthState::Yellow
            } else {
                HealthState::Green
            },
            overall_score: (80 + (i % 21)) as i32,
            fuel_remaining: 85.0,
            cpu_usage: 25.0,
            memory_usage: 40.0,
            payload_active: true,
        })
        .collect()
}

fn bench_fleet_health_calculate(c: &mut Criterion) {
    let mut group = c.benchmark_group("fleet_health_calculate");

    for size in &[10, 100, 1000] {
        let fleet = generate_mock_fleet(*size);
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &fleet,
            |b, fleet| {
                b.iter(|| FleetHealthEngine::calculate(black_box(fleet)));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_fleet_health_calculate);
criterion_main!(benches);
