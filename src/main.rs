use structopt::StructOpt;
use anyhow::anyhow;
use std::path::PathBuf;
use directories::ProjectDirs;

mod cli;
mod tasks;
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
        Init => interface::init_journal(journal_file),
        Add {description, estimated_time} => interface::add_task(journal_file, description, estimated_time),
        _ => Ok(())
    }?;
    Ok(())
}
