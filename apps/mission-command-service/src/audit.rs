use crate::AppState;
use opentelemetry::trace::TraceContextExt;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;

pub async fn log_audit(
    state: &AppState,
    command_id: Uuid,
    from: Option<&str>,
    to: &str,
    by: &str,
    notes: &str,
) -> Result<(), crate::error::AppError> {
    let trace_id = Span::current()
        .context()
        .span()
        .span_context()
        .trace_id()
        .to_string();

    sqlx::query(
        "INSERT INTO command_audit_trail (command_id, transition_from, transition_to, changed_by, trace_id, notes)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(command_id)
    .bind(from)
    .bind(to)
    .bind(by)
    .bind(trace_id)
    .bind(notes)
    .execute(&state.db_pool)
    .await?;

    Ok(())
}
