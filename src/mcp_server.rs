use crate::Mcp;
use itertools::Itertools;
use mcp_runner::config::ServerConfig;
use mcp_runner::{Config, McpRunner};
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai;
use serde::Deserialize;
use std::collections::HashMap;

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
#[derive(Deserialize)]
pub struct McpServers {
    #[serde(rename = "mcpServers")]
    pub servers: HashMap<String, McpServer>,
}

#[derive(Deserialize)]
pub struct McpServer {
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<HashMap<String, String>>,
}

impl Into<Vec<Mcp>> for McpServers {
    fn into(self) -> Vec<Mcp> {
        self.servers
            .into_values()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

impl Into<Mcp> for McpServer {
    fn into(self) -> Mcp {
        Mcp::Stdio {
            command: self.command,
            args: self.args,
            env: self.env,
        }
    }
}

impl McpServers {
    pub async fn generate_description(&self) -> String {
        let mut runner = McpRunner::new(Config {
            mcp_servers: self
                .servers
                .iter()
                .map(|(name, server)| {
                    println!("will run {} with args {:?}", server.command, server.args);
                    (
                        name.clone(),
                        ServerConfig {
                            /*
                               😡

                               Many places will present a JSON blob that the user can paste somewhere to get a server running;
                               but these same places do not allow any OS specific quirks to be specified, and they often use hacky
                               workarounds to like NPX to get things running.

                               The name of the NPX script under Windows is npx.cmd, the command would have to be "cmd /c npx" or similar
                               to invoke it on Windows without needing to specify the extension explicitly.
                            */
                            command: if server.command.eq("npx") {
                                if cfg!(windows) {
                                    "npx.cmd".to_string()
                                } else {
                                    "npx".to_string()
                                }
                            } else {
                                server.command.clone()
                            },
                            args: server.args.clone(),
                            env: server.env.clone().unwrap_or_default(),
                        },
                    )
                })
                .collect(),
            sse_proxy: None,
        });

        let openai_client = openai::Client::from_env();

        runner
            .start_all_servers()
            .await
            .expect("failed to start servers");
        let tools = runner.get_all_server_tools().await;
        let tool_str = tools
            .values()
            .flatten()
            .flat_map(serde_json::to_string)
            .join("\n\n");

        let gpt4 = openai_client
            .agent("gpt-4")
            .preamble("You are a helpful assistant.")
            .build();

        // Prompt the model and print its response
        let response = gpt4
            .prompt(format!(r#"
            We are making an agent with access to the the following tooling:
            # start of tooling
            {tool_str}
            # end of tooling

            This agent is being generated around this tooling to represent its capabilities and responsibilities as an agent to other agents.
            Other agents, as well as human developers will use the agent's description to determine whether it is relevant to communicate with and use.

            With these tools in mind, generate a short description (10 - 50 words) that describes the agent's capabilities and responsibilities."#))
            .await
            .expect("Failed to prompt GPT-4");

        runner
            .stop_all_servers()
            .await
            .expect("failed to stop servers");
        response
    }
}
