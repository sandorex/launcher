use std::path::PathBuf;
use clap::{Args, Parser, Subcommand};

/// XDG Desktop compliant indexer
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    /// Do not use cache
    #[arg(long = "no-cache", action = clap::ArgAction::SetFalse, env = "SDLT_NO_CACHE", default_value_t = true, global = true)]
    pub cache: bool,

    /// Where to store cache of desktop entries
    #[arg(long, env = "SDLT_CACHE", default_value = "~/.cache/sdlt", global = true)]
    pub cache_file: PathBuf,

    /// Path to file where favorites are stored (JSON array)
    #[arg(long, env = "SDLT_CONFIG", default_value = "~/.config/sdlt.json", global = true)]
    pub config_file: PathBuf,

    #[command(subcommand)]
    pub cmd: CliCommands,
}

#[derive(Args, Debug, Clone)]
pub struct CmdRofi {
    /// Contains code from rofi when running in script mode
    #[arg(long, env = "ROFI_RETV", hide = true)]
    pub rofi_status: Option<u8>,

    /// Filter the entries based on their properties using a DSL
    ///
    /// For syntax help read: https://docs.rs/filt-rs/
    #[arg(short, long)]
    pub query: Option<String>,

    // this is provided by rofi
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = false, hide = true)]
    pub rest: Vec<String>,
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
    #[arg(short, long, default_value = "json")]
    pub format: OutputFormat,

    // TODO this could be a struct that is flattened everywhere so its not repeated
    /// Filter the entries based on their properties using a DSL
    ///
    /// For syntax help read: https://docs.rs/filt-rs/
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = false)]
    pub query: Vec<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    /// Used in rofi script mode
    ///
    /// Automatically called when called by rofi
    Rofi(CmdRofi),

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
