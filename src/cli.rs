use clap::{Args, Parser, Subcommand};

/// XDG Desktop compliant indexer
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    /// Do not use cache
    #[arg(long = "no-cache", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub cache: bool,

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

#[derive(Args, Debug, Clone)]
pub struct CmdQuery {
    #[arg(short, long, default_value = "debug")]
    pub format: OutputFormat,

    /// Query
    ///
    /// For syntax help read: https://docs.rs/filt-rs/
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub query: Vec<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    // TODO make it the default command so its easier to use but detect env var so stdout is not
    // filled with stuff
    // Rofi,

    // TODO add option for query with rest=true
    List,

    /// Query entries
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
