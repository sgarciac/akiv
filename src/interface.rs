// Implementations of the commands. Responsible of
// - validating input..
// - printing the output.
//
// All interactions with the data should be done via models.

use crate::model;
use crate::model::{ WorkState, Task };
use chrono::{DateTime, Duration, Local};
use humantime::format_duration;
use prettytable::Table;
use rusqlite::{params, Connection, Row, OptionalExtension};
use anyhow::{Context, Result};
use anyhow::bail;

///
/// Adds a task to the current day.
///
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
    return Ok(());
}

///
/// Finishes the current task and starts the next.
///
pub fn next(db: Connection) -> Result<()> {
    let state = model::current_work_state(&db)?;

    if matches!(state, WorkState::NoPendingTasks) {
        bail!("There are no pending tasks! use 'akiv add' to add new tasks to your list.");
    }

    if matches!(state, WorkState::Stopped) {
        bail!("Work is stopped. Use 'akiv start' before moving to next task.");
    }

    // At this point there should be an active task.
    let currently_running_task = model::active_task(&db)?.unwrap();

    model::finish_task(&db, currently_running_task.position)?;
    model::start_task(&db, currently_running_task.position + 1)?;

    return Ok(());
}

///
/// Removes the task at the given position.
///
pub fn remove_task(db: Connection, position: u32) -> Result<()> {
    let tasks_count = model::tasks_count(&db)?;
    if tasks_count == 0 {
        bail!("You have no tasks to remove!")
    };

    if position < 0 || position > tasks_count {
        bail!("Unexisting task.")
    }

    model::remove_task(&db, position)?;

    Ok(())
}


///
/// Set the current work state to running.
///
pub fn start(db: Connection) -> Result<()> {
    match model::current_work_state(&db)? {
        WorkState::Running => bail!("You are already working!"),

        WorkState::Stopped => {
            model::switch_work_state(&db)?;
            println!("Running!")
        }
        WorkState::NoPendingTasks => bail!("You have no tasks to work on!")
    }
    // if no task has started yet, start the first task.
    let currently_running_task = model::active_task(&db);
    if currently_running_task?.is_none() {
        model::start_task(&db, 1)?;
    }
    return Ok(());
}

///
/// Set the current work state to stopped.
///
pub fn stop(db: Connection) -> Result<()> {
    match model::current_work_state(&db)? {
        WorkState::Stopped => bail!("Not running."),
        WorkState::Running => {
            model::switch_work_state(&db)?;
            println!("Pause!")
        }
        WorkState::NoPendingTasks => bail!("No pending tasks!"),
    }
    return Ok(());
}

///
/// Print the list of pauses for the current day.
pub fn pauses(db: Connection) -> Result<()> {
    let mut table = Table::new();

    table.add_row(row!["start", "end", "duration"]);

    let stopped_ranges = stopped_ranges(&db)?;
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
    return Ok(());
}

pub fn list(db: Connection) -> Result<()> {
    let mut table = Table::new();

    let current_time: DateTime<Local> = Local::now();


    let pauses = stopped_ranges(&db)?;

    // NOT STARTED
    let mut stmt = db.prepare("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_duration FROM task WHERE day = DATE('now','localtime') ORDER BY position")?;
    let task_iter = stmt.query_map([], |row| {
        return task_from_row(row);
    })?;

    let mut unfinished_tasks_estimated_duration = Duration::seconds(0);

    table.add_row(row![
        "id",
        "task",
        "started at",
        "expected duration",
        "ellapsed",
        "expected end time",
        "end time",
        "Time in pause"
    ]);
    for task in task_iter {
        let task = task.unwrap();

        table.add_row(row![
            task.position,
            task.description,
            format_optional_time(task.started_at),
            format_chrono_duration(task.estimated_duration),
            "",
            "", //task.fmt_estimated_end_time(unfinished_tasks_estimated_duration, paused_time(&task, &pauses)?),
            format_optional_time(task.finished_at),
            format_chrono_duration(paused_time(&task, &pauses)?)
        ]);

        // TODO: For running task, dont count already worked time
        if task.finished_at == None {
            if task.started_at == None {
                unfinished_tasks_estimated_duration =
                    unfinished_tasks_estimated_duration + task.estimated_duration;
            } else {
                let worked_time =
                    (current_time - task.started_at.unwrap()) - (paused_time(&task, &pauses)?);
                unfinished_tasks_estimated_duration = unfinished_tasks_estimated_duration
                    + std::cmp::max(task.estimated_duration - worked_time, Duration::seconds(0));
                println!("{}", unfinished_tasks_estimated_duration)
            }
        }
    }

    table.printstd();

    match current_work_state(&db)? {
        WorkState::NoPendingTasks => println!("No pending tasks."),
        WorkState::Running => println!("Current state: Running."),
        WorkState::Stopped => println!("Current state: Stopped."),
    }

    Ok(())
}

