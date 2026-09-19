mod cli;
mod entry;
mod entry_cache;
mod config;
mod modes;

use entry::*;
use rustc_hash::FxHashMap;
use std::{io::IsTerminal, path::{Path, PathBuf}, process::Command, sync::LazyLock};
use clap::Parser;
use anyhow::{Context, Result, anyhow};
use configparser::ini::Ini as IniParser;
use crate::{config::Config, entry_cache::EntryDB};
use modes::rofi;

/// Expands paths that start with `~/`
fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    static HOME: LazyLock<PathBuf> = LazyLock::new(|| {
        PathBuf::from(std::env::var("HOME")
            .expect("cannot get HOME env var"))
    });

    let p = path.as_ref();
    if let Ok(path) = path.as_ref().strip_prefix("~") {
        return HOME.join(path);
    }

    p.to_path_buf()
}

pub fn find_entries(config: &Config) -> Result<Vec<Entry>> {
    const DEFAULT_PATHS: &[&str] = &[
        "/usr/share/applications",
        "/usr/local/share/applications",
        "/var/lib/flatpak/exports/share/applications",
        "/var/lib/snapd/desktop/applications",
        "~/.local/share/flatpak/exports/share/applications",
        "~/.local/share/applications",
    ];

    let mut parser = IniParser::new();

    // each entry has to have unique id so it can be overriden
    let mut entries: FxHashMap<String, Entry> = Default::default();

    // im gathering errors here so i can conditionally deal with them
    let mut errors: Vec<String> = vec![];

    let mut collect = |root: &Path| {
        for entry in root.read_dir().unwrap().flatten() {
            if let Ok(entry_type) = entry.file_type() && (entry_type.is_file() || entry_type.is_symlink()) {
                // filter only desktop files
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if file_name.ends_with(".desktop") {
                    // parse the file
                    match parser.load(&entry.path()) {
                        Ok(_) => {},
                        Err(err) => errors.push(format!("Failed to parse ini {:?}: {err}", &entry.path())),
                    }

                    // TODO langauge
                    let entry_id = file_name.strip_suffix(".desktop").unwrap();
                    let entry = match Entry::from_parser(&parser, entry_id, None, config) {
                        Ok(None) => continue, // entry that is not invalid but should be skipped
                        Ok(Some(x)) => x,
                        Err(err) => {
                            errors.push(format!("Failed to parse entry in {:?}: {err}", &entry.path()));
                            continue;
                        }
                    };

                    // override any value if the id is the same
                    entries.insert(entry_id.to_string(), entry);
                }

                // ignore directories and other special types
            }
        }
    };

    for path in DEFAULT_PATHS {
        let path = expand_tilde(PathBuf::from(path));

        if path.try_exists().unwrap_or(false) {
            collect(&path);
        }
    }

    // support dynamic directories from the env var
    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for path in xdg_data_dirs.split(':') {
            let path = expand_tilde(PathBuf::from(path).join("applications"));

            if path.try_exists().unwrap_or(false) {
                collect(&path);
            }
        }
    }

    // TODO how to deal with the errors, just always print them?
    if !errors.is_empty() {
        eprintln!("Found {} errors while parsing desktop files:", errors.len());
        for err in &errors {
            eprintln!("  {err}");
        }
    }

    Ok(entries.into_values().collect())
}

// TODO respect entry path where to start in
/// Gets the cached entries if they are recent enough otherwise find them
fn get_cache(cache_path: Option<&Path>, config: &Config) -> Result<EntryDB> {
    use entry_cache::EntryDB;

    if let Some(path) = cache_path {
        if let Ok(db) = EntryDB::from_file(path) && !db.is_old() {
            return Ok(db);
        } else {
            let entries = find_entries(config)?;
            let cache = EntryDB::from_entries(entries);

            cache.save(path)?;

            Ok(cache)
        }
    } else {
        Ok(EntryDB::from_entries(find_entries(config)?))
    }
}

