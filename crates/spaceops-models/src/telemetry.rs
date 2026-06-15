use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub fn init_telemetry(service_name: &str, otlp_endpoint: &str) -> Result<(), anyhow::Error> {
    // Configure OpenTelemetry Tracing pipeline
    let tracer =
        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(otlp_endpoint),
            )
            .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", service_name.to_string()),
            ])))
            .install_batch(runtime::Tokio)?;

    // Integrate with Tracing Registry
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)
        .init();

    info!(
        "Tracing telemetry initialized for service '{}'",
        service_name
    );
    Ok(())
}

pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}
