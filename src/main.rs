mod cli;
mod entry;
mod entry_cache;
mod config;

use entry::*;
use std::{collections::HashMap, io::IsTerminal, path::{Path, PathBuf}, sync::LazyLock, time::Duration};
use clap::Parser;
use anyhow::{Context, Result, anyhow};
use configparser::ini::Ini as IniParser;
use filt_rs::Filter;

use crate::config::Config;

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

// TODO should TryExec be tested here?
fn find_entries(config: &Config, split_actions: bool) -> Result<Vec<Entry>> {
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
    let mut entries: HashMap<String, Entry> = HashMap::new();

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

    // split entries that have actions so each action is its own entry
    Ok(if split_actions {
        entries
            .into_values()
            .flat_map(|x| x.split_actions().into_iter())
            .collect()
    } else {
        entries
            .into_values()
            .collect()
    })
}

/// Gets the cached entries if they are recent enough otherwise find them
fn get_entries(cache_path: Option<&Path>, config: &Config, split_actions: bool) -> Result<Vec<Entry>> {
    use entry_cache::EntryCache;

    if let Some(path) = cache_path && let Ok(db) = EntryCache::read(path) {
        // TODO what should be the time before a caching is needed?
        // if the timestamp is not older than 2 hours then just use it
        if db.timestamp.elapsed().ok().and_then(|x| Some(x < Duration::from_hours(2))).unwrap_or(true) {
            return Ok(db.entries);
        }

        let entries = find_entries(config, split_actions)?;

        // save entries in database
        EntryCache::update(path, entries.clone())?;

        Ok(entries)
    } else {
        Ok(find_entries(config, split_actions)?)
    }
}

fn main() -> anyhow::Result<()> {
    // automatically run rofi subcommand if ran in rofi script mode
    let mut cli_args = if std::env::var("ROFI_RETV").is_ok() {
        let mut args: Vec<String> = std::env::args().collect();

        // set the command
        args.insert(1, "rofi".to_string());

        cli::Cli::parse_from(args)
    } else {
        cli::Cli::parse()
    };

    let cmd = std::mem::replace(&mut cli_args.cmd, cli::CliCommands::None);

    // expand home (tilde) in paths
    cli_args.cache_file = expand_tilde(cli_args.cache_file);
    cli_args.config_file = expand_tilde(cli_args.config_file);

    let cache_file = if cli_args.cache { Some(cli_args.cache_file.as_path()) } else { None };

    let get_config = || Config::read(&cli_args.config_file);

    match cmd {
        cli::CliCommands::Rofi(args) => {
            if args.rofi_status.is_none() {
                eprintln!("Error not running in rofi script mode\n\nRead more with `man 5 rofi-script`");
                return Ok(());
            }

            match args.rofi_status.unwrap() {
                // initial call
                0 => {
                    // disable custom input
                    println!("\0no-custom\x1ftrue");

                    let config = get_config()?;
                    let entries = get_entries(cache_file, &config, false)?;

                    // TODO filter with the query if set
                    // TODO maybe use custom keybindings to trigger application actions?
                    for entry in &entries {
                        // TODO generic placeholder icon if its missing?
                        // TODO only print options if there is any data
                        println!(
                            "{}\0icon\x1f{}\x1fmeta\x1f{}",
                            entry.name,
                            entry.icon.as_ref().map(|x| x.as_str()).unwrap_or(""),

                            // TODO i just added all the info but do categories fit here at all?
                            format!("{} {} {}",
                                entry.generic_name.as_ref().map(|x| x.as_str()).unwrap_or(""),
                                entry.comment.as_ref().map(|x| x.as_str()).unwrap_or(""),
                                entry.categories.join(" "),
                            ),
                        );
                    }
                },

                // TODO calling the actuall application with wrappers like systemd-cat or swaymsg
                // selected an entry
                1 => {
                    println!("got {:#?}", args.rest);
                },
                // 2          selected a custom entry
                // 3          deleted an entry
                // 10 - 28    custom keybindings
                val => {
                    return Err(anyhow!("invalid ROFI_RETV value {val:?}"));
                },
            };
        },
        cli::CliCommands::List(args) => {
            let query = args.query.join(" ");
            let query = query.trim();

            // list actions as separate entries
            let config = get_config()?;
            let entries = get_entries(cache_file, &config, true)?;

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

            // if there is no query just print
            if query.is_empty() {
                for entry in &entries {
                    custom_print(&entry)?;
                }
            } else {
                let filter = Filter::new(query)
                    .with_context(|| anyhow!("invalid query {:?}", query))?;

                for entry in &entries {
                    if args.query.len() > 0 && filter.matches(entry)? {
                        custom_print(&entry)?;
                    }
                }
            }

        },
        cli::CliCommands::None => unreachable!(),
    }

    Ok(())
}
