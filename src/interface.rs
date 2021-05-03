use std::path::PathBuf;
use chrono::{DateTime, Utc};
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

pub fn add_task(journal_path: PathBuf, description: String, estimated_time: u64) -> Result<()> {
    let conn = Connection::open(journal_path)?;

    //let mut stmt = conn.prepare()?;
    let position = conn.query_row("SELECT count(*) from task where day = DATE()", [], |row| row.get::<_,usize>(0))?;
    //println!(position);

    conn.execute("INSERT INTO task (day, description, position, created_at, estimated_time)
VALUES(CURRENT_DATE, ?1, ?2, CURRENT_TIMESTAMP, ?3)", params![description, position, estimated_time])?;
    Ok(())
}

pub fn list(journal_path: PathBuf) -> Result<()> {
    let conn = Connection::open(journal_path)?;
    let mut stmt = conn.prepare("SELECT day, description, position, created_at, started_at, finished_at, estimated_time FROM task")?;
    let task_iter = stmt.query_map([], |row| {
        Ok(tasks::Task {
            day: row.get(0)?,
            description: row.get(1)?,
            position: row.get::<_,i32>(2)?,
            created_at: row.get::<_, DateTime<Utc>>(3)?,
            started_at: row.get::<_,DateTime<Utc>>(4).ok(),
            finished_at: row.get::<_, DateTime<Utc>>(5).ok(),
            estimated_time: row.get(6)?,

        })
    })?;
    for task in task_iter {
        println!("Found task {:?}", task.expect("Failed"));
    }

    Ok(())
}
