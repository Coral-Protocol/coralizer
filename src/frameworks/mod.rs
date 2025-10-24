use custom_derive::custom_derive;
use enum_derive::*;
use std::{fmt::Display, path::Path};

mod langchain;
pub use langchain::*;

use crate::{Mcp, languages::Language};

custom_derive! {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, clap::ValueEnum)]
    #[derive(IterVariants(Frameworks))]
    pub enum Framework {
        // Python
        Langchain,
        // Rust
        CoralRs
    }
}

impl Framework {
    pub fn name(&self) -> &str {
        match self {
            Framework::Langchain => "Langchain",
            Framework::CoralRs => "coral-rs",
        }
    }
    pub fn language(&self) -> Language {
        match self {
            Framework::Langchain => Language::Python,
            Framework::CoralRs => Language::Rust,
        }
    }
}

impl Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name(), self.language())
    }
}

pub trait Template {
    fn name() -> &'static str;
    fn artifact() -> (&'static str, &'static str);

    fn include_file(entry: &ignore::DirEntry) -> bool {
        let _ = entry;
        true
    }
    fn is_templated_file(path: &Path) -> bool {
        let _ = path;
        true
    }
    fn template(mcps: &[Mcp], contents: &str) -> String;
    fn post_process(root: &Path, agent_name: &str) -> std::io::Result<()>;
}
