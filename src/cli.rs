use std::path::PathBuf;
use structopt::StructOpt;
use humantime::parse_duration;
use std::time::Duration;

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Write a new entry.
    Add {
        /// The task description text.
        #[structopt()]
        description: String,

        /// The task estimated duration (parse_duration)
        #[structopt(parse(try_from_str=parse_duration))]
        estimated_time: Duration,
    },
    /// Remove an entry from the journal file by position.
    Done {
        #[structopt()]
        position: usize,
    },
    /// List all tasks in the journal file.
    List,
    /// Init the journal file
    Init,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Akiv",
    about = "A hyper-minimalistic daily planner."
)]
pub struct CommandLineArgs {
    #[structopt(subcommand)]
    pub action: Command,

    /// Use a different journal file.
    #[structopt(parse(from_os_str), short, long)]
    pub journal_file: Option<PathBuf>,
}
