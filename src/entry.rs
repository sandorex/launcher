use std::{fmt::Display, hash::Hash, str::FromStr};
use configparser::ini::Ini as IniParser;
use filt_rs::{FilterValue, Filterable};
use serde::{Deserialize, Serialize};

use crate::config::Config;

const SECTION: &str = "Desktop Entry";
const SECTION_ACTION: &str = "Desktop Action";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryType {
    /// Application desktop file
    Application,

    /// Link desktop file that opens an URL
    Link,

    // NOTE: this is not in spec but im gonna differentiate it here
    /// Action of an application (not part of XDG spec)
    Action,

    /// Other entry types, just ignored
    Other,
}

impl Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match &self {
            Self::Application => "Application",
            Self::Link => "Link",
            Self::Other => "Other",
            Self::Action => "Action",
        })
    }
}

impl FromStr for EntryType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Application" => Ok(Self::Application),
            "Link" => Ok(Self::Link),
            "Action" => Ok(Self::Action),
            _ => Ok(Self::Other)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Should only be available when Type=Link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,

    /// Does this entry run in terminal
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub terminal: bool,

    // TODO add absolute path to icon for programs that need it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Do not show this entry except in these Desktop Environments
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub only_show_in: Vec<String>,

    /// Actions of the entry
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Self>,

    /// User applied tags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl Entry {
    /// Split entry and its actions into separate entries
    pub fn split_actions(mut self) -> Vec<Self> {
        let mut entries = std::mem::replace(&mut self.actions, vec![]);
        entries.push(self);
        entries
    }

    pub fn from_parser(parser: &IniParser, id: &str, lang: Option<&str>, config: &Config) -> Result<Option<Self>, String> {
        fn get_lang(parser: &IniParser, section: &str, key: &str, lang: &str) -> Option<String> {
            parser.get(section, &format!("{key}{lang}"))
                .or(parser.get(section, "{key}"))
        }

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

        let entry_type: EntryType = parser.get(SECTION, "Type")
            .ok_or_else(|| "Type is required in desktop files".to_string())?
            .parse()
            .unwrap(); // SAFETY: EntryType parse cannot fail

        // ignore other types
        if entry_type == EntryType::Other {
            return Ok(None);
        }

        let mut actions: Vec<Self> = vec![];
        if let Some(action_names) = parser.get(SECTION, "Actions") {
            for name in action_names.split(';') {
                let section = format!("{SECTION_ACTION} {}", name);
                actions.push(Self {
                    id: id.to_string(),
                    name: get_lang(parser, &section, "Name", &lang)
                            .ok_or_else(|| "Name is required in actions".to_string())?,
                    exec: Some(parser.get(&section, "Exec").ok_or_else(|| "Exec is required in actions".to_string())?),
                    icon: parser.get(&section, "Icon")
                            // fallback to application icon
                            .or_else(|| parser.get(SECTION, "Icon")),
                    ..Default::default()
                });
            }
        }

        Ok(Some(Self {
            id: id.to_string(),
            entry_type,
            name: get_lang(parser, SECTION, "Name", &lang).ok_or_else(|| "Name is required in desktop files".to_string())?,
            exec: parser.get(SECTION, "Exec"),
            try_exec: parser.get(SECTION, "TryExec"),
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
                .unwrap_or(vec![]),
            categories: parser
                .get(SECTION, "Categories")
                .map(|x| x.split(';')
                            .map(|y| y.to_string())
                            .collect())
                .unwrap_or(vec![]),
            tags: config.tags.get(id).cloned().unwrap_or(vec![]),
            actions,
        }))
    }
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            entry_type: EntryType::Application,
            name: "".to_string(),
            exec: None,
            try_exec: None,
            url: None,
            generic_name: None,
            comment: None,
            terminal: false,
            icon: None,
            only_show_in: vec![],
            categories: vec![],
            actions: vec![],
            tags: vec![],
        }
    }
}

impl Filterable for Entry {
    fn get(&self, key: &str) -> FilterValue<'_> {
        match key {
            "entry.type" => format!("{}", self.entry_type).into(),
            "entry.name" => self.name.as_str().into(),
            "entry.generic_name" => self.generic_name.as_ref().map(|x| x.as_str()).into(),
            "entry.exec" => self.exec.as_ref().map(|x| x.as_str()).into(),
            "entry.try_exec" => self.try_exec.as_ref().map(|x| x.as_str()).into(),
            "entry.comment" => self.comment.as_ref().map(|x| x.as_str()).into(),
            "entry.categories" => FilterValue::Tuple(self.categories.iter().map(|x| Into::<FilterValue>::into(x.as_str())).collect()),
            "entry.terminal" => self.terminal.into(),
            "entry.icon" => self.icon.as_ref().map(|x| x.as_str()).into(),
            "entry.only_show_in" => FilterValue::Tuple(self.only_show_in.iter().map(|x| Into::<FilterValue>::into(x.as_str())).collect()),
            "entry.tags" => FilterValue::Tuple(self.tags.iter().map(|x| Into::<FilterValue>::into(x.as_str())).collect()),
            _ => FilterValue::Null,
        }
    }
}
