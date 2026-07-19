use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestPlan {
    pub name: String,
    pub description: Option<String>,
    pub metadata: Option<Metadata>,
    pub config: Option<Config>,
    pub data_sources: Option<Vec<DataSource>>,
    pub scenario: Vec<Step>,
}

impl TestPlan {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file {:?}", path.as_ref()))?;
        Self::from_yaml_str(&content)
    }

    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let plan: TestPlan = serde_yaml::from_str(yaml)
            .with_context(|| "Failed to parse YAML test plan")?;
        Ok(plan)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Metadata {
    pub author: Option<String>,
    pub created: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub mode: Option<String>,
    pub duration: Option<String>,
    pub virtual_users: Option<u32>,
    pub timeout: Option<String>,
    pub step_timeout: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataSource {
    pub name: String,
    pub source: String,
    pub connection: Option<String>,
    pub query: Option<String>,
    pub path: Option<String>,
    pub sampling: Option<String>,
    pub cache: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Step {
    pub name: String,
    pub protocol: Option<String>,
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<serde_yaml::Value>,
    #[serde(default)]
    pub r#async: Option<bool>,
    // Other fields can be added here (e.g., validate, retry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_plan() {
        let yaml = r#"
name: basic_http
description: Basic HTTP scenario
config:
  virtual_users: 10
  duration: 1m
scenario:
  - name: get_homepage
    protocol: https
    method: GET
    endpoint: /
"#;
        let plan: TestPlan = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(plan.name, "basic_http");
        assert_eq!(plan.config.unwrap().virtual_users.unwrap(), 10);
        assert_eq!(plan.scenario.len(), 1);
        assert_eq!(plan.scenario[0].name, "get_homepage");
    }
}
