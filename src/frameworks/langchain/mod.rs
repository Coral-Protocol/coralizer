use std::fmt::Write as _;
use std::io;
use std::path::Path;

use itertools::Itertools;
use regex::Regex;
use toml_edit::{DocumentMut, Formatted};

use crate::Mcp;
use crate::frameworks::Template;

pub struct Langchain;

impl Template for Langchain {
    fn name() -> &'static str {
        "langchain-agent"
    }
    fn artifact() -> (&'static str, &'static str) {
        (
            "https://github.com/Coral-Protocol/langchain-agent/archive/fb3a82a5ff436f3b8cb68a6902aed5dfa77ba7d9.zip",
            "fb3a82a5ff436f3b8cb68a6902aed5dfa77ba7d9.zip",
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
    fn is_templated_file(entry: &Path) -> bool {
        if entry.file_name().map(|n| n == "main.py").unwrap_or(false) {
            return true;
        }
        false
    }
    fn template(mcps: &[Mcp], contents: &str) -> String {
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

    fn post_process(root: &Path, agent_name: &str) -> std::io::Result<()> {
        let pyproject_path = root.join("pyproject.toml");
        let mut pyproject: DocumentMut = std::fs::read_to_string(&pyproject_path)?.parse().unwrap();

        let Some(project_name) = pyproject
            .get_mut("project")
            .and_then(|e| e.get_mut("name"))
            .and_then(|e| e.as_value_mut())
        else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "No project.name key found in pyproject.toml!",
            ));
        };
        *project_name = toml_edit::Value::String(Formatted::new(agent_name.to_string()));

        let Some(project_desc) = pyproject
            .get_mut("project")
            .and_then(|e| e.get_mut("description"))
            .and_then(|e| e.as_value_mut())
        else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "No project.name key found in pyproject.toml!",
            ));
        };
        *project_desc =
            toml_edit::Value::String(Formatted::new("Coralized langchain agent".into()));

        std::fs::write(pyproject_path, pyproject.to_string())?;
        Ok(())
    }
}
