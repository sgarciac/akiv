// Implementations of the commands. Responsible of
// - validating input..
// - printing the output.
//
// All interactions with the data should be done via models.

use crate::model;
use crate::model::TaskExtra;
use crate::model::TaskState;
use crate::model::WorkState;
use anyhow::bail;
use anyhow::Result;
use chrono::{DateTime, Duration, Local};
use humantime::format_duration;
use prettytable::{Row, Table};
use rusqlite::Connection;

/// Adds a task to the current day.
///
/// - If 'at' is defined, it attempts to add the task at that
///   position. If 'at' is larger than len(tasks) + 1, it will be
///   replaced by len(tasks) + 1, (that is, after the last one).  If the
///   'at' is zero or less, it will be replaced by 1 (the first task.)
///
/// Adding a task does not set the current work state to "running".
pub fn add_task(
    db: Connection,
    description: String,
    estimated_duration: Duration,
    at: Option<u32>,
) -> Result<()> {
    let tasks_count = model::tasks_count(&db)?;
    let mut position = at.unwrap_or(tasks_count + 1);

    // automatically correct position if its out of bounds.
    if position > tasks_count {
        position = tasks_count + 1;
    }

    if position < 1 {
        position = 1;
    }

    model::add_task(&db, position, &description, estimated_duration)?;

    println!(
        "{}. {} ({})",
        position,
        &description,
        format_chrono_duration(estimated_duration)
    );
    //list(journal_path);
    Ok(())
}

/// Finishes the current task and starts the next, if any. The full
/// behavior of 'next' is described as follows:
///
/// 0. If there is no active task, and work state is "stopped", AND there
///    are not started tasks, if starts the next not started one.
///
/// 1. If the work state is "stopped", nothing happens.
///
/// 2. The active tasks is finished.
///
/// 3. If there are not started tasks, starts the next one.
pub fn next(db: Connection) -> Result<()> {
    let state = model::current_work_state(&db)?;
    let currently_running_task_option = model::active_task(&db)?;

    if matches!(state, WorkState::Stopped) {

        // If its stopped and there are no active tasks, start the next.
        // This happens either at the beginning of the day or after a task
        // was added after all tasks have been completed.
        if currently_running_task_option.is_none() {
            let first_not_started_task_option = model::first_not_started_task(&db)?;
            if let Some(first_not_started_task) = first_not_started_task_option {
                model::switch_work_state(&db)?;
                model::start_task(&db, first_not_started_task.position)?;
                return Ok(())
            }
        }
        bail!("Work is stopped. Use 'akiv start' before moving to next task.");
    }

    // At this point there should be an active task.
    let currently_running_task = currently_running_task_option.unwrap();

    // Stop the currently running task:
    model::finish_task(&db, currently_running_task.position)?;
    // Start the next task if any:
    model::start_task(&db, currently_running_task.position + 1)?;

    // Stop work if there are no tasks left.
    let unfinished_tasks_count = model::unfinished_tasks_count(&db)?;
    if unfinished_tasks_count == 0 {
        model::switch_work_state(&db)?;
    }

    Ok(())
}

/// Removes the task at the given position.
///
/// - Only not started tasks can be removed.
pub fn remove_task(db: Connection, position: u32) -> Result<()> {
    let tasks_count = model::tasks_count(&db)?;
    let first_not_started_task = model::first_not_started_task(&db)?;

    if first_not_started_task.is_none() {
        bail!("You have no tasks to remove!")
    }

    if position < first_not_started_task.unwrap().position {
        bail!("You can only remove non started tasks.")
    }

    if position > tasks_count {
        bail!("Unexisting task.")
    }

    model::remove_task(&db, position)?;

    Ok(())
}

/// Set the current work state to running. It also starts a task if none is
/// running.
pub fn start(db: Connection) -> Result<()> {
    match model::current_work_state(&db)? {
        WorkState::Running => bail!("You are already working!"),

        WorkState::Stopped => {
            let unfinished_tasks_count = model::unfinished_tasks_count(&db)?;
            if unfinished_tasks_count == 0 {
                bail!("There are no tasks to work on!");
            }
            model::switch_work_state(&db)?;
        }
    }
    // if no task has started yet, start the first task that is not running.
    let currently_running_task = model::active_task(&db);

    if currently_running_task?.is_none() {
        let task_to_start = model::first_not_started_task(&db)?;
        if task_to_start.is_some() {
            model::start_task(&db, task_to_start.unwrap().position)?;
        }
    }
    println!("Started!");
    Ok(())
}

