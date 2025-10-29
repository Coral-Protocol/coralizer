use colored::Colorize as _;
use futures_util::future::join_all;
use indicatif::ProgressBar;
use itertools::Itertools;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;

use crate::Runtime;
use crate::mcp_client::make_client;

///
/// Follows this:
/// todo: find an actual spec for this!!!!
/// ```json
/// {
///   "mcpServers": {
///     "filesystem": {
///       "command": "npx",
///       "args": [
///         "-y",
///         "@modelcontextprotocol/server-filesystem",
///         "C:\\Users\\username\\Desktop",
///         "C:\\Users\\username\\Downloads"
///       ]
///     }
///   }
/// }```
#[derive(Debug, Clone, Deserialize)]
pub struct McpServers {
    #[serde(rename = "mcpServers")]
    pub servers: HashMap<String, McpServer>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "transport")]
pub enum McpServer {
    #[serde(rename = "sse")]
    Sse {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    #[serde(rename = "http")]
    #[serde(alias = "streamableHttp")]
    Http {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    #[serde(untagged)]
    Stdio {
        command: String,
        args: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
    },
}

impl McpServer {
    pub fn runtime(&self) -> Option<Runtime> {
        match self {
            Self::Stdio { command, .. } => {
                if command.contains("npx") {
                    Some(Runtime::Npx)
                } else {
                    None
                }
            }
            Self::Http { .. } => None,
            Self::Sse { .. } => None,
        }
    }
    pub fn options(&self) -> Option<Vec<String>> {
        Some(match self {
            Self::Stdio { env, .. } => env.as_ref()?.values().cloned().collect_vec(),
            Self::Http { headers, .. } | Self::Sse { headers, .. } => {
                headers.as_ref()?.values().cloned().collect_vec()
            }
        })
    }
}

impl From<McpServers> for Vec<McpServer> {
    fn from(val: McpServers) -> Self {
        val.servers.into_values().collect()
    }
}

impl McpServers {
    pub async fn generate_description(&self, pb: ProgressBar) -> String {
        pb.set_message("Starting MCP server(s)");
        let mut tools = vec![];
        let mut clients = vec![];
        for (name, server) in self.servers.iter() {
            match make_client(server).await {
                Ok(client) => {
                    match client.list_all_tools().await {
                        Ok(t) => tools.extend(t),
                        Err(e) => {
                            eprintln!("Failed to list tools from MCP '{name}' - {e:?}");
                        }
                    }
                    clients.push(client);
                }
                Err(e) => {
                    eprintln!("Failed to connect to '{name}' - {e:?}");
                }
            }
        }
        let close_all = clients.into_iter().map(|client| client.cancel());
        let close_all = tokio::spawn(join_all(close_all));

        println!("✅ {}", format!("Found {} tools.", tools.len()).green());

        let openai_client = openai::Client::from_env();

        let tool_str = tools.iter().flat_map(serde_json::to_string).join("\n\n");

        let response = match env::var("SKIP_LLM").is_ok() {
            true => {
                // TODO: ask for description
                String::from("DUMMY DESCRIPTION")
            }
            false => {
                let gpt4 = openai_client
                    .agent("gpt-4")
                    .preamble("You are a helpful assistant.")
                    .build();

                pb.set_message("Generating agent description...");
                gpt4
                    .prompt(
                        format!(r#"
We are making an agent with access to the the following tooling:
# start of tooling
{tool_str}
# end of tooling

This agent is being generated around this tooling to represent its capabilities and responsibilities as an agent to other agents.
Other agents, as well as human developers will use the agent's description to determine whether it is relevant to communicate with and use.

With these tools in mind, generate a short description (10 - 50 words) that describes the agent's capabilities and responsibilities."#).trim())
                    .await
                    .expect("Failed to prompt GPT-4")
            }
        };

        let _ = close_all.await;
        response
    }
}
