use rdkafka::config::ClientConfig;

#[derive(Debug, Clone, Default)]
pub struct KafkaConfigBuilder {
    pub brokers: String,
    pub security_protocol: Option<String>,
    pub sasl_mechanism: Option<String>,
    pub sasl_username: Option<String>,
    pub sasl_password: Option<String>,
    pub ssl_ca_location: Option<String>,
}

impl KafkaConfigBuilder {
    pub fn new(brokers: String) -> Self {
        Self {
            brokers,
            ..Default::default()
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

        config
    }
}
