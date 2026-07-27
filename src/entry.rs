use std::{fmt::Display, hash::Hash, str::FromStr};

use filt_rs::{FilterValue, Filterable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

// TODO TryExec and spec Version key
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub entry_type: EntryType,
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub url: Option<String>,
    pub categories: Vec<String>,
    pub terminal: bool,
    pub no_display: bool,
    pub icon: Option<String>,
    pub only_show_in: Option<String>,

    pub actions: Vec<Self>,
}

impl Entry {
    /// Split entry and its actions into separate entries
    pub fn split_actions(mut self) -> Vec<Self> {
        let mut entries = std::mem::replace(&mut self.actions, vec![]);
        entries.push(self);
        entries
    }
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            entry_type: EntryType::Application,
            name: "".to_string(),
            exec: None,
            url: None,
            generic_name: None,
            comment: None,
            terminal: false,
            no_display: false,
            icon: None,
            only_show_in: None,
            categories: vec![],
            actions: vec![],
        }
    }
}

impl Filterable for Entry {
    fn get(&self, key: &str) -> FilterValue<'_> {
        match key {
            "entry.type" => format!("{}", self.entry_type).into(),
            "entry.name" => self.name.clone().into(),
            "entry.generic_name" => self.generic_name.clone().into(),
            "entry.exec" => self.exec.clone().into(),
            "entry.comment" => self.comment.clone().into(),
            "entry.categories" => FilterValue::Tuple(self.categories.iter().map(|x| Into::<FilterValue>::into(x.as_str())).collect()),
            "entry.terminal" => self.terminal.clone().into(),
            "entry.no_display" => self.no_display.clone().into(),
            "entry.icon" => self.icon.clone().into(),
            "entry.only_show_in" => self.only_show_in.clone().into(),

            _ => FilterValue::Null,
        }
    }
}