pub fn execute_entry(cli_args: &cli::Cli, entry: &Entry) -> Result<()> {
    match entry.entry_type {
        EntryType::Application if entry.terminal => {
            let exec = entry
                .exec
                .as_ref()
                .with_context(|| anyhow!("Tried to execute invalid application entry with empty Exec"))?;

            let mut cmd = Command::new(&cli_args.exec_term[0]);

            if let Some(path) = &entry.path {
                cmd.current_dir(path);
            }

            cmd.args(cli_args.exec_term.iter().skip(1).map(|x| x.replace("%command%", exec)));
            cmd.status()
                .with_context(|| anyhow!("could not execute {:?}", cli_args.exec_term[0]))?;
        },
        EntryType::Application => {
            let exec = entry
                .exec
                .as_ref()
                .with_context(|| anyhow!("Tried to execute invalid application entry with empty Exec"))?;

            let mut cmd = Command::new(&cli_args.exec_app[0]);

            if let Some(path) = &entry.path {
                cmd.current_dir(path);
            }

            cmd.args(cli_args.exec_app.iter().skip(1).map(|x| x.replace("%command%", exec)));
            cmd.status()
                .with_context(|| anyhow!("could not execute {:?}", cli_args.exec_app[0]))?;
        },
        EntryType::Link => {
            let url = entry
                .url
                .as_ref()
                .with_context(|| anyhow!("Tried to open invalid link entry with empty URL"))?;

            Command::new(&cli_args.exec_link[0])
                .args(cli_args.exec_link.iter().skip(1).map(|x| x.replace("%url%", url)))
                .status()
                .with_context(|| anyhow!("could not execute {:?}", cli_args.exec_link[0]))?;
        },
        EntryType::Other => {},
    }

    Ok(())
}

pub fn execute_action(cli_args: &cli::Cli, entry: &Entry, action: &EntryAction) -> Result<()> {
    let mut cmd = Command::new(&cli_args.exec_term[0]);

    if let Some(path) = &entry.path {
        cmd.current_dir(path);
    }

    cmd.args(cli_args.exec_term.iter().skip(1).map(|x| x.replace("%command%", &action.exec)));
    cmd.status()?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // automatically run rofi subcommand if ran in rofi script mode
    let mut cli_args = if std::env::var("ROFI_RETV").is_ok() {
        let mut args: Vec<String> = std::env::args().collect();

        // set the command if not present already
        if args.get(1).map(|x| x.as_str()) != Some("rofi") {
            args.insert(1, "rofi".to_string());
        }

        cli::Cli::parse_from(args)
    } else {
        cli::Cli::parse()
    };

    let cmd = std::mem::replace(&mut cli_args.cmd, cli::CliCommands::None);

    // expand home (tilde) in paths
    cli_args.cache = cli_args.cache.map(|x| expand_tilde(x).to_path_buf());
    cli_args.config = expand_tilde(cli_args.config);

    let get_config = || Config::read(&cli_args.config);

    match cmd {
        cli::CliCommands::Rofi(args) => rofi(&cli_args, args, get_config()?)?,
        cli::CliCommands::List(args) => {
            // list actions as separate entries
            let config = get_config()?;
            let entries = get_cache(cli_args.cache.as_deref(), &config)?;
            let entries = &entries.entries;

            // TODO this could be a lazylock
            // detect if output is interactive to output pretty formatted data
            let is_terminal = std::io::stdout().is_terminal();

            let custom_print = |entry: &Entry| -> Result<()> {
                match args.format {
                    cli::OutputFormat::Debug => {
                        if is_terminal {
                            println!("{entry:#?}");
                        } else {
                            println!("{entry:?}");
                        }
                    },
                    cli::OutputFormat::JSON => {
                        if is_terminal {
                            println!("{}", serde_json::to_string_pretty(entry)?);
                        } else {
                            println!("{}", serde_json::to_string(entry)?);
                        }
                    }
                }

                Ok(())
            };

            for entry in entries {
                custom_print(&entry)?;
            }
        },
        cli::CliCommands::Query(args) => {
            // TODO this is literally duplicate of list
            // detect if output is interactive to output pretty formatted data
            let is_terminal = std::io::stdout().is_terminal();

            let custom_print = |entry: &Entry| -> Result<()> {
                match args.format {
                    cli::OutputFormat::Debug => {
                        if is_terminal {
                            println!("{entry:#?}");
                        } else {
                            println!("{entry:?}");
                        }
                    },
                    cli::OutputFormat::JSON => {
                        if is_terminal {
                            println!("{}", serde_json::to_string_pretty(entry)?);
                        } else {
                            println!("{}", serde_json::to_string(entry)?);
                        }
                    }
                }

                Ok(())
            };

            let config = get_config()?;
            let cache = get_cache(cli_args.cache.as_deref(), &config)?;

            for id in &args.ids {
                if let Some(entry) = cache.by_id.get(id) {
                    custom_print(&entry)?;
                }
            }
        },
        cli::CliCommands::None => unreachable!(),
    }

    Ok(())
}
