use std::{fmt::Display, hash::Hash, process::Command, sync::LazyLock};
use code_docs::{code_docs_struct, DocumentedStruct};
use configparser::ini::Ini as IniParser;
use serde::{Deserialize, Serialize};
use crate::config::Config;
use anyhow::{Context, Result, anyhow};
use std::rc::Rc;
use rustc_hash::FxHashSet;

const SECTION: &str = "Desktop Entry";
const SECTION_ACTION: &str = "Desktop Action";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum EntryType {
    /// Application desktop file
    Application,

    /// Link desktop file that opens an URL
    Link,

    /// Other entry types, just ignored
    Other,
}

impl Default for EntryType {
    fn default() -> Self {
        Self::Application
    }
}

impl Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match &self {
            Self::Application => "Application",
            Self::Link => "Link",
            Self::Other => "Other",
        })
    }
}

impl From<&str> for EntryType {
    fn from(value: &str) -> Self {
        match value {
            "Application" => Self::Application,
            "Link" => Self::Link,
            _ => Self::Other
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct EntryAction {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
}

// NOTE this can be either a link or application
code_docs_struct! {
    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct Entry {
        pub id: String,
        pub entry_type: EntryType,
        pub name: String,

        /// Should be generic name like "Web Browser" but some applications write whole paragraphs in it
        /// so its not as useful
        #[serde(skip_serializing_if = "Option::is_none")]
        pub generic_name: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub comment: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub exec: Option<String>,

        /// Even though its called try_exec, its actually used to check if executable is installed by
        /// looking in the path for it...
        #[serde(skip_serializing_if = "Option::is_none")]
        pub try_exec: Option<String>,

        /// Working directory where the script will be executed
        #[serde(skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,

        /// Should only be available when Type=Link
        #[serde(skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,

        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub categories: FxHashSet<String>,

        /// Does this entry run in terminal
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        pub terminal: bool,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub icon: Option<String>,

        /// Do not show this entry except in these Desktop Environments
        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub only_show_in: FxHashSet<String>,

        /// Actions of the entry
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub actions: Vec<Rc<EntryAction>>,

        /// User applied tags
        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub tags: FxHashSet<String>,
    }
}

impl Entry {
    fn clean_exec(exec: &String) -> String {
        static PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new("%[a-zA-Z]")
                .expect("invalid exec clean regex")
        });

        PATTERN
            .replace_all(exec, "")
            // xdg escape a percent sign..
            .replace("%%", "%")
    }

    pub fn from_parser(parser: &IniParser, id: &str, lang: Option<&str>, config: &Config) -> Result<Option<Self>, String> {
        let get_lang = if lang.is_some_and(|x| !x.is_empty()) {
            // check for each key twice only if there is a language set
            |parser: &IniParser, section: &str, key: &str, lang: &str| -> Option<String> {
                parser.get(section, &format!("{key}{lang}"))
                    .or(parser.get(section, "{key}"))
            }
        } else {
            |parser: &IniParser, section: &str, key: &str, _lang: &str| -> Option<String> {
                parser.get(section, key)
            }
        };

        // skip hidden entries
        if let Ok(no_display) = parser.getbool(SECTION, "NoDisplay") && no_display.unwrap_or(false) {
            return Ok(None);
        }

        // append language marker
        let lang = if let Some(lang) = lang {
            format!("[{}]", lang)
        } else {
            "".to_string()
        };

        let entry_type: EntryType = Into::<EntryType>::into(
            parser.get(SECTION, "Type")
                  .ok_or_else(|| "Type is required in desktop files".to_string())?
                  .as_str()
        );

        // ignore other types
        if entry_type == EntryType::Other {
            return Ok(None);
        }

        let mut actions: Vec<Rc<EntryAction>> = vec![];
        if let Some(action_names) = parser.get(SECTION, "Actions") {
            for name in action_names.split(';') {
                let section = format!("{SECTION_ACTION} {}", name);
                actions.push(Rc::new(EntryAction {
                    name: get_lang(parser, &section, "Name", &lang)
                            .ok_or_else(|| "Name is required in actions".to_string())?,
                    exec: Self::clean_exec(&parser.get(&section, "Exec").ok_or_else(|| "Exec is required in actions".to_string())?),
                    icon: parser.get(&section, "Icon"),
                }));
            }
        }

        Ok(Some(Self {
            id: id.to_string(),
            entry_type,
            name: get_lang(parser, SECTION, "Name", &lang).ok_or_else(|| "Name is required in desktop files".to_string())?,
            exec: parser.get(SECTION, "Exec").map(|x| Self::clean_exec(&x)),
            try_exec: parser.get(SECTION, "TryExec"),
            path: parser.get(SECTION, "Path"),
            url: parser.get(SECTION, "URL"),
            generic_name: get_lang(parser, SECTION, "GenericName", &lang),
            comment: get_lang(parser, SECTION, "Comment", &lang),
            terminal: parser.getbool(SECTION, "Terminal").ok().flatten().unwrap_or(false),
            icon: parser.get(SECTION, "Icon"),
            only_show_in: parser
                .get(SECTION, "OnlyShowIn")
                .map(|x| x.split(';')
                            .map(|y| y.to_string())
                            .collect())
                .unwrap_or_default(),
            categories: parser
                .get(SECTION, "Categories")
                .map(|x| x.split(';')
                            .map(|y| y.to_string())
                            .collect())
                .unwrap_or_default(),
            tags: config
                .tags
                .get(id)
                .cloned()
                .unwrap_or_default(),
            actions,
        }))
    }
}

pub fn execute_entry(cli_args: &crate::cli::Cli, entry: &Entry) -> Result<()> {
    use std::process::Stdio;

    match entry.entry_type {
        EntryType::Application if entry.terminal => {
            let exec = entry
                .exec
                .as_ref()
                .with_context(|| anyhow!("Tried to execute invalid application entry with empty Exec"))?;

            let mut cmd = Command::new(&cli_args.exec_term[0]);

            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            cmd.stdin(Stdio::null());

            if let Some(path) = &entry.path {
                cmd.current_dir(path);
            }

            cmd.args(cli_args.exec_term.iter().skip(1).map(|x| x.replace("%command%", exec)));
            cmd.spawn()
                .with_context(|| anyhow!("could not execute {:?}", cli_args.exec_term[0]))?;
        },
        EntryType::Application => {
            let exec = entry
                .exec
                .as_ref()
                .with_context(|| anyhow!("Tried to execute invalid application entry with empty Exec"))?;

            let mut cmd = Command::new(&cli_args.exec_app[0]);

            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            cmd.stdin(Stdio::null());

            if let Some(path) = &entry.path {
                cmd.current_dir(path);
            }

            cmd.args(cli_args.exec_app.iter().skip(1).map(|x| x.replace("%command%", exec)));
            cmd.spawn()
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

pub fn execute_action(cli_args: &crate::cli::Cli, entry: &Entry, action: &EntryAction) -> Result<()> {
    let mut cmd = Command::new(&cli_args.exec_term[0]);

    if let Some(path) = &entry.path {
        cmd.current_dir(path);
    }

    cmd.args(cli_args.exec_term.iter().skip(1).map(|x| x.replace("%command%", &action.exec)));
    cmd.status()?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::Entry;

    #[test]
    fn desktop_file_placeholders() {
        assert_eq!(
            Entry::clean_exec(&"/nix/store/ix23cwqfjdap6wyhx27lz189kv5hsw3x-vivaldi-8.1.4087.70/bin/vivaldi %U".to_string()),
            "/nix/store/ix23cwqfjdap6wyhx27lz189kv5hsw3x-vivaldi-8.1.4087.70/bin/vivaldi ".to_string()
        );
    }
}
