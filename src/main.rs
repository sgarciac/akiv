#[macro_use] extern crate prettytable;

use structopt::StructOpt;
use anyhow::anyhow;
use std::path::PathBuf;
use directories::ProjectDirs;

mod cli;
mod model;
mod interface;

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

    // Perform the action.
    match action {
        Add {description, estimated_time, at} => {
            interface::add_task(journal_file, description, estimated_time.as_secs(), at)
        },
        List => interface::list(journal_file),
        Pauses => interface::pauses(journal_file),
        Start => interface::start(journal_file),
        Stop => interface::stop(journal_file),
        Next => interface::next(journal_file),
        Rm {position} => interface::remove_task(journal_file, position),
    }?;
    Ok(())
}
