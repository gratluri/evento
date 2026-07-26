use crate::engine::context::VuContext;
use crate::dsl::dsl_parser::Step;
use anyhow::Result;
use std::collections::HashMap;
use async_trait::async_trait;
use reqwest::Method;

/// Represents a standardized response from any protocol (HTTP, Kafka, DB, etc.)
#[derive(Debug, Clone)]
pub struct ProtocolResponse {
    pub status_code: u16,
    pub latency_ms: u64,
    pub headers: HashMap<String, String>,
    pub body_json: Option<serde_json::Value>,
    pub body_text: Option<String>,
    pub body_bytes: Option<Vec<u8>>,
}

impl ProtocolResponse {
    pub fn extract(&self, path: &str) -> Option<String> {
        if let Some(header_key) = path.strip_prefix("header.") {
            return self.headers.get(header_key).cloned();
        }
        
        if let Some(json_path) = path.strip_prefix("json.") {
            if let Some(json) = &self.body_json {
                let mut current = json;
                for part in json_path.split('.') {
                    if let Some(next) = current.get(part) {
                        current = next;
                    } else if let Ok(idx) = part.parse::<usize>() {
                        if let Some(next) = current.get(idx) {
                            current = next;
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                
                if let Some(s) = current.as_str() {
                    return Some(s.to_string());
                } else {
                    return Some(current.to_string());
                }
            }
        }
        None
    }
}

/// A trait that defines how a protocol should be executed.
#[async_trait]
pub trait ProtocolExecutor {
    /// Executes the given step and returns a ProtocolResponse.
    async fn execute(&self, step: &Step, ctx: &mut VuContext) -> Result<ProtocolResponse>;
}

/// HTTP Protocol implementation
pub struct HttpExecutor;

#[async_trait]
impl ProtocolExecutor for HttpExecutor {
    async fn execute(&self, step: &Step, ctx: &mut VuContext) -> Result<ProtocolResponse> {
        let method = step.method.as_deref().unwrap_or("GET");
        let reqwest_method = match method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            _ => Method::GET,
        };

        let endpoint_str = step.endpoint.as_deref().unwrap_or("/");
        let url = ctx.interpolate(endpoint_str);

        let mut request = ctx.get_http_client().request(reqwest_method, &url);

        if let Some(headers) = &step.headers {
            for (k, v) in headers {
                request = request.header(k, ctx.interpolate(v));
            }
        }

        if let Some(body_val) = &step.body {
            if let Some(body_str) = body_val.as_str() {
                request = request.body(ctx.interpolate(body_str));
            } else {
                // If it's a JSON object, serialize to string, interpolate, then set as body
                let mut body_str = serde_json::to_string(body_val).unwrap_or_default();
                body_str = ctx.interpolate(&body_str);
                request = request.body(body_str);
                request = request.header("Content-Type", "application/json");
            }
        }

        let start = std::time::Instant::now();
        
        let response = request.send().await?;
        let latency_ms = start.elapsed().as_millis() as u64;
        let status_code = response.status().as_u16();

        let mut header_map = HashMap::new();
        for (k, v) in response.headers() {
            if let Ok(val) = v.to_str() {
                header_map.insert(k.as_str().to_string(), val.to_string());
            }
        }

        let bytes = response.bytes().await?.to_vec();
        
        let mut body_json = None;
        let mut body_text = None;

        if let Ok(text) = String::from_utf8(bytes.clone()) {
            body_text = Some(text.clone());
            if let Ok(json) = serde_json::from_str(&text) {
                body_json = Some(json);
            }
        }

        Ok(ProtocolResponse {
            status_code,
            latency_ms,
            headers: header_map,
            body_json,
            body_text,
            body_bytes: Some(bytes),
        })
    }
}

pub struct PostgresExecutor;

#[async_trait]
impl ProtocolExecutor for PostgresExecutor {
    async fn execute(&self, step: &Step, ctx: &mut VuContext) -> Result<ProtocolResponse> {
        let endpoint_str = step.connection.as_deref().or(step.endpoint.as_deref()).ok_or_else(|| anyhow::anyhow!("Postgres protocol requires an endpoint or connection"))?;
        let url = ctx.interpolate(endpoint_str);

        // Extract query from body or query
        let query_str = step.query.as_ref()
            .map(|s| s.as_str())
            .or_else(|| step.body.as_ref().and_then(|v| v.as_str()))
            .ok_or_else(|| anyhow::anyhow!("Postgres protocol requires a SQL query string in the 'query' or 'body' field"))?;
        
        let query_str = ctx.interpolate(query_str);
        
        let pool = ctx.get_postgres_client(&url).await?.clone();

        use sqlx::{Row, Column};
        let start = std::time::Instant::now();
        let rows = sqlx::query(&query_str).fetch_all(&pool).await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let mut result_obj = serde_json::Map::new();
        result_obj.insert("row_count".to_string(), serde_json::Value::Number(rows.len().into()));
        
        let mut rows_json = Vec::new();
        for row in rows {
            let mut row_map = serde_json::Map::new();
            for col in row.columns() {
                let name = col.name();
                // Attempt to extract as different types and convert to string
                let val: Option<String> = row.try_get::<String, _>(name).ok()
                    .or_else(|| row.try_get::<i32, _>(name).ok().map(|i| i.to_string()))
                    .or_else(|| row.try_get::<i64, _>(name).ok().map(|i| i.to_string()))
                    .or_else(|| row.try_get::<f64, _>(name).ok().map(|f| f.to_string()))
                    .or_else(|| row.try_get::<bool, _>(name).ok().map(|b| b.to_string()));
                
                if let Some(s) = val {
                    row_map.insert(name.to_string(), serde_json::Value::String(s));
                } else {
                    row_map.insert(name.to_string(), serde_json::Value::Null);
                }
            }
            rows_json.push(serde_json::Value::Object(row_map));
        }
        result_obj.insert("rows".to_string(), serde_json::Value::Array(rows_json));
        
        Ok(ProtocolResponse {
            status_code: 200,
            latency_ms,
            headers: HashMap::new(),
            body_json: Some(serde_json::Value::Object(result_obj)),
            body_text: None,
            body_bytes: None,
        })
    }
}

pub struct KafkaExecutor;

#[async_trait]
impl ProtocolExecutor for KafkaExecutor {
    async fn execute(&self, step: &Step, ctx: &mut VuContext) -> Result<ProtocolResponse> {
        let endpoint_str = step.connection.as_deref().or(step.endpoint.as_deref()).ok_or_else(|| anyhow::anyhow!("Kafka protocol requires an endpoint (brokers)"))?;
        let url = ctx.interpolate(endpoint_str);

        let client = ctx.get_kafka_client(&url).await?;

        let topic_name = step.headers.as_ref()
            .and_then(|h| h.get("topic"))
            .ok_or_else(|| anyhow::anyhow!("Kafka protocol requires a 'topic' header"))?;
        let topic_name = ctx.interpolate(topic_name);

        let partition_client = client.partition_client(topic_name.clone(), 0, rskafka::client::partition::UnknownTopicHandling::Retry).await?;

        let body_str = step.body.as_ref()
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if let Some(v) = &step.body {
                    serde_json::to_string(v).unwrap_or_default()
                } else {
                    "".to_string()
                }
            });
        
        let body_str = ctx.interpolate(&body_str);

        let start = std::time::Instant::now();
        use rskafka::record::Record;
        let record = Record {
            key: None,
            value: Some(body_str.into_bytes()),
            headers: std::collections::BTreeMap::new(),
            timestamp: chrono::Utc::now(),
        };

        partition_client.produce(vec![record], rskafka::client::partition::Compression::NoCompression).await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(ProtocolResponse {
            status_code: 200,
            latency_ms,
            headers: HashMap::new(),
            body_json: None,
            body_text: Some("Message produced".to_string()),
            body_bytes: None,
        })
    }
}

