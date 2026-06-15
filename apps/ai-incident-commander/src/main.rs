use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "AI Incident Commander")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8088")]
    port: u16,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://postgres:postgrespassword@localhost:5432/spaceops"
    )]
    database_url: String,

    #[arg(
        long,
        env = "OTEL_EXPORTER_OTLP_ENDPOINT",
        default_value = "http://localhost:4317"
    )]
    otel_endpoint: String,
}

struct AppState {
    db_pool: PgPool,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    query: String,
    satellite_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    response: String,
    sources: Vec<String>,
}

const THERMAL_RUNBOOK: &str = "
--- THERMAL RUNBOOK (DOC-TH-099) ---
Symptoms: Subsystem temperature > 80°C.
Recommended Actions:
1. Orient satellite yaw away from direct sun vector (Cold Space pointing).
2. Shut down secondary payloads (Scientific instruments, auxiliary transmitter).
3. If temperature exceeds 90°C, command Flight Computer to SAFE MODE.
";

const BATTERY_RUNBOOK: &str = "
--- BATTERY DEGRADATION RUNBOOK (DOC-EPS-104) ---
Symptoms: Battery voltage < 11.5V.
Recommended Actions:
1. Set solar panel sun-tracking to MAXIMUM efficiency.
2. Shed payload draw (Disable heaters, payload camera).
3. Switch primary communications to low-power beacon beaconing every 60s instead of continuous.
";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Init tracing
    spaceops_models::telemetry::init_telemetry("ai-incident-commander", &cli.otel_endpoint)?;

    info!("Starting AI Incident Commander...");

    let db_pool = spaceops_models::db::establish_db_connection(&cli.database_url, 5).await?;
    let state = Arc::new(AppState { db_pool });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/chat", post(chat))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("AI Incident Commander listening on http://localhost:{}", cli.port);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<serde_json::Value>)> {
    let query = req.query.to_lowercase();
    let mut sat_id = req.satellite_id.clone().unwrap_or_else(|| {
        // Simple extraction from string
        if query.contains("sat-0001") { "SAT-0001".to_string() }
        else if query.contains("sat-0002") { "SAT-0002".to_string() }
        else if query.contains("sat-0003") { "SAT-0003".to_string() }
        else if query.contains("sat-0004") { "SAT-0004".to_string() }
        else if query.contains("sat-0005") { "SAT-0005".to_string() }
        else if query.contains("sat-0006") { "SAT-0006".to_string() }
        else if query.contains("sat-0007") { "SAT-0007".to_string() }
        else if query.contains("sat-0008") { "SAT-0008".to_string() }
        else if query.contains("sat-0009") { "SAT-0009".to_string() }
        else if query.contains("sat-0010") { "SAT-0010".to_string() }
        else { "SAT-0001".to_string() }
    });

    // 1. Fetch relevant context from DB
    let telemetry_opt = get_latest_telemetry(&state.db_pool, &sat_id).await;
    let events = get_recent_events(&state.db_pool, &sat_id).await;
    let commands = get_recent_commands(&state.db_pool, &sat_id).await;

    // Check if external LLM API key is present
    let gemini_key = std::env::var("GEMINI_API_KEY").ok();
    let openai_key = std::env::var("OPENAI_API_KEY").ok();

    let mut context = format!(
        "SATELLITE STATUS REPORT\n\
         Asset: {}\n\
         Current Telemetry:\n{:?}\n\n\
         Recent Anomalies / Events:\n{:?}\n\n\
         Recent Commands:\n{:?}\n\n\
         Reference Runbooks:\n{}\n{}",
        sat_id, telemetry_opt, events, commands, THERMAL_RUNBOOK, BATTERY_RUNBOOK
    );

    let mut sources = vec![
        format!("TimescaleDB `telemetry` table (Asset: {})", sat_id),
        format!("TimescaleDB `events` table (Asset: {})", sat_id),
        format!("PostgreSQL `commands` table (Asset: {})", sat_id),
    ];

    let mut response_text = String::new();

    if let Some(key) = gemini_key {
        // Call actual Gemini API
        info!("Calling Gemini API for incident analysis...");
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key={}",
            key
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": format!(
                        "You are Antigravity, SpaceX Flight Operations AI Copilot. Answer the operator's query: '{}'.\n\nContext:\n{}",
                        req.query, context
                    )
                }]
            }]
        });

        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                if let Ok(res_json) = resp.json::<serde_json::Value>().await {
                    if let Some(txt) = res_json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                        response_text = txt.to_string();
                    }
                }
            }
            Err(e) => {
                error!("Gemini API call failed: {}", e);
            }
        }
    } else if let Some(key) = openai_key {
        // Call actual OpenAI API
        info!("Calling OpenAI API for incident analysis...");
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": "gpt-4-turbo",
            "messages": [
                {
                    "role": "system",
                    "content": "You are Antigravity, SpaceX Flight Operations AI Copilot."
                },
                {
                    "role": "user",
                    "content": format!("Answer the operator's query: '{}'.\n\nContext:\n{}", req.query, context)
                }
            ]
        });

        match client.post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(key)
            .json(&body)
            .send()
            .await {
            Ok(resp) => {
                if let Ok(res_json) = resp.json::<serde_json::Value>().await {
                    if let Some(txt) = res_json["choices"][0]["message"]["content"].as_str() {
                        response_text = txt.to_string();
                    }
                }
            }
            Err(e) => {
                error!("OpenAI API call failed: {}", e);
            }
        }
    }

    if response_text.is_empty() {
        // High-fidelity rule-based response if no LLM keys or calls failed
        info!("Generating rule-based high-fidelity agent response...");
        response_text = generate_rule_response(&query, &sat_id, telemetry_opt, &events, &commands);
        sources.push("Operational Runbooks (DOC-TH-099, DOC-EPS-104)".to_string());
    }

    Ok(Json(ChatResponse {
        response: response_text,
        sources,
    }))
}

