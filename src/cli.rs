use std::path::PathBuf;
use clap::{Args, Parser, Subcommand};

const CONFIG: &str = concat!("~/.config/", env!("CARGO_PKG_NAME"), ".json");
const CACHE: &str = concat!("~/.cache/", env!("CARGO_PKG_NAME"));

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    /// Where to store cache of desktop entries
    #[arg(long, env = concat!(env!("CARGO_PKG_NAME_UPPERCASE"), "_CACHE"), default_value = Some(CACHE))]
    pub cache: Option<PathBuf>,

    /// Path to config
    #[arg(long, env = concat!(env!("CARGO_PKG_NAME_UPPERCASE"), "_CONFIG"), default_value = CONFIG)]
    pub config: PathBuf,

    /// Command to start terminal applications, `%command%` is replaced by the command
    ///
    /// Use `--` to end the command
    #[arg(
        short = 'a',
        long = "app",
        env = concat!(env!("CARGO_PKG_NAME_UPPERCASE"), "_APP"),
        default_values = vec!["systemd-run", "--user", concat!("--slice=", env!("CARGO_PKG_NAME")), "sh", "-c", "%command%"],
        num_args = 1..,
        value_terminator = "--",
        allow_hyphen_values = true,
    )]
    pub exec_app: Vec<String>,

    /// Command to start terminal applications, `%command%` is replaced by the command
    ///
    /// Use `--` to end the command
    #[arg(
        short = 't',
        long = "term",
        env = concat!(env!("CARGO_PKG_NAME_UPPERCASE"), "_TERM"),
        default_values = vec!["systemd-run", "--user", concat!("--slice=", env!("CARGO_PKG_NAME")), "kitty", "-e", "sh", "-c", "%command%"],
        num_args = 1..,
        value_terminator = "--",
        allow_hyphen_values = true,
    )]
    pub exec_term: Vec<String>,

    /// Command to start links, `%url%` is replaced by the url
    ///
    /// Use `--` to end the command
    #[arg(
        short = 'l',
        long = "link",
        env = concat!(env!("CARGO_PKG_NAME_UPPERCASE"), "_LINK"),
        default_values = vec!["xdg-open", "%url%"], num_args = 1..,
        value_terminator = "--",
        allow_hyphen_values = true,
    )]
    pub exec_link: Vec<String>,

    /// Sort entries so tagged ones are at top
    #[arg(long)]
    pub sort_tag: Option<String>,

    /// Only show entries that are tagged
    #[arg(long, conflicts_with = "sort_tag")]
    pub only_tag: Option<String>,

    #[command(subcommand)]
    pub cmd: CliCommands,
}

#[derive(Args, Debug, Clone)]
pub struct CmdRofi {
    /// Contains code from rofi when running in script mode
    #[arg(long, env = "ROFI_RETV", hide = true)]
    pub rofi_status: u8,

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
    /// Used in rofi script mode (for more info `man 5 rofi-script`)
    ///
    /// Automatically called when called by rofi
    Rofi(CmdRofi),

    /// List all entries
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