// get a task from a row in this order: day, description, position, created_at, started_at, finished_at, estimated_duration
fn task_from_row(row: &Row) -> rusqlite::Result<Task> {
    let task = Task {
        id: row.get(0)?,
        day: row.get(1)?,
        description: row.get(2)?,
        position: row.get::<_, u32>(3)?,
        created_at: row.get::<_, DateTime<Local>>(4)?,
        started_at: row.get::<_, DateTime<Local>>(5).ok(),
        finished_at: row.get::<_, DateTime<Local>>(6).ok(),
        estimated_duration: Duration::seconds(row.get::<_, i64>(7)?),
    };
    return Ok(task);
}



fn task_at(db: &Connection, position: u32) -> Option<Task> {
    let task = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_duration FROM task WHERE day = DATE('now','localtime') AND position = ?1", params![position], |row| task_from_row(row));
    return task.ok();
}


// Returns the duration a task has been in pause.
fn paused_time(
    task: &Task,
    pauses: &Vec<(DateTime<Local>, Option<DateTime<Local>>)>,
) -> Result<Duration> {
    if task.started_at == None {
        return Ok(Duration::seconds(0));
    }

    let pauses_iter = pauses.iter();
    let mut paused_time = Duration::seconds(0);
    for pause in pauses_iter {
        paused_time = paused_time
            + overlap(
                (task.started_at.unwrap(), task.finished_at),
                (pause.0, pause.1),
                Local::now(),
            )
    }

    return Ok(paused_time);
}

// Returns a slice of ranges defining the times where work has been stopped.
fn stopped_ranges(db: &Connection) -> Result<Vec<(DateTime<Local>, Option<DateTime<Local>>)>> {
    let mut stmt = db.prepare(
        "SELECT timestamp FROM work WHERE day = DATE('now','localtime') ORDER BY id ASC",
    )?;
    let mut state_changes_iter =
        stmt.query_map([], |row| return row.get::<_, DateTime<Local>>(0))?;

    // skip the first start
    state_changes_iter.next();
    let mut current_state = WorkState::Running;
    let mut current_start_date: Option<DateTime<Local>> = None;
    let mut ranges: Vec<(DateTime<Local>, Option<DateTime<Local>>)> = Vec::new();
    for state_change in state_changes_iter {
        let change = state_change.unwrap();
        if matches!(current_state, WorkState::Running) {
            current_start_date = Some(change);
            current_state = WorkState::Stopped
        } else {
            ranges.push((current_start_date.unwrap(), Some(change)));
            current_state = WorkState::Running;
        }
    }
    // add an open ended range if stopped
    if matches!(current_state, WorkState::Stopped) {
        ranges.push((current_start_date.unwrap(), None));
    }
    return Ok(ranges);
}

// Returns the duration of the overlap between two ranges. Ranges can have an open end, but no open start.
// If both ranges are open ended, "end" is used as the limit to calculate the duration. "end" should be bigger than both starts.
fn overlap(
    range1: (DateTime<Local>, Option<DateTime<Local>>),
    range2: (DateTime<Local>, Option<DateTime<Local>>),
    end: DateTime<Local>,
) -> Duration {
    // both open ranges
    if range2.1.is_none() && range1.1.is_none() {
        if range1.0 > range2.0 {
            return end - range1.0;
        } else {
            return end - range2.0;
        }
    }

    // range1 fully contains range2
    if range2.1.is_some()
        && (range1.0 <= range2.0)
        && (range1.1.is_none() || (range1.1.unwrap() >= range2.0))
    {
        return range2.1.unwrap() - range2.0;
    }
    // range2 fully contains range1
    if range1.1.is_some()
        && (range2.0 <= range1.0)
        && (range2.1.is_none() || (range2.1.unwrap() >= range1.0))
    {
        return range1.1.unwrap() - range1.0;
    }
    // range1 ends inside range2
    if range1.1.is_some()
        && (range1.1.unwrap() >= range2.0)
        && (range2.1.is_none() || (range2.1.unwrap() >= range1.1.unwrap()))
    {
        return range1.1.unwrap() - range2.0;
    }
    // range2 ends inside range1
    if range2.1.is_some()
        && (range2.1.unwrap() >= range1.0)
        && (range1.1.is_none() || (range1.1.unwrap() >= range2.1.unwrap()))
    {
        return range2.1.unwrap() - range1.0;
    }
    // no overlap
    return Duration::seconds(0);
}

fn format_optional_time(optional_timestamp: Option<DateTime<Local>>) -> String {
    match optional_timestamp {
        Some(timestamp) => timestamp.format("%T").to_string(),
        None => "".to_string(),
    }
}

fn format_chrono_duration(duration: Duration) -> String {
    format_duration(duration.to_std().unwrap()).to_string()
}
