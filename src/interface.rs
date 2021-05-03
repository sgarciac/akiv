use std::path::PathBuf;
use rusqlite::{params, Connection, Result};
use crate::tasks;

pub fn init_journal(journal_path: PathBuf) -> Result<()> {
    let conn = Connection::open(journal_path)?;
    conn.execute(
        "CREATE TABLE if not exists task (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  day             TEXT NOT NULL,
                  description     TEXT NOT NULL,
                  position        INTEGER NOT NULL UNIQUE,
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

pub fn add_task(journal_path: PathBuf, description: String, estimated_time: usize) -> Result<()> {
    let conn = Connection::open(journal_path)?;

    //let mut stmt = conn.prepare()?;
    let position = conn.query_row("SELECT count(*) from task where day = DATE()", [], |row| row.get::<_,i32>(0))?;
    //println!(position);

    conn.execute("INSERT INTO task (day, description, position, created_at, estimated_time)
VALUES(CURRENT_DATE, ?1, ?2, CURRENT_TIME, ?3)", params![description, position, estimated_time])?;
    Ok(())
}
//pub fn complete_task(journal_path: PathBuf, task_position: usize) -> Result<()> { ... }
//pub fn list_tasks(journal_path: PathBuf) -> Result<()> { ... }
