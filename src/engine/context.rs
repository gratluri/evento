use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug)]
pub struct ProtocolClients {
    pub http: Option<reqwest::Client>,
    // Postgres pool is Clone (internally Arc)
    pub postgres: Option<sqlx::PgPool>,
    // Kafka client is Clone (internally Arc)
    pub kafka: Option<std::sync::Arc<rskafka::client::Client>>,
    // Cassandra session is Clone (internally Arc)
    pub cassandra: Option<std::sync::Arc<scylla::Session>>,
}

/// Context holds the variables extracted and manipulated by a Virtual User (VU) during a test run.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VuContext {
    pub vu_id: u32,
    pub run_id: String,
    pub variables: HashMap<String, String>,
    #[serde(skip)]
    pub clients: ProtocolClients,
}

impl VuContext {
    pub fn new(run_id: String, vu_id: u32) -> Self {
        Self {
            vu_id,
            run_id,
            variables: HashMap::new(),
            clients: ProtocolClients::default(),
        }
    }

    /// Lazily initialize and get the HTTP client
    pub fn get_http_client(&mut self) -> &reqwest::Client {
        if self.clients.http.is_none() {
            self.clients.http = Some(
                reqwest::Client::builder()
                    .cookie_store(true) // Maintain session cookies per VU
                    .build()
                    .unwrap_or_default(),
            );
        }
        self.clients.http.as_ref().unwrap()
    }

    pub async fn get_postgres_client(&mut self, endpoint: &str) -> Result<&sqlx::PgPool, anyhow::Error> {
        if self.clients.postgres.is_none() {
            let pool = sqlx::PgPool::connect(endpoint).await?;
            self.clients.postgres = Some(pool);
        }
        Ok(self.clients.postgres.as_ref().unwrap())
    }

    pub async fn get_kafka_client(&mut self, endpoint: &str) -> Result<std::sync::Arc<rskafka::client::Client>, anyhow::Error> {
        if self.clients.kafka.is_none() {
            let connection = rskafka::client::ClientBuilder::new(vec![endpoint.to_string()])
                .build()
                .await?;
            self.clients.kafka = Some(std::sync::Arc::new(connection));
        }
        Ok(self.clients.kafka.as_ref().unwrap().clone())
    }

    pub async fn get_cassandra_client(&mut self, endpoint: &str) -> Result<std::sync::Arc<scylla::Session>, anyhow::Error> {
        if self.clients.cassandra.is_none() {
            let session = scylla::SessionBuilder::new()
                .known_node(endpoint)
                .build()
                .await?;
            self.clients.cassandra = Some(std::sync::Arc::new(session));
        }
        Ok(self.clients.cassandra.as_ref().unwrap().clone())
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// Replaces occurrences of `${variable_name}` in the input string with values from the context.
    pub fn interpolate(&self, input: &str) -> String {
        let mut result = input.to_string();
        for (k, v) in &self.variables {
            let placeholder = format!("${{{}}}", k);
            result = result.replace(&placeholder, v);
        }
        result
    }
}
