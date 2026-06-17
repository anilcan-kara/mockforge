use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockConfig {
    pub tenants: Vec<Tenant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub name: String,
    pub prefix: Option<String>,
    pub host: Option<String>,
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub method: String,
    pub state: Option<String>,
    pub rules: Option<Vec<Rule>>,
    pub default: Option<MockResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub r#if: String,
    pub status: Option<u16>,
    pub body: Option<serde_json::Value>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    pub status: Option<u16>,
    pub body: Option<serde_json::Value>,
    pub headers: Option<HashMap<String, String>>,
}

impl MockConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let config: MockConfig = serde_yaml::from_reader(file)?;
        Ok(config)
    }
}
