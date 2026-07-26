// Copyright 2026 OrbitLogs Authors

export interface RegisteredSat {
  id: string;
  satellite_id: string;
  satellite_name: string;
  norad_id?: number;
  orbit_type: string;
  tle_line1?: string;
  tle_line2?: string;
  status: string;
}

export interface RegisteredStation {
  id: string;
  station_id: string;
  station_name: string;
  latitude: number;
  longitude: number;
  altitude_m: number;
  antenna_type?: string;
  frequency_band?: string;
  status: string;
}
