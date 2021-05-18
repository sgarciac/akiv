use anyhow::{Context, Result};
use chrono::{DateTime, Duration, DurationRound, Local};
use rusqlite::{params, Connection, OptionalExtension, Row};

/// A single task, saved as an entry in the stasks table.
#[derive(Debug)]
pub struct Task {
    pub id: u32,
    pub description: String,
    pub created_at: DateTime<Local>,
    pub started_at: Option<DateTime<Local>>,
    pub finished_at: Option<DateTime<Local>>,
    pub day: String,
    pub position: u32,
    pub estimated_duration: Duration, // in seconds
}

/// An enumeration to capture the possible states of the work
/// activity.  The user is either working or not working. The program
/// is always stopped if there are no pending tasks.
#[derive(Debug)]
pub enum WorkState {
    Running,
    Stopped,
}

/// The state of a task.
#[derive(Debug)]
pub enum TaskState {
    Done,
    Active,
    Pending,
}

/// Get an iterator to the daily tasks
pub fn tasks(db: &Connection) -> Result<Vec<Task>> {
    let mut stmt = db.prepare("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_duration FROM task WHERE day = DATE('now','localtime') ORDER BY position")?;
    let mapped_rows = stmt.query_map([], |row| {
        return task_from_row(row);
    })?;

    let mut tasks = Vec::new();
    for task in mapped_rows {
        tasks.push(task?);
    }

    Ok(tasks)
}

/// Initialize the journal database.
pub fn init_journal(db: &Connection) -> Result<()> {
    db.execute(
        "CREATE TABLE if not exists task (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  day             TEXT NOT NULL,
                  description     TEXT NOT NULL,
                  position        INTEGER NOT NULL,
                  created_at      TEXT NOT NULL,
                  started_at      TEXT,
                  finished_at     TEXT,
                  estimated_duration  INTEGER NOT NULL
                  )",
        [],
    )
    .context("Faied to create task table.")?;

    db.execute(
        "CREATE UNIQUE INDEX day_position ON task (day, position)",
        [],
    )
    .context("Failed to create unique index on task table.")?;

    db.execute(
        "CREATE TABLE if not exists work (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  day             TEXT NOT NULL,
                  timestamp       TEXT
                  )",
        [],
    )
    .context("Failed to create work table.")?;

    db.execute("CREATE INDEX day_index ON work (day)", [])
        .context("Failed to create unique index on work table.")?;

    Ok(())
}

/// Return the number of tasks for the current day.
pub fn tasks_count(db: &Connection) -> Result<u32> {
    let count = db
        .query_row(
            "SELECT count(*) from task where day = DATE('now', 'localtime')",
            [],
            |row| row.get::<_, u32>(0),
        )
        .context("Failed to count tasks from database.")?;
    Ok(count)
}

/// Return the number of unfinished tasks for the current day (including the active one).
pub fn unfinished_tasks_count(db: &Connection) -> Result<u32> {
    let count = db
        .query_row(
            "SELECT count(*) FROM task WHERE day = DATE('now','localtime') AND finished_at IS NULL",
            [],
            |row| row.get::<_, u32>(0),
        )
        .context("Failed to count unfinished tasks from database.")?;
    return Ok(count);
}

/// Add a task to the current day, at the defined position. It will
/// move all positions from and after it (if any) to the right to
/// prevent two tasks at the same place. Position is expected to be between (and including) 1 and N+1,
/// and the list of tasks is expected not to contain gaps.
pub fn add_task(
    db: &Connection,
    position: u32,
    description: &String,
    estimated_duration: Duration,
) -> Result<()> {
    // hack to shift all positions after the insert to the right without breaking the unique constraint.
    db.execute("UPDATE task set position = - (position + 1) where day = DATE('now', 'localtime') and position >= ?1",
               params![position])
        .context("Failed to shift tasks to the right in database.")?;

    db.execute("UPDATE task set position = - position where day = DATE('now', 'localtime') and position < 0",[])
        .context("Failed to shift tasks to the right in database.")?;

    db.execute("INSERT INTO task (day, description, position, created_at, estimated_duration) VALUES(DATE('now', 'localtime'), ?1, ?2, CURRENT_TIMESTAMP, ?3)",
               params![description, position, estimated_duration.to_std()?.as_secs()]).context("Failed to insert task to database.")?;
    Ok(())
}

