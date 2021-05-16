use chrono::{DateTime, Local, Duration};
use rusqlite::{params, Connection, Row, OptionalExtension};
use anyhow::{Context, Result};
use anyhow::anyhow;
use anyhow::Result;

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

/// An enumeration to capture the possible states of the work activity.
/// The user is either working on a task, on pause, or she has no tasks
/// to work on.
#[derive(Debug)]
pub enum WorkState {
    Running,
    Stopped,
    NoPendingTasks
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
    ).context("Faied to create task table.")?;

    db.execute(
        "CREATE UNIQUE INDEX day_position ON task (day, position)",
        [],
    ).context("Failed to create unique index on task table.")?;

    db.execute(
        "CREATE TABLE if not exists work (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  day             TEXT NOT NULL,
                  timestamp       TEXT
                  )",
        [],
    ).context("Failed to create work table.")?;

    db.execute("CREATE INDEX day_index ON work (day)", [])
        .context("Failed to create unique index on work table.")?;

    Ok(())
}

/// Return the number of tasks for the current day.
pub fn tasks_count(db: &Connection) -> Result<u32> {
    let count = db.query_row(
        "SELECT count(*) from task where day = DATE('now', 'localtime')",
        [],
        |row| row.get::<_, u32>(0),
    ).context("Failed to count tasks from database.")?;
    Ok(count)
}

/// Return the number of unfinished tasks for the current day.
fn pending_tasks_count(db: &Connection) -> Result<u32> {
    let count = db.query_row(
        "SELECT count(*) FROM task WHERE day = DATE('now','localtime') AND finished_at IS NULL",
        [],
        |row| row.get::<_, u32>(0),
    ).context("Failed to count unfinished tasks from database.")?;
    return Ok(count);
}


/// Add a task to the current day, at the defined position. It will
/// move all positions from and after it (if any) to the right to
/// prevent two tasks at the same place. Position is expected to be between (and including) 1 and N+1,
/// and the list of tasks is expected not to contain gaps.
pub fn add_task(db: &Connection, position: u32, description: &String, estimated_duration: Duration) -> Result<()> {
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
    if pending_tasks_count(db)? == 0 {
        return Ok(WorkState::NoPendingTasks);
    }

    let switchs_count = db.query_row(
        "SELECT count(*) FROM work WHERE day = DATE('now','localtime') ",
        [],
        |row| row.get::<_, usize>(0),
    ).context("Failed to count work entries from database.")?;

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
    ).context("Failed to remove tasks from database.")?;

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
    ).context("Failed to insert entry to the work table.")?;
    return Ok(());
}


/// Finish task at given position. It supposes task to be active.
pub fn finish_task(db: &Connection, position: u32) -> Result<()> {
    db.execute(
        "UPDATE task set finished_at = CURRENT_TIMESTAMP where position = ?1",
        params![position],
    ).context("Failed to finish task in the database")?;
    Ok(())
}

/// Start task at given position. It supposes task to be active.
pub fn start_task(db: &Connection, position: u32) -> Result<()> {
    db.execute(
        "UPDATE task set started_at = CURRENT_TIMESTAMP where position = ?1",
        params![position],
    ).context("Failed to start task in the database")?;
    Ok(())
}


/// Returns the currently active task, if any. This is, the task that has been started but not finished.
pub fn active_task(db: &Connection) -> Result<Option<Task>> {
    let task = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_duration FROM task WHERE day = DATE('now','localtime') AND started_at IS NOT NULL AND finished_at IS NULL ORDER BY position LIMIT 1",
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


//    fn fmt_estimated_end_time(&self, before: i64, paused_time: Duration) -> String {
        //let local_time: DateTime<Local> = Local::now();
        //if self.finished_at == None {
        //    if self.started_at == None {
        //        (local_time + Duration::seconds(i64::from(before + i64::from(self.estimated_duration)))).format("%T").to_string()
        //    } else {
        //        let worked_time = (local_time - self.started_at.unwrap()) - paused_time;
        //        println!("worked time {}", worked_time.num_seconds());
        //       ((local_time + Duration::seconds(std::cmp::max(i64::from(self.estimated_duration) - worked_time.num_seconds(),0)))).format("%T").to_string()
        //    }

//        } else {
  //          "".to_string()
        //    }
