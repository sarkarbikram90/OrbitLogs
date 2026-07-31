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
export interface Satellite {
  id: string;
  state: 'GREEN' | 'YELLOW' | 'RED';
  score: number;
}

export interface GroundStation {
  timestamp: string;
  station_id: string;
  station_name: string;
  status: string; // 'ONLINE', 'DEGRADED', 'OUTAGE'
  antenna_health: number;
  weather_state: string;
  congestion: number;
  latency: number;
  bandwidth: number;
  explanation: string;
}

export interface EventLog {
  timestamp: string;
  satellite_id: string;
  event_type: string;
  severity: 'INFO' | 'WARNING' | 'CRITICAL';
  message: string;
}

export interface ReplaySession {
  session_id: string;
  satellite_id: string | null;
  start_time: string;
  end_time: string;
  current_position: string;
  speed: number;
  status: 'PLAYING' | 'PAUSED' | 'STOPPED';
}

export interface SubsystemHealth {
  state: string;
  score: number;
  explanation: string;
}

export interface SatelliteHealth {
  timestamp: string;
  satellite_id: string;
  mission_id: string;
  overall_state: string;
  overall_score: number;
  power: SubsystemHealth;
  thermal: SubsystemHealth;
  communications: SubsystemHealth;
  propulsion: SubsystemHealth;
  adcs: SubsystemHealth;
  payload: SubsystemHealth;
  flight_computer: SubsystemHealth;
  explanation: string;
}

export interface TelemetryRecord {
  satellite_id: string;
  subsystem_id: string;
  mission_id: string;
  timestamp: string;
  battery_voltage?: number;
  battery_current?: number;
  solar_panel_output?: number;
  cpu_usage?: number;
  memory_usage?: number;
  temperature?: number;
  orientation_pitch?: number;
  orientation_roll?: number;
  orientation_yaw?: number;
  velocity?: number;
  altitude?: number;
  fuel_remaining?: number;
  communication_latency?: number;
  packet_loss?: number;
  signal_strength?: number;
  payload_power_draw?: number;
  payload_active?: boolean;
}

export interface Command {
  id: string;
  satellite_id: string;
  command_type: string;
  status: string;
  parameters: Record<string, any>;
  created_by: string;
  approved_by?: string;
  scheduled_time?: string;
  executed_at?: string;
  created_at: string;
}

export interface TimelineEvent {
  timestamp: string;
  event_id: string;
  source: string;
  satellite_id?: string;
  event_type: string;
  severity: 'INFO' | 'WARNING' | 'CRITICAL';
  message: string;
  details?: Record<string, any>;
}

export interface Satellite {
  id: string;
  state: 'GREEN' | 'YELLOW' | 'RED';
  score: number;
}

export interface GroundStation {
  timestamp: string;
  station_id: string;
  station_name: string;
  status: string; // 'ONLINE', 'DEGRADED', 'OUTAGE'
  antenna_health: number;
  weather_state: string;
  congestion: number;
  latency: number;
  bandwidth: number;
  explanation: string;
}

export interface EventLog {
  timestamp: string;
  satellite_id: string;
  event_type: string;
  severity: 'INFO' | 'WARNING' | 'CRITICAL';
  message: string;
}

export interface ReplaySession {
  session_id: string;
  satellite_id: string | null;
  start_time: string;
  end_time: string;
  current_position: string;
  speed: number;
  status: 'PLAYING' | 'PAUSED' | 'STOPPED';
}

export interface SubsystemHealth {
  state: string;
  score: number;
  explanation: string;
}

export interface SatelliteHealth {
  timestamp: string;
  satellite_id: string;
  mission_id: string;
  overall_state: string;
  overall_score: number;
  power: SubsystemHealth;
  thermal: SubsystemHealth;
  communications: SubsystemHealth;
  propulsion: SubsystemHealth;
  adcs: SubsystemHealth;
  payload: SubsystemHealth;
  flight_computer: SubsystemHealth;
  explanation: string;
}

export interface TelemetryRecord {
  satellite_id: string;
  subsystem_id: string;
  mission_id: string;
  timestamp: string;
  battery_voltage?: number;
  battery_current?: number;
  solar_panel_output?: number;
  cpu_usage?: number;
  memory_usage?: number;
  temperature?: number;
  orientation_pitch?: number;
  orientation_roll?: number;
  orientation_yaw?: number;
  velocity?: number;
  altitude?: number;
  fuel_remaining?: number;
  communication_latency?: number;
  packet_loss?: number;
  signal_strength?: number;
  payload_power_draw?: number;
  payload_active?: boolean;
}

export interface Command {
  id: string;
  satellite_id: string;
  command_type: string;
  status: string;
  parameters: Record<string, any>;
  created_by: string;
  approved_by?: string;
  scheduled_time?: string;
  executed_at?: string;
  created_at: string;
}

export interface TimelineEvent {
  timestamp: string;
  event_id: string;
  source: string;
  satellite_id?: string;
  event_type: string;
  severity: 'INFO' | 'WARNING' | 'CRITICAL';
  message: string;
  details?: Record<string, any>;
}

export interface SubsystemStateProjection {
  status: string;
  score: number;
  details: string;
}

export interface TwinHealthProjection {
  satellite_id: string;
  timestamp: string;
  subsystems: {
    power: SubsystemStateProjection;
    thermal: SubsystemStateProjection;
    adcs: SubsystemStateProjection;
    communications: SubsystemStateProjection;
    payload: SubsystemStateProjection;
    propulsion: SubsystemStateProjection;
  };
  life_expectancy_days: number;
  recommendations: string[];
}

export interface ConjunctionEvent {
  primary_id: string;
  secondary_id: string;
  secondary_type: string;
  tca: string;
  miss_distance_m: number;
  relative_velocity_m_s: number;
  collision_probability: number;
  combined_hard_body_radius_m: number;
}

export interface CamProposal {
  delta_v_ric_m_s: [number, number, number];
  delta_v_magnitude_m_s: number;
  burn_time: string;
  propellant_kg: number;
  post_burn_miss_distance_m: number;
  post_burn_pc: number;
  post_burn_fuel_pct: number;
}

export interface SpaceWeatherData {
  kp_index: number;
  solar_proton_flux: number;
  dst_index_nt: number;
  seu_risk: 'LOW' | 'ELEVATED' | 'HIGH' | 'CRITICAL';
  atmospheric_density_kg_m3: number;
}
