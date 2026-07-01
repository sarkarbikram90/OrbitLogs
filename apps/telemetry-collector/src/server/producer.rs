use rdkafka::message::{Header, OwnedHeaders};
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

struct CircuitBreakerState {
    state: CircuitState,
    consecutive_failures: usize,
    last_state_change: Instant,
}

#[derive(Clone)]
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    failure_threshold: usize,
    recovery_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, recovery_timeout: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState {
                state: CircuitState::Closed,
                consecutive_failures: 0,
                last_state_change: Instant::now(),
            })),
            failure_threshold,
            recovery_timeout,
        }
    }

    pub fn allow_request(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        match state.state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open => {
                if state.last_state_change.elapsed() >= self.recovery_timeout {
                    state.state = CircuitState::HalfOpen;
                    state.last_state_change = Instant::now();
                    info!("Kafka Producer Circuit Breaker transitioned to HALF-OPEN (testing Kafka connectivity)");
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        if state.state == CircuitState::HalfOpen {
            state.state = CircuitState::Closed;
            info!("Kafka Producer Circuit Breaker transitioned to CLOSED (Kafka is healthy)");
        }
        state.consecutive_failures = 0;
    }

    pub fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();
        state.consecutive_failures += 1;
        if state.state == CircuitState::HalfOpen
            || state.consecutive_failures >= self.failure_threshold
        {
            state.state = CircuitState::Open;
            state.last_state_change = Instant::now();
            error!(
                "Kafka Producer Circuit Breaker transitioned to OPEN ({} consecutive failures, tripping breaker)",
                state.consecutive_failures
            );
        }
    }
}

#[derive(Clone)]
pub struct KafkaProducer {
    producer: FutureProducer,
    cb: CircuitBreaker,
}

impl KafkaProducer {
    pub fn new(
        brokers: &str,
        security_protocol: Option<&str>,
        sasl_mechanism: Option<&str>,
        sasl_username: Option<&str>,
        sasl_password: Option<&str>,
        ssl_ca_location: Option<&str>,
    ) -> Result<Self, anyhow::Error> {
        let mut config = orbitlogs_models::kafka::KafkaConfigBuilder::new(brokers.to_string())
            .with_security(
                security_protocol.map(|s| s.to_string()),
                sasl_mechanism.map(|s| s.to_string()),
                sasl_username.map(|s| s.to_string()),
                sasl_password.map(|s| s.to_string()),
                ssl_ca_location.map(|s| s.to_string()),
            )
            .with_ssl(
                orbitlogs_models::load_secret("KAFKA_SSL_CERTIFICATE_LOCATION"),
                orbitlogs_models::load_secret("KAFKA_SSL_KEY_LOCATION"),
                orbitlogs_models::load_secret("KAFKA_SSL_KEY_PASSWORD"),
            )
            .build();
        config
            .set("message.timeout.ms", "5000")
            .set("queue.buffering.max.messages", "100000");

        let producer: FutureProducer = config.create()?;
        let cb = CircuitBreaker::new(5, Duration::from_secs(10));

        Ok(Self { producer, cb })
    }

    pub async fn publish(
        &self,
        topic: &str,
        key: &str,
        payload: &str,
    ) -> Result<(), anyhow::Error> {
        if !self.cb.allow_request() {
            return Err(anyhow::anyhow!(
                "Circuit breaker is OPEN. Kafka is unreachable."
            ));
        }

        let mut headers = OwnedHeaders::new();
        let context = tracing::Span::current().context();
        let mut carrier = std::collections::HashMap::new();
        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&context, &mut carrier);
        });
        for (k, v) in &carrier {
            headers = headers.insert(Header {
                key: k,
                value: Some(v),
            });
        }

        let record = FutureRecord::to(topic)
            .key(key)
            .payload(payload)
            .headers(headers);

        match self.producer.send(record, Duration::from_secs(5)).await {
            Ok((partition, offset)) => {
                debug!(
                    "Successfully published event to topic {} [partition: {}, offset: {}]",
                    topic, partition, offset
                );
                self.cb.record_success();
                Ok(())
            }
            Err((e, _)) => {
                error!("Failed to publish event to topic {}: {}", topic, e);
                self.cb.record_failure();
                Err(anyhow::anyhow!("Kafka send error: {}", e))
            }
        }
    }
}
