mod cli;
mod entry;
mod entry_cache;
mod config;
mod modes;
mod formatter;

use entry::*;
use rustc_hash::FxHashMap;
use std::{io::IsTerminal, path::{Path, PathBuf}, sync::LazyLock};
use clap::Parser;
use anyhow::Result;
use configparser::ini::Ini as IniParser;
use crate::{config::Config, entry_cache::get_cache, formatter::Formatter};
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

pub fn get_fallback_icon(entry_type: &EntryType) -> &'static str {
    match entry_type {
        EntryType::Application => "application-x-executable",
        EntryType::Link => "open-link",
        EntryType::Other => "",
    }
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

// TODO println! can fail when stdout closes with broken pipe erorr
fn main() -> anyhow::Result<()> {
    let mut cli_args = cli::Cli::parse();

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
            let mut cache = get_cache(cli_args.cache.as_deref(), &config)?;

            if let Some(tag) = &cli_args.sort_tag {
                cache.entries.sort_by(|b, a| {
                    a.tags.contains(tag)
                        .cmp(&b.tags.contains(tag))
                        .then_with(|| a.name.cmp(&b.name))
                });
            }

            // filter only tags
            let entries = if let Some(tag) = &cli_args.only_tag {
                if let Some(entries) = cache.by_tag.get(tag) {
                    entries
                } else {
                    eprintln!("No entries found with tag {:?}", tag);
                    return Ok(());
                }
            } else {
                &cache.entries
            };

            for entry in entries {
                println!("{}", args.format.format_entry(entry)?);
            }
        },
        cli::CliCommands::Query(args) => {
            let config = get_config()?;
            let cache = get_cache(cli_args.cache.as_deref(), &config)?;

            for id in &args.ids {
                if let Some(entry) = cache.by_id.get(id) {
                    println!("{}", args.format.format_entry(entry)?);
                }
            }
        },
        cli::CliCommands::None => unreachable!(),
    }

    Ok(())
}
