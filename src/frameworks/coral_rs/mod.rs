use std::collections::HashSet;
use std::fmt::Write as _;
use std::io;
use std::path::Path;
use std::sync::Arc;

use quote::quote;
use regex::Regex;
use toml_edit::{DocumentMut, value};

use crate::Runtime;
use crate::edit::edit_file_str;
use crate::frameworks::Template;
use crate::mcp_server::{McpServer, McpServers};

#[derive(Clone)]
pub struct CoralRs {
    pub runtimes: Arc<HashSet<Runtime>>,
    pub mcps: Arc<McpServers>,
}

impl Template for CoralRs {
    fn name(&self) -> &'static str {
        "coral-rs"
    }
    fn artifact(&self) -> (&'static str, &'static str) {
        (
            "https://github.com/Coral-Protocol/coral-rs-agent/archive/d55baba502dd17e8a885b4f0d4b70c7613351834.zip",
            "d55baba502dd17e8a885b4f0d4b70c7613351834.zip",
        )
    }
    fn include_file(entry: &ignore::DirEntry) -> bool {
        if entry
            .file_name()
            .to_str()
            .map(|n| n.starts_with("flake"))
            .unwrap_or(false)
        {
            return false;
        }
        true
    }
    fn is_templated_file(&self, entry: &Path) -> bool {
        if entry.file_name().map(|n| n == "main.rs").unwrap_or(false) {
            return true;
        }
        false
    }
    fn template(&self, contents: &str) -> String {
        let mcp_client_re =
            Regex::new(r#"let mut agent = Agent::new\((?:.|\w|\n)*?(\n\w*\n)"#).unwrap();
        let mut contents = contents.to_string();
        if let Some(m) = mcp_client_re.find(&contents) {
            let mut s = String::new();
            writeln!(s, ",").unwrap();
            let mut servers = vec![];
            for (mcp_name, mcp) in &self.mcps.servers {
                // TODO (alan): dedupe this
                servers.push(match mcp {
                    McpServer::Stdio { command, args, env } => {
                        if let Some(e) = &env && !e.is_empty() {
                            eprintln!("coral-rs does not support passing environment variables to stdio MCP Servers!");
                        }
                        let err_msg = format!("failed to spawn stdio mcp server '{mcp_name}'");
                        quote! {
                            mcp_server(McpConnectionBuilder::stdio(#command, [#(#args),*], #mcp_name).connect().await.expect(#err_msg))
                        }
                    }
                    McpServer::Http { .. } => {
                        eprintln!("MCP Servers with http transport not supported in coral-rs!");
                        continue;
                    }
                    McpServer::Sse { url, headers } => {
                        let err_msg = format!("failed to connect to sse mcp server '{mcp_name}'");
                        if let Some(h) = &headers && !h.is_empty() {
                            eprintln!("coral-rs does not support passing headers to SSE MCP Servers!");
                        }
                        quote! {
                            mcp_server(McpConnectionBuilder::sse(#url).connect().await.expect(#err_msg))
                        }
                    }
                });
            }
            let servers = servers.into_iter().fold(quote! {agent}, |acc, ident| {
                quote! { #acc.#ident }
            });
            let tokens = quote! {
                agent = #servers;
            };
            let text = format!("    {tokens}\n");
            contents.insert_str(m.end() + 1, &text);
        } else {
            panic!("bad");
        }
        contents
    }

    fn post_process(&self, root: &Path, agent_name: &str) -> std::io::Result<()> {
        if let Err(e) = edit_file_str(root.join("Cargo.toml"), |contents| {
            let mut toml: DocumentMut = contents.parse().unwrap();
            toml["package"]["name"] = value(agent_name);
            Ok::<_, io::Error>(toml.to_string())
        }) {
            eprintln!("error modifying Cargo.toml - {e:?}");
        }

        match std::process::Command::new("cargo")
            .arg("fmt")
            .current_dir(root)
            .spawn()
        {
            Ok(_) => println!("Formatted"),
            Err(_) => {
                eprintln!("Failed to format coralized project. Code may look weird.");
            }
        }

        // if self.runtimes.contains(&Runtime::Npx) {
        //     print!("🔧 {:>18} fixup", style("'Dockerfile'").blue());
        //
        //     let dockerfile_path = root.join("Dockerfile");
        //     let mut dockerfile = std::fs::read_to_string(&dockerfile_path)?;
        //
        //     const NEEDLE: &str = "COPY --from=builder --chown=app:app /app/ /app/";
        //     let off = dockerfile
        //         .find(NEEDLE)
        //         .ok_or_else(|| io::Error::other("Could not find relevant line in Dockerfile"))?;
        //
        //     // dockerfile.insert_str(off, include_str!("./nodejs.Dockerfile"));
        //
        //     println!(
        //         " -> {}",
        //         style(format!("'{}'", dockerfile_path.display())).blue()
        //     );
        //     std::fs::write(dockerfile_path, dockerfile)?;
        // }

        Ok(())
    }
}
