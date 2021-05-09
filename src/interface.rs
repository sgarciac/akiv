use std::path::PathBuf;
use chrono::{DateTime, Duration, Local};
use rusqlite::{params, Connection, Result, Row};
use crate::model;
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
    Ok(())
}

pub fn add_task(journal_path: PathBuf, description: String, estimated_time: u64, at: Option<u32>) -> Result<()> {
    let conn = Connection::open(&journal_path)?;

    // Let mut stmt = c5onn.prepare()?;
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

        let mut finished_at_string : String = "".to_string();
        if let Some(finished_at) = task.finished_at {
            finished_at_string = finished_at.format("%T").to_string();
        }


        table.add_row(row![task.position,
                           task.description,
                           format_duration(STDDuration::from_secs(u64::from(task.estimated_time))),
                           "",
                           (local_time + Duration::seconds(i64::from(cumulated_todo_duration))).format("%T").to_string(),
                           finished_at_string
        ]);
    }
    table.printstd();
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
    let row = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_time FROM task WHERE day = DATE('now','localtime') AND started_at IS NULL ORDER BY position LIMIT 1", [], |row| task_from_row(row))?;
    return Ok(row);
}

fn task_at(db: &Connection, position: u32) -> Result<model::Task> {
    let row = db.query_row("SELECT id, day, description, position, created_at, started_at, finished_at, estimated_time FROM task WHERE day = DATE('now','localtime') AND position = ?1", params![position], |row| task_from_row(row))?;
    return Ok(row);
}
