use std::path::PathBuf;
use chrono::{DateTime, Duration, Local};
use rusqlite::{params, Connection, Result, Row};
use crate::model;
use model::TaskLabels;
use std::time::Duration as STDDuration;
use humantime::format_duration;
use prettytable::{Table};

pub fn init_journal(journal_path: PathBuf) -> Result<()> {
    let conn = Connection::open(&journal_path)?;
    conn.execute(
        "CREATE TABLE if not exists task (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  day             TEXT NOT NULL,
                  description     TEXT NOT NULL,
                  position        INTEGER NOT NULL,
                  created_at      TEXT NOT NULL,
                  started_at      TEXT,
                  finished_at     TEXT,
                  estimated_time  INTEGER NOT NULL
                  )",

        [],
    )?;
    conn.execute("CREATE UNIQUE INDEX day_position ON task (day, position)",[])?;
    conn.execute(
        "CREATE TABLE if not exists work (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  day             TEXT NOT NULL,
                  timestamp       TEXT
                  )",

        [],
    )?;
    conn.execute("CREATE INDEX day_index ON work (day)",[])?;
    Ok(())
}

pub fn add_task(journal_path: PathBuf, description: String, estimated_time: u64, at: Option<u32>) -> Result<()> {
    let conn = Connection::open(&journal_path)?;

    let tasks_count = conn.query_row("SELECT count(*) from task where day = DATE('now', 'localtime')", [], |row| row.get::<_,u32>(0))?;
    let mut position = at.unwrap_or(tasks_count + 1);

    if position > tasks_count {
        position = tasks_count + 1;
    }

    // hack to shift all positions after the insert to the right without breaking the unique constraint.
    conn.execute("UPDATE task set position = - (position + 1) where day = DATE('now', 'localtime') and position >= ?1",
                 params![position])?;
    conn.execute("UPDATE task set position = - position where day = DATE('now', 'localtime') and position < 0",[])?;


    conn.execute("INSERT INTO task (day, description, position, created_at, estimated_time) VALUES(DATE('now', 'localtime'), ?1, ?2, CURRENT_TIMESTAMP, ?3)",
                 params![description, position, estimated_time])?;


    println!("{}. {} ({})", position, description, format_duration(STDDuration::from_secs(u64::from(estimated_time))));
    //list(journal_path);
    return Ok(())
}

pub fn next(journal_path: PathBuf) -> Result<()>{
    let conn = Connection::open(&journal_path)?;
    let state = current_work_state(&conn)?;

    if matches!(state,model::WorkState::NoPendingTasks ) {
        println!("There are no pending tasks! use 'akiv add' to add new tasks to your list.");
        return Ok(());
    }

    if matches!(state,model::WorkState::Stopped ) {
        println!("Work is stopped. Use 'akiv start' before completing current task.");
        return Ok(());
    }

    let current_task = current_task(&conn)?;
    conn.execute("UPDATE task set finished_at = CURRENT_TIMESTAMP where id = ?1", params![current_task.id])?;
    return Ok(());
}

pub fn remove_task(journal_path: PathBuf, position: usize) -> Result<()> {
    let conn = Connection::open(&journal_path)?;
    conn.execute("DELETE FROM task where day = DATE('now', 'localtime') and position = ?1", params![position])?;

    // hack to shift all positions after the remove to the left without breaking the unique constraint.
    conn.execute("UPDATE task set position = - (position - 1) where day = DATE('now', 'localtime') and position > ?1", params![position])?;
    conn.execute("UPDATE task set position = - position  where day = DATE('now', 'localtime') and position < 0", [])?;
    Ok(())
}

pub fn start(journal_path: PathBuf) -> Result<()> {
    let conn = Connection::open(&journal_path)?;
    match current_work_state(&conn)? {
        model::WorkState::Running => println!("Already running."),
        model::WorkState::Stopped => {println!("Running!"); switchWorkState(&conn)?;},
        model::WorkState::NoPendingTasks => println!("No pending tasks!")
    }
    return Ok(())
}

pub fn stop(journal_path: PathBuf) -> Result<()> {
    let conn = Connection::open(&journal_path)?;
    match current_work_state(&conn)? {
        model::WorkState::Stopped => println!("Not running."),
        model::WorkState::Running => {println!("Stopped."); switchWorkState(&conn)?;},
        model::WorkState::NoPendingTasks => println!("No pending tasks!")
    }
    return Ok(())
}

fn switchWorkState(db: &Connection) -> Result<()> {
    db.execute("INSERT INTO work (day, timestamp) VALUES(DATE('now', 'localtime'), CURRENT_TIMESTAMP)",
               [])?;
    return Ok(());
}

pub fn list(journal_path: PathBuf) -> Result<()> {
    let mut table = Table::new();

    let conn = Connection::open(&journal_path)?;


    // NOT STARTED
    let mut stmt = conn.prepare("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_time FROM task WHERE day = DATE('now','localtime') ORDER BY position")?;
    let task_iter = stmt.query_map([], |row| {
        return task_from_row(row);
    })?;

    let mut cumulated_todo_duration = 0i64;
    let local_time: DateTime<Local> = Local::now();

    table.add_row(row!["id", "task", "expected duration", "diff.", "expected end time", "end time"]);
    for task in task_iter {
        let task = task.unwrap();
        if task.finished_at == None {
            cumulated_todo_duration += i64::from(task.estimated_time);
        }

        table.add_row(row![task.fmt_position(),
                           task.fmt_description(),
                           task.fmt_estimated_time(),
                           "",
                           (local_time + Duration::seconds(i64::from(cumulated_todo_duration))).format("%T").to_string(),
                           task.fmt_finished_at()
        ]);
    }

    table.printstd();

    match current_work_state(&conn)? {
        model::WorkState::NoPendingTasks => println!("No pending tasks."),
        model::WorkState::Running => println!("Current state: Running."),
        model::WorkState::Stopped => println!("Current state: Stopped."),
    }

    let stopped_ranges = stopped_ranges(&conn)?;
    for range in stopped_ranges {
        match range.1 {
            Some(end) => println!("{} - {}",range.0, end),
            None => println!("{} - {}",range.0, "-")
        }
    }

    Ok(())
}

