use std::path::PathBuf;
use chrono::{DateTime, Utc, Duration, Local};
use rusqlite::{params, Connection, Result};
use crate::tasks;
use std::time::Duration as STDDuration;
use humantime::format_duration;
use prettytable::{Table};

pub fn init_journal(journal_path: PathBuf) -> Result<()> {
    let conn = Connection::open(journal_path)?;
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

pub fn add_task(journal_path: PathBuf, description: String, estimated_time: u64) -> Result<()> {
    let conn = Connection::open(journal_path)?;

    //let mut stmt = conn.prepare()?;
    let position = conn.query_row("SELECT count(*) from task where day = DATE('now', 'localtime')", [], |row| row.get::<_,usize>(0))?;
    //println!(position);

    conn.execute("INSERT INTO task (day, description, position, created_at, estimated_time)
VALUES(DATE('now', 'localtime'), ?1, ?2, CURRENT_TIMESTAMP, ?3)", params![description, position, estimated_time])?;
    Ok(())
}


pub fn remove_task(journal_path: PathBuf, position: usize) -> Result<()> {
    let conn = Connection::open(journal_path)?;

    conn.execute("DELETE FROM task where day = DATE('now', 'localtime') and position = ?1", params![position])?;
    Ok(())
}

pub fn list(journal_path: PathBuf) -> Result<()> {
    let mut table = Table::new();

    let conn = Connection::open(journal_path)?;

    // NOT STARTED
    let mut stmt = conn.prepare("SELECT day, description, position, created_at, started_at, finished_at, estimated_time FROM task WHERE day = DATE('now','localtime') ORDER BY position")?;
    let task_iter = stmt.query_map([], |row| {
        Ok(tasks::Task {
            day: row.get(0)?,
            description: row.get(1)?,
            position: row.get::<_,u32>(2)?,
            created_at: row.get::<_, DateTime<Utc>>(3)?,
            started_at: row.get::<_,DateTime<Utc>>(4).ok(),
            finished_at: row.get::<_, DateTime<Utc>>(5).ok(),
            estimated_time: row.get::<_,u32>(6)?,

        })
    })?;

    let mut cumulated_todo_duration = 0i64;
    let local_time: DateTime<Local> = Local::now();

    table.add_row(row!["position", "task", "exp. duration", "gap", "exp. end time", "end time"]);
    for task in task_iter {
        let task = task.unwrap();
        if task.finished_at == None {
            cumulated_todo_duration += i64::from(task.estimated_time);
        }


        table.add_row(row![task.position,
                           task.description,
                           format_duration(STDDuration::from_secs(u64::from(task.estimated_time))),
                           "",
                           (local_time + Duration::seconds(i64::from(cumulated_todo_duration))).format("%T").to_string()
        ]);
    }
    table.printstd();
    Ok(())
}