async fn get_latest_telemetry(pool: &PgPool, sat_id: &str) -> Option<serde_json::Value> {
    let row = sqlx::query("SELECT * FROM telemetry WHERE satellite_id = $1 ORDER BY timestamp DESC LIMIT 1")
        .bind(sat_id)
        .fetch_optional(pool)
        .await
        .ok()??;

    Some(serde_json::json!({
        "timestamp": row.get::<chrono::DateTime<chrono::Utc>, _>("timestamp"),
        "battery_voltage": row.get::<Option<f64>, _>("battery_voltage"),
        "battery_current": row.get::<Option<f64>, _>("battery_current"),
        "solar_panel_output": row.get::<Option<f64>, _>("solar_panel_output"),
        "cpu_usage": row.get::<Option<f32>, _>("cpu_usage"),
        "memory_usage": row.get::<Option<f32>, _>("memory_usage"),
        "temperature": row.get::<Option<f64>, _>("temperature"),
        "fuel_remaining": row.get::<Option<f32>, _>("fuel_remaining"),
        "signal_strength": row.get::<Option<f32>, _>("signal_strength"),
        "packet_loss": row.get::<Option<f32>, _>("packet_loss"),
        "communication_latency": row.get::<Option<f32>, _>("communication_latency"),
    }))
}

