#![allow(unused)]

use std::rc::Rc;
use anyhow::{Result, anyhow};
use crate::{cli::CmdRofi, config::Config, entry::{Entry, EntryAction}, get_cache};

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

// TODO add open in submenu before the action
// TODO add back option in submenu to leave it?

/// Not to be confused with `CmdRofi` these are commands that are used in `ROFI_INFO`
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
enum RofiCommand {
    Entry(Rc<Entry>),
    Action(Rc<EntryAction>),
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

// TODO potentionally use a tmpfile with whole cache so it cannot change while rofi is running?
// just store the path in ROFI_DATA
// TODO calling the actuall application with wrappers like systemd-cat or swaymsg
pub fn rofi(cli_args: &crate::cli::Cli, args: CmdRofi, config: Config) -> Result<()> {
    let status = args.rofi_status;

    match status {
        RETV_INIT_CALL => {
            println!("\0no-custom\x1ftrue");    // no custom entries
            println!("\0use-hot-keys\x1ftrue"); // allow custom keybindings

            let cache = get_cache(cli_args.cache.as_deref(), &config)?;

            for entry in &cache.entries {
                // TODO generic placeholder icon if its missing? seperate one for links
                // TODO only print options if there is any data
                println!(
                    "{name}\0icon\x1f{icon}\x1fmeta\x1f{meta}\x1finfo\x1f{info}",
                    name = entry.name,
                    icon = entry.icon.as_ref().map(|x| x.as_str()).unwrap_or("application-default"),
                    meta = format!("{} {} {}",
                        entry.generic_name.as_ref().map(|x| x.as_str()).unwrap_or(""),
                        entry.comment.as_ref().map(|x| x.as_str()).unwrap_or(""),
                        // TODO i just added all the info but do categories fit here at all?
                        entry.categories.join(" "),
                    ),
                    info = RofiCommand::Entry(Rc::clone(entry)).serialize()?,
                );
            }
        },

        // selected entry or custom keybinding 1
        RETV_SELECTED_ENTRY | RETV_CUSTOM_KB_1 => {
            let info = std::env::var("ROFI_INFO").unwrap();
            match RofiCommand::deserialize(&info)? {
                // select the action normally
                RofiCommand::Entry(entry) if status == RETV_SELECTED_ENTRY => {
                    println!("would execute entry {:?} exec {:?}", entry.name, entry.exec);
                },

                // alt select open the actions
                RofiCommand::Entry(entry) => {
                    for action in &entry.actions {
                        println!(
                            "{name}\0icon\x1f{icon}\x1finfo\x1f{info}",
                            name = action.name,
                            icon = action.icon.as_ref().map(|x| x.as_str()).unwrap_or(""),
                            info = RofiCommand::Action(Rc::clone(action)).serialize()?,
                        );
                    }
                },

                RofiCommand::Action(action) => {
                    println!("should execute action exec {:?}", action.exec);
                }
            }
        },

        val => {
            return Err(anyhow!("invalid ROFI_RETV value {val:?}"));
        },
    };

    Ok(())
}
