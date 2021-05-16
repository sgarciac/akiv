#[macro_use] extern crate prettytable;

use structopt::StructOpt;
use anyhow::anyhow;
use std::path::PathBuf;
use directories::ProjectDirs;
use chrono::Duration;

mod cli;
mod model;
mod interface;
use rusqlite::{Connection};
use crate::model::{init_journal};

use cli::{Command::*, CommandLineArgs};

fn find_default_journal_file() -> Option<PathBuf> {
    if let Some(base_dirs) = ProjectDirs::from("com","gozque","akiv") {
        let root_dir = base_dirs.data_dir();
        if !root_dir.exists() {
            std::fs::create_dir(root_dir).expect("Failed to create directory.");
        }
        let mut path = PathBuf::from(root_dir);
        path.push("db.sqlite");
        Some(path)
    } else {
        None
    }
}

/// Get a connection to the journal database, creating it if it does
/// not exist.
pub fn get_journal_db(journal_path: PathBuf) -> anyhow::Result<Connection> {
    let journal_exists = journal_path.exists();
    let db = Connection::open(&journal_path)?;
    if !journal_exists {
        init_journal(&db)?;
    }
    Ok(db)
}


fn main() -> anyhow::Result<()> {
    // Get the command-line arguments.
    let CommandLineArgs {
        action,
        journal_file,
    } = CommandLineArgs::from_args();

    // Unpack the journal file.
    let journal_file = journal_file
        .or_else(find_default_journal_file)
        .ok_or(anyhow!("Failed to find journal file."))?;

    let database = get_journal_db(journal_file)?;

    // Perform the action.
    match action {
        Add {description, estimated_time, at} => {
            interface::add_task(database, description, estimated_time, at)
        },
        List => interface::list(database),
        Pauses => interface::pauses(database),
        Start => interface::start(database),
        Stop => interface::stop(database),
        Next => interface::next(database),
        Rm {position} => interface::remove_task(database, position),
    }?;
    Ok(())
}
