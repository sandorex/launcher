mod cli;
mod entry;

use entry::*;
use std::{collections::HashMap, path::{Path, PathBuf}};
use clap::Parser;
use anyhow::{Context, Result, anyhow, bail};
use configparser::ini::Ini as IniParser;
use filt_rs::Filter;

const SECTION: &str = "Desktop Entry";
const SECTION_ACTION: &str = "Desktop Action";

pub fn entry_from_parser(parser: &IniParser, id: &str, lang: Option<&str>) -> Result<Option<Entry>, String> {
    fn get_lang(parser: &IniParser, section: &str, key: &str, lang: &str) -> Option<String> {
        parser.get(section, &format!("{key}{lang}"))
            .or(parser.get(section, "{key}"))
    }

    // append language marker
    let lang = if let Some(lang) = lang {
        format!("[{}]", lang)
    } else {
        "".to_string()
    };

    let entry_type: EntryType = parser.get(SECTION, "Type")
        .ok_or_else(|| "Type is required in desktop files".to_string())?
        .parse()
        .unwrap(); // EntryType parse cannot fail

    // ignore other types
    if entry_type == EntryType::Other {
        return Ok(None);
    }

    let mut actions: Vec<Entry> = vec![];
    if let Some(action_names) = parser.get(SECTION, "Actions") {
        for name in action_names.split(';') {
            let section = format!("{SECTION_ACTION} {}", name);
            actions.push(Entry {
                id: id.to_string(),
                name: get_lang(parser, &section, "Name", &lang)
                        .ok_or_else(|| "Name is required in actions".to_string())?,
                exec: Some(parser.get(&section, "Exec").ok_or_else(|| "Exec is required in actions".to_string())?),
                icon: parser.get(&section, "Icon"),

                ..Default::default()
            });
        }
    }

    Ok(Some(Entry {
        id: id.to_string(),
        entry_type,
        name: get_lang(parser, SECTION, "Name", &lang).ok_or_else(|| "Name is required in desktop files".to_string())?,
        exec: parser.get(SECTION, "Exec"),
        url: parser.get(SECTION, "URL"),
        generic_name: get_lang(parser, SECTION, "GenericName", &lang),
        comment: get_lang(parser, SECTION, "Comment", &lang),
        terminal: parser.getbool(SECTION, "Terminal").ok().flatten().unwrap_or(false),
        no_display: parser.getbool(SECTION, "NoDisplay").ok().flatten().unwrap_or(false),
        icon: parser.get(SECTION, "Icon"),
        only_show_in: parser.get(SECTION, "OnlyShowIn"),
        categories: parser
            .get(SECTION, "Categories")
            .map(|x| x.split(';')
                        .map(|y| y.to_string())
                        .collect())
            .unwrap_or(vec![]),
        actions,
    }))
}

fn find_entries() -> Result<Vec<Entry>> {
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
            // TODO it is not checked whether the symlink points to a file!
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

                    let entry_id = file_name.strip_suffix(".desktop").unwrap();
                    let entry = match entry_from_parser(&parser, entry_id, None) {
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
        let path = PathBuf::from(path);

        if path.try_exists().unwrap_or(false) {
            collect(&path);
        }
    }

    // support dynamic directories from the env var
    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for path in xdg_data_dirs.split(':') {
            let path = PathBuf::from(path).join("applications");

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
    let entries: Vec<Entry> = entries
        .into_values()
        .flat_map(|x| x.split_actions().into_iter())
        .collect();

    Ok(entries)
}

fn main() -> anyhow::Result<()> {
    let mut args = cli::Cli::parse();
    let cmd = std::mem::replace(&mut args.cmd, cli::CliCommands::None);

    match cmd {
        cli::CliCommands::List => todo!(),
        cli::CliCommands::Test => {
            // let e = Entry::default();
            let entries = find_entries()?;

            let filter = Filter::new("entry.show_in == \"Link\"")?;

            for entry in &entries {
                if filter.matches(entry)? {
                    println!("{entry:#?}");
                }
            }
        },
        cli::CliCommands::None => unreachable!(),
    }

    Ok(())
}
