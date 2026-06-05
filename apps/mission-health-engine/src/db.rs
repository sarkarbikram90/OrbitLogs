use crate::health::SatelliteHealthAssessment;
use sqlx::{Error, PgPool};

pub async fn insert_satellite_health(
    pool: &PgPool,
    assessment: &SatelliteHealthAssessment,
) -> Result<(), Error> {
    sqlx::query(
        r#"
        INSERT INTO satellite_health (
            timestamp, satellite_id, mission_id, overall_state, overall_score,
            power_state, thermal_state, comm_state, propulsion_state, adcs_state,
            payload_state, fc_state, explanation
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
        )
        "#,
    )
    .bind(assessment.timestamp)
    .bind(&assessment.satellite_id)
    .bind(&assessment.mission_id)
    .bind(assessment.overall_state.to_string())
    .bind(assessment.overall_score)
    .bind(assessment.power.state.to_string())
    .bind(assessment.thermal.state.to_string())
    .bind(assessment.communications.state.to_string())
    .bind(assessment.propulsion.state.to_string())
    .bind(assessment.adcs.state.to_string())
    .bind(assessment.payload.state.to_string())
    .bind(assessment.flight_computer.state.to_string())
    .bind(&assessment.explanation)
    .execute(pool)
    .await?;

    Ok(())
}
