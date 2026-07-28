use std::path::PathBuf;
use clap::{Args, Parser, Subcommand};

/// XDG Desktop compliant indexer
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    /// Do not use cache
    #[arg(long = "no-cache", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub cache: bool,

    /// Where to store cache of desktop entries
    #[arg(long, default_value = "~/.cache/sdlt.json")]
    pub cache_path: PathBuf,

    /// Path to file where favorites are stored (JSON array)
    #[arg(long = "favorites", default_value = "~/.config/sdlt-favorites.json")]
    pub favorites: PathBuf,

    #[command(subcommand)]
    pub cmd: CliCommands,
}

/// Output format of data
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    /// Rust debug format
    Debug,
    JSON,
}

// TODO add examples for queries
#[derive(Args, Debug, Clone)]
pub struct CmdList {
    #[arg(short, long, default_value = "debug")]
    pub format: OutputFormat,

    /// Filter the entries based on their properties using a DSL
    ///
    /// For syntax help read: https://docs.rs/filt-rs/
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = false)]
    pub query: Vec<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    /// Meant to be used in rofi script mode
    Rofi,

    /// Query entries
    List(CmdList),

    #[clap(skip)]
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
