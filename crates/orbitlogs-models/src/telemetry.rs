use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub fn init_telemetry(service_name: &str, otlp_endpoint: &str) -> Result<(), anyhow::Error> {
    // Register the global trace context propagator for context
    // propagation across services (e.g. over Kafka)
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Configure standard resources (service name merged with SDK defaults)
    let resource = Resource::default().merge(&Resource::new(vec![KeyValue::new(
        "service.name",
        service_name.to_string(),
    )]));

    // Configure OpenTelemetry Tracing pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .with_trace_config(sdktrace::config().with_resource(resource))
        .install_batch(runtime::Tokio)?;

    // Integrate with Tracing Registry
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .with(telemetry_layer)
        .init();

    info!(
        "Tracing telemetry initialized for the service '{}'",
        service_name
    );
    Ok(())
}

pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}
