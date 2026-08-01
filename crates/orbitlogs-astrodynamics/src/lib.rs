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
pub mod conjunction;
pub mod coordinates;
pub mod navic;
pub mod space_weather;
pub mod tle;

pub use conjunction::{
    compute_cam_delta_v, compute_collision_probability, compute_covariance_ellipsoid, CamProposal,
    ConjunctionEvent, CovarianceEllipsoid,
};
pub use coordinates::{
    calculate_gmst_rad, calculate_look_angles, datetime_to_jd, ecef_to_enu, ecef_to_geodetic,
    eci_to_ecef, eci_velocity_to_ecef, geodetic_to_ecef, EnuCoords, GeodeticCoords, LookAngles,
};
pub use navic::{
    calculate_navic_iono_free_pseudorange, compute_navic_position_fix, irnt_to_utc,
    NavicIonosphericModel, NavicNavigationSolution, NavicObservation, NavicSatellite,
    NAVIC_L5_FREQ_MHZ, NAVIC_S_FREQ_MHZ,
};
pub use space_weather::{
    assess_seu_risk, calculate_atmospheric_drag_multiplier, calculate_orbit_decay_rate_m_day,
    generate_space_weather_observation, GeomagneticStormLevel, RadiationStormLevel, SeuRiskLevel,
    SpaceWeatherObservation,
};
pub use tle::{predict_contact_windows, AstrodynamicsError, ContactWindow, TleRecord};
