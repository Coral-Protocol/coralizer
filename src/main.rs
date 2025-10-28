use clap::Parser;
use ignore::{WalkBuilder, WalkState};
use inquire::{error::InquireResult, validator::ValueRequiredValidator};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::{self, File},
    hash::Hash,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use toml_edit::{DocumentMut, Formatted, InlineTable, TableLike};
use zip::read::root_dir_common_filter;

use crate::frameworks::{Framework, Langchain, Template};
use crate::mcp_server::McpServers;
use custom_derive::custom_derive;
use enum_derive::*;

#[derive(Parser)]
pub enum Cli {
    Mcp(McpParams),
}
#[derive(clap::Args)]
pub struct McpParams {
    pub path: PathBuf,
    pub mcp_servers_path: PathBuf,

    #[arg(long, short)]
    pub framework: Option<Framework>,
    #[arg(long, short)]
    pub name: Option<String>,
}

pub mod frameworks;
pub mod mcp_server;

pub mod languages {
    use custom_derive::custom_derive;
    use enum_derive::*;

    custom_derive! {
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, EnumDisplay, IterVariants(Languages))]
        pub enum Language {
            Python,
            Rust,
        }
    }
}

custom_derive! {
    #[derive(Debug, IterVariants(TransportKind))]
    pub enum McpKind {
        Npx,
        Stdio,
        Sse,
    }
}

impl Display for McpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            McpKind::Npx => "npx",
            McpKind::Stdio => "stdio",
            McpKind::Sse => "sse",
        })
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Runtime {
    Npx,
}

impl McpKind {
    fn runtime(&self) -> Option<Runtime> {
        match self {
            McpKind::Npx => Some(Runtime::Npx),
            McpKind::Stdio | McpKind::Sse => None,
        }
    }
    fn env_wizard() -> InquireResult<HashMap<String, String>> {
        let mut envs = HashMap::new();
        loop {
            let env_name = inquire::Text::new(
                "Environment variable name (leave blank to stop adding env vars)",
            )
            .prompt()?;
            if env_name.is_empty() {
                break;
            }
            let opt_name = inquire::Text::new("Name of the agent option for this env var")
                .with_placeholder(&env_name)
                .prompt()?;

            let opt_name = match opt_name.is_empty() {
                true => env_name.clone(),
                false => opt_name,
            };

            envs.insert(env_name, opt_name);
        }
        Ok(envs)
    }
    pub fn wizard(self) -> InquireResult<Mcp> {
        Ok(match self {
            McpKind::Npx => {
                let package = inquire::Text::new("Name of the npm package (e.g '@org/mcp-server')")
                    .prompt()?;

                let envs = Self::env_wizard()?;

                let mut args = vec!["-y".into(), package];

                let extra_args =
                    inquire::Text::new("Additional command line arguments (leave blank if none)")
                        .prompt()?;
                args.extend(extra_args.split_whitespace().map(|s| s.to_owned()));

                Mcp::Stdio {
                    command: "npx".to_string(),
                    args,
                    env: Some(envs),
                }
            }
            McpKind::Stdio => {
                let raw = inquire::Text::new("Command to run")
                    .with_validator(ValueRequiredValidator::default())
                    .prompt()?;

                let mut raw = raw.split_whitespace().map(|a| a.to_string());
                let command = raw.next().expect("at least one");
                let args = raw.collect_vec();

                let env = Some(Self::env_wizard()?);

                Mcp::Stdio { command, args, env }
            }
            McpKind::Sse => todo!(),
        })
    }
}

pub enum Mcp {
    Stdio {
        command: String,
        args: Vec<String>,
        env: Option<HashMap<String, String>>,
    },
    Sse {},
}

impl Mcp {
    pub fn options(&self) -> Vec<String> {
        match self {
            Mcp::Stdio { env, .. } => env
                .as_ref()
                .map(|e| e.values().cloned().collect_vec())
                .unwrap_or_default(),
            Mcp::Sse {} => todo!(),
        }
    }
}