/// Set the current work state to stopped.
///
pub fn stop(db: Connection) -> Result<()> {
    match model::current_work_state(&db)? {
        WorkState::Stopped => bail!("Not running."),
        WorkState::Running => {
            model::switch_work_state(&db)?;
            println!("Pause!")
        }
    }
    Ok(())
}

///
/// Print the list of pauses for the current day.
///
pub fn pauses(db: Connection) -> Result<()> {
    let mut table = Table::new();

    table.add_row(row!["start", "end", "duration"]);

    let stopped_ranges = model::stopped_ranges(&db)?;
    for range in stopped_ranges {
        match range.1 {
            Some(end) => table.add_row(row![
                range.0.format("%T"),
                end.format("%T"),
                format_duration((end - range.0).to_std().unwrap())
            ]),
            None => table.add_row(row![
                range.0.format("%T"),
                "-",
                format_duration((Local::now() - range.0).to_std().unwrap())
            ]),
        };
    }

    table.printstd();
    Ok(())
}

///
/// List the daily plan!
///
pub fn list(db: Connection) -> Result<()> {
    let mut table = Table::new();

    let current_time: DateTime<Local> = Local::now();

    let pauses = model::stopped_ranges(&db)?;

    let work_state = model::current_work_state(&db)?;
    let tasks = model::tasks(&db)?;
    let task_iter = tasks.iter();

    let mut unfinished_tasks_estimated_duration = Duration::seconds(0);

    table.add_row(row![
        "id",
        "task",
        "started at",
        "exp. duration",
        "ellapsed",
        "exp. end time",
        "pause time"
    ]);
 
    for task in task_iter {
        let etime: Duration = model::ellapsed_time(task, &pauses)?;

        table.add_row(Row::new(vec![
            cell!(task.position),
            match task.state() {
                TaskState::Active => match work_state {
                    WorkState::Running => cell!(bFG->textwrap::fill(&task.description, 38)),
                    WorkState::Stopped => cell!(bFM->textwrap::fill(&task.description, 38)),
                },
                TaskState::Done => cell!(Fg->textwrap::fill(&task.description, 38)),
                TaskState::Pending => cell!(textwrap::fill(&task.description, 38)),
            },
            cell!(format_optional_time(task.started_at, "".to_string())),
            cell!(format_chrono_duration(task.estimated_duration)),
            if etime > task.estimated_duration {
                cell!(FR->format_chrono_duration(etime))
            } else {
                cell!(format_chrono_duration(etime))
            },
            cell!(format_optional_time(
                model::estimated_end_time(task, unfinished_tasks_estimated_duration, &pauses)?,
                "DONE".to_string()
            )),
            cell!(format_chrono_duration(model::paused_time(task, &pauses)?)),
        ]));

        if task.finished_at.is_none() {
            if task.started_at.is_none() {
                unfinished_tasks_estimated_duration =
                    unfinished_tasks_estimated_duration + task.estimated_duration;
            } else {
                let worked_time = (current_time - task.started_at.unwrap())
                    - (model::paused_time(task, &pauses)?);
                unfinished_tasks_estimated_duration = unfinished_tasks_estimated_duration
                    + std::cmp::max(task.estimated_duration - worked_time, Duration::seconds(0));
            }
        }
    }

    table.printstd();
     
    let first_not_started_option = model::first_not_started_task(&db)?;
    if let Some(first_not_started) = first_not_started_option {
        if first_not_started.position == 1 {
            println!("You have not yet started your work for the day. Type 'akiv start'.");
        }
    }

    Ok(())
}

fn format_optional_time(optional_timestamp: Option<DateTime<Local>>, default: String) -> String {
    match optional_timestamp {
        Some(timestamp) => timestamp.format("%T").to_string(),
        None => default,
    }
}

fn format_chrono_duration(duration: Duration) -> String {
    format_duration(duration.to_std().unwrap()).to_string()
}
