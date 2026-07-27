use clap::{Args, Parser, Subcommand};
// use zbus::zvariant::{OwnedValue, Value};

/// Utility for interacting with and querying tray indicators
///
/// Intended for use with window managers but works on any complient desktop
/// environment
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: CliCommands,
}

// #[derive(Args, Debug, Clone)]
// pub struct CmdList {
//     /// Print help for this command
//     #[clap(short, long, exclusive = true)]
//     pub help: bool,
// }
//
// #[derive(Args, Debug, Clone)]
// pub struct CmdActivate {
//     /// Call context menu
//     #[clap(short, long, conflicts_with = "secondary")]
//     pub context_menu: bool,
//
//     /// Call secondary activate
//     #[clap(short, long)]
//     pub secondary: bool,
//
//     #[clap(value_name = "ITEM_PATH")]
//     pub path: String,
//
//     /// X position of activation
//     pub x: i32,
//
//     /// Y position of activation
//     pub y: i32,
// }
//
// #[derive(Args, Debug, Clone)]
// pub struct CmdScroll {
//     /// Path to the StatusNotifierItem object including destination
//     #[clap(value_name = "ITEM_PATH")]
//     pub path: String,
//
//     /// How much was scrolled
//     pub delta: i32,
//
//     /// Direction of scrolling
//     pub orientation: String,
// }
//
// #[derive(Args, Debug, Clone)]
// pub struct CmdLayout {
//     /// Show hidden menu items
//     #[clap(short='H', long)]
//     pub hidden: bool,
//
//     /// Show disabled menu items
//     #[clap(short, long)]
//     pub disabled: bool,
//
//     /// Path to the dbusmenu object including destination
//     #[clap(value_name = "MENU_PATH")]
//     pub path: String,
// }
//
// fn parse_data(input: &str) -> Result<OwnedValue, String> {
//     // TODO brute force approach at parsing the data basically try everything
//     if let Ok(val) = input.parse::<i32>() {
//         Ok(OwnedValue::try_from(val).unwrap())
//     } else if let Ok(val) = input.parse::<f64>() {
//         Ok(OwnedValue::try_from(val).unwrap())
//     } else {
//         match input {
//             "true" | "false" => Ok(OwnedValue::try_from(Value::Bool(input == "true")).unwrap()),
//             _ => Ok(OwnedValue::try_from(Value::Str(input.into())).unwrap())
//         }
//     }
// }
//
// #[derive(Args, Debug, Clone)]
// pub struct CmdClick {
//     /// Path to the dbusmenu object including destination
//     #[clap(value_name = "MENU_PATH")]
//     pub path: String,
//
//     /// Id of the menu item
//     pub id: i32,
//
//     #[clap(value_parser = parse_data, allow_negative_numbers = true)]
//     pub data: Option<OwnedValue>,
// }
//
// // TODO Daemonization should be the default, use crate?
// #[derive(Args, Debug, Clone)]
// pub struct CmdDaemon {
//     /// Run in foreground
//     #[clap(long = "fg")]
//     pub foreground: bool,
// }

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    // /// List tray items
    // #[command(disable_help_flag = true)]
    // List(CmdList),
    //
    // /// Call activate on a tray item
    // Activate(CmdActivate),
    //
    // /// Call scroll on a tray item
    // Scroll(CmdScroll),
    //
    // /// Get tray item menu layout
    // Layout(CmdLayout),
    //
    // /// Activate specific item in menu
    // Click(CmdClick),

    // TODO add option for query with rest=true
    List,

    // test command
    Test,

    #[clap(skip)]
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