// get a task from a row in this order: day, description, position, created_at, started_at, finished_at, estimated_time
fn task_from_row(row: &Row) -> Result<model::Task> {
    return Ok(model::Task {
        id: row.get(0)?,
        day: row.get(1)?,
        description: row.get(2)?,
        position: row.get::<_,u32>(3)?,
        created_at: row.get::<_, DateTime<Local>>(4)?,
        started_at: row.get::<_,DateTime<Local>>(5).ok(),
        finished_at: row.get::<_, DateTime<Local>>(6).ok(),
        estimated_time: row.get::<_,u32>(7)?,
    })
}

fn current_task(db: &Connection) -> Result<model::Task> {
    let task = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_time FROM task WHERE day = DATE('now','localtime') AND started_at IS NULL ORDER BY position LIMIT 1", [], |row| task_from_row(row))?;
    return Ok(task);
}

fn pending_tasks_count(db: &Connection) -> Result<usize> {
    let count = db.query_row("SELECT count(*) FROM task WHERE day = DATE('now','localtime') AND finished_at IS NULL", [], |row| row.get::<_,usize>(0))?;
    return Ok(count);
}


fn task_at(db: &Connection, position: u32) -> Result<model::Task> {
    let task = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_time FROM task WHERE day = DATE('now','localtime') AND position = ?1", params![position], |row| task_from_row(row))?;
    return Ok(task);
}

fn current_work_state(db: &Connection) -> Result<model::WorkState> {
    if pending_tasks_count(db)? == 0 {
        return Ok(model::WorkState::NoPendingTasks)
    }

    let switchs_count = db.query_row("SELECT count(*) FROM work WHERE day = DATE('now','localtime') ", [], |row| row.get::<_,usize>(0))?;

    if switchs_count % 2 != 0 {
        return Ok(model::WorkState::Running);
    } else {
        return Ok(model::WorkState::Stopped);
    }
}

// Returns a slice of ranges defining the times where work has been stopped.
fn stopped_ranges(db: &Connection) -> Result<Vec<(DateTime<Local>,Option<DateTime<Local>>)>> {
    let mut stmt = db.prepare("SELECT timestamp FROM work WHERE day = DATE('now','localtime') ORDER BY id ASC")?;
    let mut state_changes_iter = stmt.query_map([], |row| {
        return row.get::<_, DateTime<Local>>(0)
    })?;

    // skip the first start
    state_changes_iter.next();
    let mut current_state = model::WorkState::Running;
    let mut current_start_date: Option<DateTime<Local>> = None;
    let mut ranges : Vec<(DateTime<Local>,Option<DateTime<Local>>)> = Vec::new();
    for state_change in state_changes_iter {
        let change = state_change.unwrap();
        if matches!(current_state,model::WorkState::Running) {
            current_start_date = Some(change);
            current_state = model::WorkState::Stopped
        } else {
            ranges.push((current_start_date.unwrap(), Some(change)));
            current_state = model::WorkState::Running;
        }
    }
    // add an open ended range if stopped
    if matches!(current_state,model::WorkState::Stopped) {
        ranges.push((current_start_date.unwrap(), None));
    }
    return Ok(ranges);
}

// Returns the duration of the overlap between two ranges. Ranges can have an open end, but no open start.
// If both ranges are open ended, "end" is used as the limit to calculate the duration. "end" should be bigger than both starts.
fn overlap(range1: (DateTime<Local>, Option<DateTime<Local>>),range2: (DateTime<Local>, Option<DateTime<Local>>), end: DateTime<Local>) -> Duration {
    // both open ranges
    if range2.1.is_none() && range1.1.is_none() {
        if range1.0 > range2.0 {
            return end - range1.0
        } else {
            return end - range2.0
        }
    }

    // range1 fully contains range2
    if range2.1.is_some() && (range1.0 <= range2.0) && (range1.1.is_none() || (range1.1.unwrap() >= range2.0)) {
        return range2.1.unwrap() - range2.0;
    }
    // range2 fully contains range1
    if range1.1.is_some() && (range2.0 <= range1.0) && (range2.1.is_none() || (range2.1.unwrap() >= range1.0)) {
        return range1.1.unwrap() - range1.0;
    }
    // range1 ends inside range2
    if range1.1.is_some() && (range1.1.unwrap() >= range2.0) && (range2.1.is_none() || (range2.1.unwrap() >= range1.1.unwrap())) {
        return range1.1.unwrap() - range2.0;
    }
    // range2 ends inside range1
    if range2.1.is_some() && (range2.1.unwrap() >= range1.0) && (range1.1.is_none() || (range1.1.unwrap() >= range2.1.unwrap())) {
        return range2.1.unwrap() - range1.0;
    }
    // no overlap
    return Duration::seconds(0);
}