async fn mcp_wizard(params: McpParams) -> InquireResult<()> {
    if fs::exists(&params.path).unwrap() {
        if !inquire::Confirm::new(&format!(
            "'{}' already exists - continue & delete existing?",
            params.path.as_path().display()
        ))
        .with_default(false)
        .prompt()?
        {
            println!("Cancelled.");
            return Ok(());
        }
    } else {
        fs::create_dir_all(&params.path)?;
    }

    let agent_name = params.name.unwrap_or_else(|| {
        params
            .path
            .canonicalize()
            .unwrap()
            .file_name()
            .expect("folder name not in output path")
            .to_str()
            .expect("folder name to be utf8")
            .to_string()
    });

    let framework = match params.framework {
        Some(framework) => framework,
        None => inquire::Select::new(
            "Choose a framework",
            Framework::iter_variants()
                .sorted_by_key(|f| f.language())
                .collect::<Vec<_>>(),
        )
        .prompt()?,
    };

    let mut mcp_servers: McpServers = serde_json::from_str(
        fs::read_to_string(params.mcp_servers_path)
            .unwrap()
            .as_str(),
    )
    .expect("invalid json");
    mcp_servers
        .servers
        .values_mut()
        .flat_map(|server| server.env.iter_mut().flat_map(|env| env.iter_mut()))
        .for_each(|(k, v)| *v = k.clone());

    let description = mcp_servers.generate_description().await;
    let mcps: Vec<Mcp> = mcp_servers.into();

    // todo: alan what was the purpose of this...
    let runtimes: HashSet<Runtime> = HashSet::from([Runtime::Npx]);

    match framework {
        Framework::Langchain => {
            let templater = Arc::new(Langchain { runtimes });
            let dirs = directories_next::ProjectDirs::from("com", "coral-protocol", "coralizer")
                .expect("cache dir");

            let extracted_path = dirs.cache_dir().join("templates").join(templater.name());
            fs::create_dir_all(&extracted_path).unwrap();

            let (url, artefact_name) = templater.artifact();
            let artefact_path = dirs.cache_dir().join("artefacts").join(artefact_name);
            fs::create_dir_all(artefact_path.parent().unwrap(/* Safety: see above */)).unwrap();

            let response = reqwest::get(url).await.unwrap().bytes().await.unwrap();

            let mut artefact = File::create(&artefact_path).unwrap();
            artefact.write_all(&response).unwrap();
            drop(artefact);

            let mut artefact = File::open(&artefact_path).unwrap();
            let mut archive = zip::ZipArchive::new(artefact).unwrap();
            archive
                .extract_unwrapped_root_dir(&extracted_path, root_dir_common_filter)
                .unwrap();

            let (tx, rx) = crossbeam::channel::unbounded();

            let agent_toml: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));

            fs::remove_dir_all(&params.path).unwrap();

            let mut builder = WalkBuilder::new(&extracted_path);

            builder
                .hidden(false)
                .git_ignore(true)
                .filter_entry(|entry| {
                    Langchain::include_file(entry) && !entry.path().ends_with(".git")
                });

            builder.build_parallel().run(|| {
                let tx = tx.clone();
                let agent_toml = agent_toml.clone();
                let templater = templater.clone();
                Box::new(move |entry| {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(e) => {
                            eprintln!("Error reading file - {e:?}");
                            return WalkState::Continue;
                        }
                    };
                    let path = entry.path();
                    println!("{}", path.display());

                    if !path.is_file() {
                        return WalkState::Continue;
                    }

                    if let Some(file_name) = path.file_name()
                        && file_name == "coral-agent.toml"
                        && agent_toml
                            .lock()
                            .unwrap()
                            .replace(path.to_path_buf())
                            .is_some()
                    {
                        eprintln!("Warning: found multiple coral-agent.toml's?")
                    }

                    if let Err(e) = tx.send(path.to_owned()) {
                        eprintln!("thread: {e:?}");
                        return WalkState::Quit;
                    }
                    WalkState::Continue
                })
            });

            drop(tx);

            let handle = std::thread::spawn(move || {
                while let Ok(path) = rx.recv() {
                    let rel_path = path
                        .strip_prefix(&extracted_path)
                        .expect("path to be in base");
                    let final_path = params.path.join(rel_path);

                    if let Some(parent) = final_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    match templater.is_templated_file(rel_path) {
                        true => {
                            let contents = std::fs::read_to_string(&path)?;
                            let contents = templater.template(&mcps, &contents);

                            std::fs::write(final_path, contents)?;
                        }

                        false => {
                            std::fs::copy(path, final_path)?;
                        }
                    }
                }
                let Some(agent_toml_path) = agent_toml.lock().unwrap().take() else {
                    eprintln!("No coral-agent.toml found in template source.");
                    return Ok(());
                };

                let mut agent_toml: DocumentMut =
                    std::fs::read_to_string(&agent_toml_path)?.parse().unwrap();

                // todo: alan (remove unwrap please, also, what happens if description set in template?...)
                agent_toml
                    .get_mut("agent")
                    .unwrap()
                    .as_table_mut()
                    .unwrap()
                    .insert("description", description.into());

                let Some(toml_agent_name) = agent_toml
                    .get_mut("agent")
                    .and_then(|agent| agent.get_mut("name"))
                    .and_then(|name| name.as_value_mut())
                else {
                    eprintln!("No agent.name key found in coral-agent.toml!");
                    return Ok(());
                };

                *toml_agent_name = toml_edit::Value::String(Formatted::new(agent_name.clone()));

                let Some(options) = agent_toml.get_mut("options") else {
                    eprintln!("No options table found in coral-agent.toml!");
                    return Ok(());
                };
                let options = options.as_table_mut().expect("'options' key to be a table");
                for opt in mcps.iter().flat_map(|mcp| mcp.options()) {
                    let mut table = InlineTable::new();
                    table.insert(
                        "type",
                        toml_edit::Value::String(Formatted::new("string".into())),
                    );
                    table.insert("required", toml_edit::Value::Boolean(Formatted::new(true)));
                    options.insert(
                        &opt,
                        toml_edit::Item::Value(toml_edit::Value::InlineTable(table)),
                    );
                }

                let final_toml = params.path.join(
                    agent_toml_path
                        .strip_prefix(&extracted_path)
                        .expect("path to be in base"),
                );

                std::fs::write(final_toml, agent_toml.to_string())?;

                templater.post_process(&params.path, &agent_name)?;

                Ok::<(), Box<std::io::Error>>(())
            });
            if let Err(e) = handle.join().expect("couldn't join on templating thread") {
                eprintln!("Templating failed - {e:?}");
                return Ok(());
            }
        }
        Framework::CoralRs => todo!(),
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli {
        Cli::Mcp(params) => {
            if let Err(e) = mcp_wizard(params).await {
                match e {
                    inquire::InquireError::OperationCanceled
                    | inquire::InquireError::OperationInterrupted => {
                        eprintln!("\nCancelled.");
                    }
                    e => panic!("{e:?}"),
                }
            }
        }
    }
}
