use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Context holds the variables extracted and manipulated by a Virtual User (VU) during a test run.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VuContext {
    pub vu_id: u32,
    pub run_id: String,
    pub variables: HashMap<String, String>,
}

impl VuContext {
    pub fn new(run_id: String, vu_id: u32) -> Self {
        Self {
            vu_id,
            run_id,
            variables: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }
}
