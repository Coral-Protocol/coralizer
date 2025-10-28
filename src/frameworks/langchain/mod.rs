use std::collections::HashSet;
use std::fmt::Write as _;
use std::io;
use std::path::Path;

use itertools::Itertools;
use regex::Regex;
use toml_edit::{DocumentMut, Formatted};

use crate::frameworks::Template;
use crate::{Mcp, Runtime};

#[derive(Clone)]
pub struct Langchain {
    pub runtimes: HashSet<Runtime>,
}

impl Template for Langchain {
    fn name(&self) -> &'static str {
        "langchain-agent"
    }
    fn artifact(&self) -> (&'static str, &'static str) {
        (
            "https://github.com/Coral-Protocol/langchain-agent/archive/d77845581b94e17c39bfcf0f57c6faf89bdc90d2.zip",
            "d77845581b94e17c39bfcf0f57c6faf89bdc90d2.zip",
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
        if entry.file_name().map(|n| n == "main.py").unwrap_or(false) {
            return true;
        }
        false
    }
    fn template(&self, mcps: &[Mcp], contents: &str) -> String {
        let mcp_client_re =
            Regex::new(r#"MultiServerMCPClient\s*\(\s*connections\s*=\s*\{\s*"coral"\s*:\s*\{(\s*".*,\n)*(\s*)}"#)
                .unwrap();
        let mut contents = contents.to_string();
        if let Some(caps) = mcp_client_re.captures(&contents) {
            let group = caps.get(2).expect("group 2 to exist");
            let ind = " ".repeat(group.len());
            let mut s = String::new();
            writeln!(s, ",").unwrap();
            for (i, mcp) in mcps.iter().enumerate() {
                match mcp {
                    Mcp::Stdio { command, args, env } => {
                        let args = args.iter().map(|a| format!("\"{a}\"")).collect_vec();
                        writeln!(s, r#"{ind}"TODO": {{"#).unwrap();
                        writeln!(s, r#"{ind}    "transport": "stdio","#).unwrap();
                        writeln!(s, r#"{ind}    "command": "{command}","#).unwrap();
                        if let Some(env) = env
                            && !env.is_empty()
                        {
                            writeln!(s, r#"{ind}    "env": {{"#).unwrap();
                            for (i, (env, opt)) in env.iter().enumerate() {
                                write!(s, r#"{ind}        "{env}": asserted_env("{opt}")"#)
                                    .unwrap();
                                match i + 1 == env.len() {
                                    true => writeln!(s, ","),
                                    false => writeln!(s),
                                }
                                .unwrap()
                            }
                            writeln!(s, r#"{ind}    }},"#).unwrap();
                        }
                        writeln!(s, r#"{ind}    "args": [{}]"#, args.join(", ")).unwrap();
                        write!(s, r#"{ind}}}"#).unwrap();
                        match i + 1 == mcps.len() {
                            true => write!(s, ","),
                            false => writeln!(s),
                        }
                        .unwrap()
                    }
                    Mcp::Sse {} => todo!(),
                }
            }
            contents.insert_str(group.end() + 1, &s);
        } else {
            panic!("bad");
        }
        contents
    }

    fn post_process(&self, root: &Path, agent_name: &str) -> std::io::Result<()> {
        let pyproject_path = root.join("pyproject.toml");
        println!("`pyproject.toml` fixup...");
        let mut pyproject: DocumentMut = std::fs::read_to_string(&pyproject_path)?.parse().unwrap();

        let Some(project_name) = pyproject
            .get_mut("project")
            .and_then(|e| e.get_mut("name"))
            .and_then(|e| e.as_value_mut())
        else {
            return Err(io::Error::other(
                "No project.name key found in pyproject.toml!",
            ));
        };
        *project_name = toml_edit::Value::String(Formatted::new(agent_name.to_string()));

        let Some(project_desc) = pyproject
            .get_mut("project")
            .and_then(|e| e.get_mut("description"))
            .and_then(|e| e.as_value_mut())
        else {
            return Err(io::Error::other(
                "No project.name key found in pyproject.toml!",
            ));
        };
        *project_desc =
            toml_edit::Value::String(Formatted::new("Coralized langchain agent".into()));

        println!("Writing final 'pyproject.toml' to {pyproject_path:?}...");
        std::fs::write(pyproject_path, pyproject.to_string())?;

        let dockerfile_path = root.join("Dockerfile");
        println!("'Dockerfile' fixup...");
        let mut dockerfile = std::fs::read_to_string(&dockerfile_path)?;

        const NEEDLE: &str = "COPY --from=builder --chown=app:app /app/ /app/";
        let off = dockerfile
            .find(NEEDLE)
            .ok_or_else(|| io::Error::other("Could not find relevant line in Dockerfile"))?;

        dockerfile.insert_str(off, include_str!("./nodejs.Dockerfile"));

        println!("Writing final 'Dockerfile' to {dockerfile_path:?}...");
        std::fs::write(dockerfile_path, dockerfile)?;

        Ok(())
    }
}
