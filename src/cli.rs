use std::path::PathBuf;
use clap::{Args, Parser, Subcommand};

/// XDG Desktop compliant indexer
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    /// Where to store cache of desktop entries
    #[arg(long, env = "SDLT_CACHE", default_value = Some("~/.cache/sdlt"), global = true)]
    pub cache: Option<PathBuf>,

    /// Path to config
    #[arg(long, env = "SDLT_CONFIG", default_value = "~/.config/sdlt.json", global = true)]
    pub config: PathBuf,

    #[command(subcommand)]
    pub cmd: CliCommands,
}

#[derive(Args, Debug, Clone)]
pub struct CmdRofi {
    /// Contains code from rofi when running in script mode
    #[arg(long, env = "ROFI_RETV", hide = true)]
    pub rofi_status: u8,

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

#[derive(Args, Debug, Clone)]
pub struct CmdList {
    #[arg(short, long, default_value = "json")]
    pub format: OutputFormat,

    // // TODO this could be a struct that is flattened everywhere so its not repeated
    // /// Filter the entries based on their properties using a DSL
    // ///
    // /// For syntax help read: https://docs.rs/filt-rs/
    // #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = false)]
    // pub query: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct CmdQuery {
    #[arg(short, long, default_value = "json")]
    pub format: OutputFormat,

    /// Ids of the applications
    pub ids: Vec<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    /// Used in rofi script mode
    ///
    /// Automatically called when called by rofi
    Rofi(CmdRofi),

    /// Query entries
    List(CmdList),

    /// Query for specific application using the id
    Query(CmdQuery),

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
