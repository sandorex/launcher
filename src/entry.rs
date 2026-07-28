use std::{fmt::Display, hash::Hash, str::FromStr};

use filt_rs::{FilterValue, Filterable};
use serde::{Deserialize, Serialize};

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

// TODO TryExec and spec Version key
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub entry_type: EntryType,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generic_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,

    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub terminal: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub only_show_in: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
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
            icon: None,
            only_show_in: vec![],
            categories: vec![],
            actions: vec![],
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
            "entry.comment" => self.comment.as_ref().map(|x| x.as_str()).into(),
            "entry.categories" => FilterValue::Tuple(self.categories.iter().map(|x| Into::<FilterValue>::into(x.as_str())).collect()),
            "entry.terminal" => self.terminal.into(),
            "entry.icon" => self.icon.as_ref().map(|x| x.as_str()).into(),
            "entry.only_show_in" => FilterValue::Tuple(self.only_show_in.iter().map(|x| Into::<FilterValue>::into(x.as_str())).collect()),
            _ => FilterValue::Null,
        }
    }
}