/// Return whether the user has declared to be currently:
/// - working
/// - in a pause
/// - has no more tasks left to work on.
pub fn current_work_state(db: &Connection) -> Result<WorkState> {
    let switchs_count = db
        .query_row(
            "SELECT count(*) FROM work WHERE day = DATE('now','localtime') ",
            [],
            |row| row.get::<_, usize>(0),
        )
        .context("Failed to count work entries from database.")?;

    if switchs_count % 2 != 0 {
        return Ok(WorkState::Running);
    } else {
        return Ok(WorkState::Stopped);
    }
}

/// Remove a task from the database, shifting tasks from and after it (if any) to the left,
/// to close the gap.
pub fn remove_task(db: &Connection, position: u32) -> Result<()> {
    db.execute(
        "DELETE FROM task where day = DATE('now', 'localtime') and position = ?1",
        params![position],
    )
    .context("Failed to remove tasks from database.")?;

    // hack to shift all positions after the remove to the left without breaking the unique constraint.
    db.execute("UPDATE task set position = - (position - 1) where day = DATE('now', 'localtime') and position > ?1", params![position])
        .context("Failed to shift tasks to the left")?;
    db.execute("UPDATE task set position = - position  where day = DATE('now', 'localtime') and position < 0", [])
        .context("Failed to shift tasks to the left")?;
    Ok(())
}

/// If the current work state is running, add a stop. If the current
/// work state is stopped, add a start.
pub fn switch_work_state(db: &Connection) -> Result<()> {
    db.execute(
        "INSERT INTO work (day, timestamp) VALUES(DATE('now', 'localtime'), CURRENT_TIMESTAMP)",
        [],
    )
    .context("Failed to insert entry to the work table.")?;
    return Ok(());
}

/// Finish task at given position. It supposes task to be active.
pub fn finish_task(db: &Connection, position: u32) -> Result<()> {
    db.execute(
        "UPDATE task set finished_at = CURRENT_TIMESTAMP where position = ?1",
        params![position],
    )
    .context("Failed to finish task in the database")?;
    Ok(())
}

/// Start task at given position. It does nothing if the task does not exist.
pub fn start_task(db: &Connection, position: u32) -> Result<()> {
    db.execute(
        "UPDATE task set started_at = CURRENT_TIMESTAMP where position = ?1",
        params![position],
    )
    .context("Failed to start task in the database")?;
    Ok(())
}

/// Returns the currently active task, if any. This is, the task that has been started but not finished.
pub fn active_task(db: &Connection) -> Result<Option<Task>> {
    let task = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_duration FROM task WHERE day = DATE('now','localtime') AND started_at IS NOT NULL AND finished_at IS NULL ORDER BY position LIMIT 1",
                            [],
                            |row| task_from_row(row)).optional().context("Failed to obtain active tasks from database.")?;
    return Ok(task);
}

/// Returns the first not running job, if any.
pub fn first_not_started_task(db: &Connection) -> Result<Option<Task>> {
    let task = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_duration FROM task WHERE day = DATE('now','localtime') AND started_at IS NULL ORDER BY position LIMIT 1",
                            [],
                            |row| task_from_row(row)).optional().context("Failed to obtain active tasks from database.")?;
    return Ok(task);
}

