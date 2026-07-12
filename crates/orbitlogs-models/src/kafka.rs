use rdkafka::config::ClientConfig;

#[derive(Debug, Clone, Default)]
pub struct KafkaConfigBuilder {
    pub brokers: String,
    pub security_protocol: Option<String>,
    pub sasl_mechanism: Option<String>,
    pub sasl_username: Option<String>,
    pub sasl_password: Option<String>,
    pub ssl_ca_location: Option<String>,
    pub ssl_certificate_location: Option<String>,
    pub ssl_key_location: Option<String>,
    pub ssl_key_password: Option<String>,
}

impl KafkaConfigBuilder {
    pub fn gen_config(brokers: String) -> Self {
        Self {
            brokers,
            ..Default::default()
        }
    }

    pub fn new(brokers: String) -> Self {
        Self {
            brokers,
            ..Default::default()
        }
    }

    pub fn from_env(brokers: String) -> Self {
        use crate::load_secret;
        Self {
            brokers,
            security_protocol: load_secret("KAFKA_SECURITY_PROTOCOL"),
            sasl_mechanism: load_secret("KAFKA_SASL_MECHANISM"),
            sasl_username: load_secret("KAFKA_SASL_USERNAME"),
            sasl_password: load_secret("KAFKA_SASL_PASSWORD"),
            ssl_ca_location: load_secret("KAFKA_SSL_CA_LOCATION"),
            ssl_certificate_location: load_secret("KAFKA_SSL_CERTIFICATE_LOCATION"),
            ssl_key_location: load_secret("KAFKA_SSL_KEY_LOCATION"),
            ssl_key_password: load_secret("KAFKA_SSL_KEY_PASSWORD"),
        }
    }

    pub fn with_security(
        mut self,
        security_protocol: Option<String>,
        sasl_mechanism: Option<String>,
        sasl_username: Option<String>,
        sasl_password: Option<String>,
        ssl_ca_location: Option<String>,
    ) -> Self {
        self.security_protocol = security_protocol;
        self.sasl_mechanism = sasl_mechanism;
        self.sasl_username = sasl_username;
        self.sasl_password = sasl_password;
        self.ssl_ca_location = ssl_ca_location;
        self
    }

    pub fn with_ssl(
        mut self,
        ssl_certificate_location: Option<String>,
        ssl_key_location: Option<String>,
        ssl_key_password: Option<String>,
    ) -> Self {
        self.ssl_certificate_location = ssl_certificate_location;
        self.ssl_key_location = ssl_key_location;
        self.ssl_key_password = ssl_key_password;
        self
    }

    pub fn build(&self) -> ClientConfig {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", &self.brokers);

        if let Some(ref proto) = self.security_protocol {
            config.set("security.protocol", proto);
        }
        if let Some(ref mech) = self.sasl_mechanism {
            config.set("sasl.mechanism", mech);
        }
        if let Some(ref user) = self.sasl_username {
            config.set("sasl.username", user);
        }
        if let Some(ref pass) = self.sasl_password {
            config.set("sasl.password", pass);
        }
        if let Some(ref ca) = self.ssl_ca_location {
            config.set("ssl.ca.location", ca);
        }
        if let Some(ref cert) = self.ssl_certificate_location {
            config.set("ssl.certificate.location", cert);
        }
        if let Some(ref key) = self.ssl_key_location {
            config.set("ssl.key.location", key);
        }
        if let Some(ref key_pass) = self.ssl_key_password {
            config.set("ssl.key.password", key_pass);
        }

        config
    }
}

pub const TOPIC_TELEMETRY_RAW: &str = "telemetry.raw";
pub const TOPIC_TELEMETRY_PROCESSED: &str = "telemetry.processed";
pub const TOPIC_MISSION_HEALTH: &str = "mission.health";
pub const TOPIC_GROUNDSTATION_HEALTH: &str = "groundstation.health";
pub const TOPIC_CONJUNCTION_ALERTS: &str = "conjunction.alerts";
pub const TOPIC_CONJUNCTION_MANEUVERS: &str = "conjunction.maneuvers";

pub static KAFKA_CONSUMER_LAG: std::sync::LazyLock<prometheus::GaugeVec> =
    std::sync::LazyLock::new(|| {
        prometheus::register_gauge_vec!(
            "kafka_consumer_lag",
            "Lag of the consumer group for a topic and partition",
            &["topic", "partition"]
        )
        .unwrap()
    });

pub struct MetricsContext;

impl rdkafka::client::ClientContext for MetricsContext {
    fn stats_raw(&self, statistics: &[u8]) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(statistics) {
            if let Some(topics) = v.get("topics").and_then(|t| t.as_object()) {
                for (topic_name, topic_val) in topics {
                    if let Some(partitions) =
                        topic_val.get("partitions").and_then(|p| p.as_object())
                    {
                        for (partition_id, partition_val) in partitions {
                            if let Some(lag) =
                                partition_val.get("consumer_lag").and_then(|l| l.as_i64())
                            {
                                KAFKA_CONSUMER_LAG
                                    .with_label_values(&[topic_name, partition_id])
                                    .set(lag as f64);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl rdkafka::consumer::ConsumerContext for MetricsContext {}
