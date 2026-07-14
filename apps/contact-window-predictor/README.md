# Contact Window Predictor Service 📡🔮

The **Contact Window Predictor** is a background daemon that periodically calculates upcoming communication visibility windows between the satellite constellation and Earth-fixed ground stations.

## How It Works

1. **Active Fleet Enumeration**: Iterates over all active assets in the satellite constellation (`SAT-0001` through `SAT-0010`).
2. **TLE Retrieval**: Queries the database (`tle_records`) for the latest ingested Two-Line Element set for each spacecraft.
3. **Geometry Prediction**: Uses standard SGP4 propagation via the `orbitlogs-astrodynamics` library to calculate the spacecraft's orbital track over the next 24 hours.
4. **Elevation Angle Check**: Evaluates if the satellite's elevation angle exceeds the target station's minimum elevation mask (typically $10^\circ$).
5. **Persistence**: Saves verified visibility ranges into the `contact_schedules` hypertable, which is subsequently read by the constellation scheduler and ground station dashboards.

## Configuration

The service is configured via command-line arguments or matching environment variables:

| Parameter | CLI Flag | Env Var | Default | Description |
|:---|:---|:---|:---|:---|
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres connection string |
| **Prediction Interval** | `--interval-secs` | `PREDICTION_INTERVAL_SECS` | `60` | Loop sleep interval between runs |

## Running Locally

To compile and launch the predictor:

```bash
cargo run -p contact-window-predictor
```
