#![allow(unused)]

use std::{path::{Path, PathBuf}, process::Command, rc::Rc};
use anyhow::{Result, anyhow};
use crate::{cli::CmdRofi, config::Config, entry::{Entry, EntryAction, EntryType}, entry_cache::EntryDB, get_cache};

const RETV_INIT_CALL: u8 = 0;
const RETV_SELECTED_ENTRY: u8 = 1;
const RETV_SELECTED_CUSTOM: u8 = 2;
const RETV_DELETED_ENTRY: u8 = 3;
const RETV_CUSTOM_START: u8 = 10;
const RETV_CUSTOM_END: u8 = 28;

const RETV_CUSTOM_KB_1: u8 = 10;
const RETV_CUSTOM_KB_2: u8 = 11;
const RETV_CUSTOM_KB_3: u8 = 12;
const RETV_CUSTOM_KB_4: u8 = 13;
const RETV_CUSTOM_KB_5: u8 = 14;
const RETV_CUSTOM_KB_6: u8 = 15;
const RETV_CUSTOM_KB_7: u8 = 16;
const RETV_CUSTOM_KB_8: u8 = 17;
const RETV_CUSTOM_KB_9: u8 = 18;

/// Not to be confused with `CmdRofi` these are commands that are used in `ROFI_INFO`
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
enum RofiCommand {
    ListEntries,
    Entry {
        entry: Rc<Entry>,
        force_execute: bool,
    },
    // ListActions(Rc<Entry>),
    ExecuteAction(Rc<Entry>, Rc<EntryAction>),
}

impl Default for RofiCommand {
    fn default() -> Self {
        Self::ListEntries
    }
}

impl RofiCommand {
    /// Serializes into a string of hex
    pub fn serialize(&self) -> Result<String> {
        Ok(hex::encode(rkyv::to_bytes::<rkyv::rancor::Error>(self)?))
    }

    /// Deserializes from a string of hex
    pub fn deserialize(input: &str) -> Result<Self> {
        Ok(rkyv::from_bytes::<Self, rkyv::rancor::Error>(&hex::decode(input)?)?)
    }
}

fn get_fallback_icon(entry_type: &EntryType) -> &'static str {
    match entry_type {
        EntryType::Application => "application-x-executable",
        EntryType::Link => "open-link",
        EntryType::Other => "",
    }
}

pub fn rofi(cli_args: &crate::cli::Cli, args: CmdRofi, config: Config) -> Result<()> {
    let status = args.rofi_status;

    match status {
        RETV_INIT_CALL => {
            println!("\0no-custom\x1ftrue");    // no custom entries
            println!("\0use-hot-keys\x1ftrue"); // allow custom keybindings
        },
        RETV_SELECTED_ENTRY | RETV_CUSTOM_KB_1 => {},
        _ => return Ok(()), // ignore all other actions
    }

    let cmd = if let Ok(data) = std::env::var("ROFI_INFO") {
        RofiCommand::deserialize(&data)?
    } else {
        RofiCommand::default()
    };

    match &cmd {
        RofiCommand::ListEntries => {
            // reuse global cache
            let cache = get_cache(cli_args.cache.as_deref(), &config)?;

            for entry in &cache.entries {
                println!(
                    "{name}\0icon\x1f{icon}\x1fmeta\x1f{meta}\x1finfo\x1f{info}",
                    name = entry.name,
                    icon = entry.icon.as_ref().map(|x| x.as_str()).unwrap_or_else(|| get_fallback_icon(&entry.entry_type)),
                    meta = format!("{} {} {}",
                        entry.generic_name.as_ref().map(|x| x.as_str()).unwrap_or(""),
                        entry.comment.as_ref().map(|x| x.as_str()).unwrap_or(""),
                        // TODO i just added all the info but do categories fit here at all?
                        entry.categories.join(" "),
                    ),
                    info = RofiCommand::Entry { entry: Rc::clone(entry), force_execute: false }.serialize()?,
                );
            }
        },

        RofiCommand::Entry { entry, force_execute } => {
            if status == RETV_SELECTED_ENTRY || *force_execute {
                crate::execute_entry(&cli_args, &entry)?;
                std::process::exit(0); // also terminated rofi
            } else {
                println!(
                    "Start\0icon\x1f{icon}\x1finfo\x1f{info}",
                    icon = entry.icon.as_ref().map(|x| x.as_str()).unwrap_or_else(|| get_fallback_icon(&entry.entry_type)),
                    info = RofiCommand::Entry { entry: Rc::clone(entry), force_execute: true }.serialize()?,
                );

                for action in &entry.actions {
                    println!(
                        "{name}\0icon\x1f{icon}\x1finfo\x1f{info}",
                        name = action.name,
                        icon = action.icon.as_ref().map(|x| x.as_str()).unwrap_or_else(|| get_fallback_icon(&entry.entry_type)),
                        info = RofiCommand::ExecuteAction(Rc::clone(&entry), Rc::clone(&action)).serialize()?,
                    );
                }

                println!(
                    "Back\0icon\x1f{icon}\x1finfo\x1f{info}",
                    icon = "go-previous",
                    info = RofiCommand::ListEntries.serialize()?,
                );
            }
        }

        RofiCommand::ExecuteAction(entry, action) => {
            crate::execute_action(&cli_args, &entry, &action)?;
            std::process::exit(0); // also terminated rofi
        }
    }

    Ok(())
}