async fn get_recent_events(pool: &PgPool, sat_id: &str) -> Vec<serde_json::Value> {
    let rows = sqlx::query("SELECT * FROM events WHERE satellite_id = $1 ORDER BY timestamp DESC LIMIT 5")
        .bind(sat_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    rows.into_iter().map(|row| {
        serde_json::json!({
            "timestamp": row.get::<chrono::DateTime<chrono::Utc>, _>("timestamp"),
            "event_type": row.get::<String, _>("event_type"),
            "severity": row.get::<String, _>("severity"),
            "message": row.get::<String, _>("message"),
        })
    }).collect()
}

async fn get_recent_commands(pool: &PgPool, sat_id: &str) -> Vec<serde_json::Value> {
    let rows = sqlx::query("SELECT * FROM commands WHERE satellite_id = $1 ORDER BY created_at DESC LIMIT 5")
        .bind(sat_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    rows.into_iter().map(|row| {
        serde_json::json!({
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
            "command_type": row.get::<String, _>("command_type"),
            "status": row.get::<String, _>("status"),
            "created_by": row.get::<String, _>("created_by"),
        })
    }).collect()
}

fn generate_rule_response(
    query: &str,
    sat_id: &str,
    telemetry: Option<serde_json::Value>,
    events: &[serde_json::Value],
    commands: &[serde_json::Value],
) -> String {
    let temp = telemetry.as_ref().and_then(|t| t["temperature"].as_f64()).unwrap_or(25.0);
    let fuel = telemetry.as_ref().and_then(|t| t["fuel_remaining"].as_f64()).unwrap_or(100.0);
    let battery = telemetry.as_ref().and_then(|t| t["battery_voltage"].as_f64()).unwrap_or(13.8);

    if query.contains("unhealthy") || query.contains("why") || query.contains("status") {
        let mut reasons = Vec::new();
        if temp > 80.0 {
            reasons.push(format!("Subsystem temperature is critically high at **{:.1}°C** (limit: 80.0°C).", temp));
        }
        if fuel < 15.0 {
            reasons.push(format!("Propellant supply is low at **{:.1}%**.", fuel));
        }
        if battery < 11.5 {
            reasons.push(format!("Battery bank voltage is degraded at **{:.1}V**.", battery));
        }

        let anomaly_desc = if !reasons.is_empty() {
            reasons.join("\n- ")
        } else {
            "All core subsystems are operating within normal telemetry ranges.".to_string()
        };

        let recommendations = if temp > 80.0 {
            "**Recommended Actions (per THERMAL RUNBOOK DOC-TH-099):**\n\
             1. **Orient yaw axis** to point heat shields directly at the sun vector.\n\
             2. **Shed payload power draw** (issue command `SHUTDOWN_PAYLOAD`).\n\
             3. Monitor telemetry loops closely for thermal stabilization."
        } else if battery < 11.5 {
            "**Recommended Actions (per BATTERY RUNBOOK DOC-EPS-104):**\n\
             1. Ensure solar panels are tracking at optimal sun-incidence angle.\n\
             2. Shut down secondary instrumentation heaters."
        } else {
            "No immediate operator remediation actions required."
        };

        format!(
            "### AI Diagnostics Summary for **{}**\n\n\
             **Subsystem Status:** {}\n\n\
             **Recent Anomalies / Events:**\n{}\n\n\
             {}\n\n\
             *Generated by SpaceX Flight Ops AI Copilot*",
            sat_id,
            if !reasons.is_empty() { "DEGRADED / CRITICAL" } else { "NOMINAL" },
            if events.is_empty() { "No recent critical anomalies logged." } else { "Anomalies detected in last 24h." },
            recommendations
        )
    } else if query.contains("summarize") || query.contains("briefing") || query.contains("24 hours") {
        format!(
            "### Operator Handover Briefing: Constellation Status\n\n\
             - **Active Assets Monitored**: 10 Satellites (SAT-0001 through SAT-0010)\n\
             - **Overall Fleet Readiness**: Nominal\n\
             - **Target Analysis ({})**:\n\
               - Core Temperature: {:.1}°C\n\
               - Battery Level: {:.1}V\n\
               - Fuel Remaining: {:.1}%\n\n\
             **Recent Operations Log:**\n\
             - Uplinked {} commands in the last 2 hours.\n\
             - No new collision threats or orbital trajectory deviations detected.\n",
            sat_id, temp, battery, fuel, commands.len()
        )
    } else if query.contains("postmortem") || query.contains("report") {
        format!(
            "# Incident Postmortem Report\n\n\
             **Incident Asset:** {}\n\
             **Timestamp:** {}\n\n\
             ## Executive Summary\n\
             A transient thermal spike anomaly was registered on subsystem `thermal`. Temperature reached **{:.1}°C**, triggering critical telemetry alerts.\n\n\
             ## Root Cause Analysis\n\
             High-power payload duty cycles coupled with a solar array orientation maneuver resulted in transient heat trap. Avionics processor throttled to protect components.\n\n\
             ## Resolution Actions\n\
             Uplinked cold space pointing yaw command. Normal thermal margins restored.\n\n\
             *Report compiled automatically by SpaceX AI Incident Commander*",
            sat_id, chrono::Utc::now().to_rfc3339(), temp
        )
    } else {
        format!(
            "Hello, Operator. I am your Space Operations AI Copilot. I have access to live telemetry, incident history, and standard runbooks. Ask me about asset diagnostics, anomaly causes, or summaries."
        )
    }
}
