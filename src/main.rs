mod cli;
mod entry;
mod entry_cache;
mod config;

use entry::*;
use rustc_hash::FxHashMap;
use std::{io::IsTerminal, path::{Path, PathBuf}, sync::LazyLock};
use clap::Parser;
use anyhow::{Result, anyhow};
use configparser::ini::Ini as IniParser;
use crate::{cli::CmdRofi, config::Config, entry_cache::EntryDB};

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

fn find_entries(config: &Config) -> Result<Vec<Entry>> {
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

    // split entries that have actions so each action is its own entry
    Ok(entries.into_values().collect())
}

/// Gets the cached entries if they are recent enough otherwise find them
fn get_entries(cache_path: Option<&Path>, config: &Config) -> Result<EntryDB> {
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

// TODO
// 2. benchmark
// 3. actually useable formatter for rofi script mode
// 4. cli for testing with filters n shit
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

    // TODO make cli_args.cache_file an Option<..> so it can be removed when disabled
    // expand home (tilde) in paths
    cli_args.cache_file = expand_tilde(cli_args.cache_file);
    cli_args.config_file = expand_tilde(cli_args.config_file);

    let cache_file = if cli_args.cache { Some(cli_args.cache_file.as_path()) } else { None };
    let get_config = || Config::read(&cli_args.config_file);

    match cmd {
        cli::CliCommands::Rofi(args) => rofi(&cli_args, args, get_config()?)?,
        cli::CliCommands::List(args) => {
            // let query = args.query.join(" ");
            // let query = query.trim();

            // list actions as separate entries
            let config = get_config()?;
            let entries = get_entries(cache_file, &config)?;
            let entries = &entries.entries;

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

            // // if there is no query just print
            // if query.is_empty() {
                for entry in entries {
                    custom_print(&entry)?;
                }
            // } else {
            //     let filter = Filter::new(query)
            //         .with_context(|| anyhow!("invalid query {:?}", query))?;
            //
            //     for entry in &entries {
            //         if args.query.len() > 0 && filter.matches(entry)? {
            //             custom_print(&entry)?;
            //         }
            //     }
            // }
        },
        cli::CliCommands::None => unreachable!(),
    }

    Ok(())
}

fn rofi(cli_args: &cli::Cli, args: CmdRofi, config: Config) -> Result<()> {
    if args.rofi_status.is_none() {
        eprintln!("Error not running in rofi script mode\n\nRead more with `man 5 rofi-script`");
        return Ok(());
    }

    let cache_file = if cli_args.cache { Some(cli_args.cache_file.as_path()) } else { None };

    match args.rofi_status.unwrap() {
        // initial call
        0 => {
            // disable custom input
            println!("\0no-custom\x1ftrue");

            let cache = get_entries(cache_file, &config)?;

            for entry in &cache.entries {
                // TODO generic placeholder icon if its missing?
                // TODO only print options if there is any data
                println!(
                    "{name}\0icon\x1f{icon}\x1fmeta\x1f{meta}\x1finfo\x1f{info}",
                    name = entry.name,
                    icon = entry.icon.as_ref().map(|x| x.as_str()).unwrap_or(""),
                    meta = format!("{} {} {}",
                        entry.generic_name.as_ref().map(|x| x.as_str()).unwrap_or(""),
                        entry.comment.as_ref().map(|x| x.as_str()).unwrap_or(""),
                        // TODO i just added all the info but do categories fit here at all?
                        entry.categories.join(" "),
                    ),
                    info = entry.id,
                );
            }
        },

        // TODO calling the actuall application with wrappers like systemd-cat or swaymsg
        // selected an entry
        1 => {
            let info = std::env::var("ROFI_INFO").unwrap();
            let cache = get_entries(cache_file, &config)?;

            if let Some(entry) = cache.by_id.get(&info) {
                println!("got entry {:?}, cmd {:?}", entry.id, entry.exec);
            }
        },
        // 2          selected a custom entry
        // 3          deleted an entry
        // 10 - 28    custom keybindings
        val => {
            return Err(anyhow!("invalid ROFI_RETV value {val:?}"));
        },
    };

    Ok(())
}
