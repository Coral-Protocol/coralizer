use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct CoralAgent {
    pub agent: AgentDetails,
    pub options: Option<HashMap<String, AgentOption>>,
    pub runtimes: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct AgentDetails {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentOption {
    #[serde(rename = "type")]
    pub kind: String,
    pub required: Option<bool>,
    pub description: Option<String>,
}

impl CoralAgent {
    pub fn from_toml(content: &str) -> Result<Self, toml_edit::de::Error> {
        toml_edit::de::from_str(content)
    }
}