pub struct CassandraExecutor;

#[async_trait]
impl ProtocolExecutor for CassandraExecutor {
    async fn execute(&self, step: &Step, ctx: &mut VuContext) -> Result<ProtocolResponse> {
        let endpoint_str = step.connection.as_deref().or(step.endpoint.as_deref()).ok_or_else(|| anyhow::anyhow!("Cassandra protocol requires an endpoint or connection"))?;
        let url = ctx.interpolate(endpoint_str);

        // Extract query from body or query
        let query_str = step.query.as_ref()
            .map(|s| s.as_str())
            .or_else(|| step.body.as_ref().and_then(|v| v.as_str()))
            .ok_or_else(|| anyhow::anyhow!("Cassandra protocol requires a CQL query string in the 'query' or 'body' field"))?;
        
        let query_str = ctx.interpolate(query_str);
        
        let session = ctx.get_cassandra_client(&url).await?.clone();

        let start = std::time::Instant::now();
        // Execute CQL
        let result = session.query_unpaged(query_str, &[]).await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let mut result_obj = serde_json::Map::new();
        // We just return tracing info for MVP. To return full rows, we'd iterate result.rows.
        if let Some(rows) = result.rows {
            result_obj.insert("row_count".to_string(), serde_json::Value::Number(rows.len().into()));
        } else {
            result_obj.insert("row_count".to_string(), serde_json::Value::Number(0.into()));
        }
        
        Ok(ProtocolResponse {
            status_code: 200,
            latency_ms,
            headers: HashMap::new(),
            body_json: Some(serde_json::Value::Object(result_obj)),
            body_text: None,
            body_bytes: None,
        })
    }
}
