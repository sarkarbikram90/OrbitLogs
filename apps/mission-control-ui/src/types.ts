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