/// Return a task from a row in this order: [day, description,
/// position, created_at, started_at, finished_at, estimated_duration]
pub fn task_from_row(row: &Row) -> rusqlite::Result<Task> {
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

type Pauses = Vec<(DateTime<Local>, Option<DateTime<Local>>)>;

/// Returns a slice of ranges defining the times where work has been stopped.
/// If the work is currently stopped, the last range is open ended.
pub fn stopped_ranges(db: &Connection) -> Result<Pauses> {
    let mut stmt = db
        .prepare("SELECT timestamp FROM work WHERE day = DATE('now','localtime') ORDER BY id ASC")
        .context("Failed to fetch work from database.")?;

    let mut state_changes_iter = stmt
        .query_map([], |row| return row.get::<_, DateTime<Local>>(0))
        .context("Failed to fetch work from database.")?;

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

/// Get the Task at a given position.
//pub fn task_at(db: &Connection, position: u32) -> Option<Task> {
//    let task = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_duration FROM task WHERE day = DATE('now','localtime') AND position = ?1", params![position], |row| task_from_row(row)).with_context(|| format!("Failed to get task at position {} from database.", position));
//    return task.ok();
//}

/// Calculate the total time a task has been stopped.
/// with seconds precision.
pub fn paused_time(
    task: &Task,
    pauses: &Vec<(DateTime<Local>, Option<DateTime<Local>>)>,
) -> Result<Duration> {
    // If the task has not started, it has not been paused.
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
                clt_secs()?,
            )
    }
    return Ok(paused_time);
}

/// Calculate the total time the used has worked on a task (that is without the pauses)
/// with seconds precision.
pub fn ellapsed_time(
    task: &Task,
    pauses: &Vec<(DateTime<Local>, Option<DateTime<Local>>)>,
) -> Result<Duration> {
    match task.state() {
        TaskState::Pending => Ok(Duration::seconds(0)),
        TaskState::Active => {
            Ok((clt_secs()? - task.started_at.unwrap()) - paused_time(&task, pauses)?)
        }
        TaskState::Done => {
            println!(
                "{}",
                (task.finished_at.unwrap() - task.started_at.unwrap())
                    .num_microseconds()
                    .unwrap()
            );
            println!(
                "{}",
                (paused_time(&task, pauses)?).num_microseconds().unwrap()
            );
            println!(
                "{}",
                ((task.finished_at.unwrap() - task.started_at.unwrap())
                    - paused_time(&task, pauses)?)
                .num_microseconds()
                .unwrap()
            );
            Ok(std::cmp::max(
                Duration::seconds(0),
                (task.finished_at.unwrap() - task.started_at.unwrap())
                    - paused_time(&task, pauses)?,
            ))
        }
    }
}

/// Returns the duration of the overlap between two ranges. Ranges can have an
/// open end, but no open start.  If both ranges are open ended, "end" is used
/// as the limit to calculate the duration. "end" should therefore be bigger
/// than both starts.
pub fn overlap(
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

/// It returns the estimated time for tasks that have not been finished.
/// None for those that have.
/// parameters:
///
/// task: the task for which the estimated end time is being calculated.
/// before: the estimated time it will take to start the task.
/// pauses: The day pauses.
pub fn estimated_end_time(
    task: &Task,
    before: Duration,
    pauses: &Pauses, //    paused_time: Duration,
) -> Result<Option<DateTime<Local>>> {
    let local_time: DateTime<Local> = Local::now();
    let paused_time = paused_time(&task, pauses)?;

    if task.finished_at == None {
        if let Some(started_at) = task.started_at {
            let worked_time = (local_time - started_at) - paused_time;
            let end_time = local_time
                + std::cmp::max(Duration::seconds(0), task.estimated_duration - worked_time);
            Ok(Some(end_time))
        } else {
            Ok(Some(local_time + before + task.estimated_duration))
        }
    } else {
        Ok(None)
    }
}

/// Return the current local, with seconds precision
fn clt_secs() -> Result<DateTime<Local>> {
    let clt = Local::now().duration_round(Duration::seconds(1))?;
    Ok(clt)
}

/// Traits
pub trait TaskExtra {
    fn is_active(&self) -> bool;
    fn is_done(&self) -> bool;
    fn state(&self) -> TaskState;
}

impl TaskExtra for Task {
    fn is_active(&self) -> bool {
        self.started_at.is_some() && self.finished_at.is_none()
    }

    fn is_done(&self) -> bool {
        self.finished_at.is_some()
    }

    fn state(&self) -> TaskState {
        if self.is_active() {
            TaskState::Active
        } else {
            if self.is_done() {
                TaskState::Done
            } else {
                TaskState::Pending
            }
        }
    }
}
