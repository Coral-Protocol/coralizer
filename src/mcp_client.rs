use rmcp::{
    RmcpError, ServiceExt as _,
    model::{ClientCapabilities, ClientInfo, Implementation, InitializeRequestParam},
    service::RunningService,
    transport::{
        ConfigureCommandExt as _, SseClientTransport, StreamableHttpClientTransport,
        TokioChildProcess,
    },
};
use tokio::process::Command;

use crate::mcp_server::McpServer;

pub enum Client {
    Local(RunningService<rmcp::RoleClient, ()>),
    Network(RunningService<rmcp::RoleClient, InitializeRequestParam>),
}

impl Client {
    pub fn peer(&self) -> &rmcp::Peer<rmcp::RoleClient> {
        match self {
            Client::Local(s) => s.peer(),
            Client::Network(s) => s.peer(),
        }
    }
    pub async fn list_all_tools(&self) -> Result<Vec<rmcp::model::Tool>, rmcp::ServiceError> {
        self.peer().list_all_tools().await
    }

    pub async fn cancel(self) -> Result<rmcp::service::QuitReason, tokio::task::JoinError> {
        match self {
            Client::Local(s) => s.cancel().await,
            Client::Network(s) => s.cancel().await,
        }
    }
}

pub async fn make_client(mcp: &McpServer) -> anyhow::Result<Client> {
    let client_info = || ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: env!("CARGO_PKG_NAME").into(),
            title: None,
            version: env!("CARGO_PKG_VERSION").into(),
            website_url: None,
            icons: None,
        },
    };
    Ok(match mcp {
        McpServer::Stdio { command, args, env } => Client::Local(
            /*
               😡

               Many places will present a JSON blob that the user can paste somewhere to get a server running;
               but these same places do not allow any OS specific quirks to be specified, and they often use hacky
               workarounds to like NPX to get things running.

               The name of the NPX script under Windows is npx.cmd, the command would have to be "cmd /c npx" or similar
               to invoke it on Windows without needing to specify the extension explicitly.
            */
            ().serve(
                TokioChildProcess::new(
                    Command::new(match command.as_str() {
                        "npx" if cfg!(windows) => "npx.cmd",
                        command => command,
                    })
                    .configure(|cmd| {
                        cmd.args(args);
                        if let Some(env) = env {
                            cmd.envs(env.keys().map(|k| (k.to_string(), "dummy".to_string())));
                        }
                    }),
                )
                .map_err(RmcpError::transport_creation::<TokioChildProcess>)?,
            )
            .await?,
        ),
        McpServer::Sse { url, .. } => {
            let client = client_info()
                .serve(SseClientTransport::start(url.to_owned()).await?)
                .await?;
            Client::Network(client)
        }
        McpServer::Http { url, .. } => {
            let client = client_info()
                .serve(StreamableHttpClientTransport::from_uri(url.to_owned()))
                .await?;
            Client::Network(client)
        }
    })
}
