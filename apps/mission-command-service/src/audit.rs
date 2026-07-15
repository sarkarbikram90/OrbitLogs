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
