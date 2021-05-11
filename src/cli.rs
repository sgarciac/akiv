use std::path::PathBuf;
use structopt::StructOpt;
use humantime::parse_duration;
use std::time::Duration;

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Add a new task to the list.
    Add {
        /// Position at which the item should be inserted. By default, the task will be inserted at the end.
        #[structopt(short, about = "insert task after this.")]
        at: Option<u32>,

        /// The task's description.
        #[structopt()]
        description: String,

        /// The task's estimated duration.
        #[structopt(parse(try_from_str=parse_duration))]
        estimated_time: Duration

    },
    /// Remove a task.
    Rm {
        #[structopt()]
        position: usize,
    },
    /// List all tasks in the journal file.
    List,
    /// List all pauses in the journal file.
    Pauses,
    /// Mark current task as done, and advance to next task.
    Next,
    /// Start working
    Start,
    /// Stop working
    Stop,
    /// Init the journal file.
    Init,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Akiv",
    about = "The ultra minimalistic daily planner."
)]
pub struct CommandLineArgs {
    #[structopt(subcommand)]
    pub action: Command,

    /// Use a different journal file.
    #[structopt(parse(from_os_str), short, long)]
    pub journal_file: Option<PathBuf>,
}
