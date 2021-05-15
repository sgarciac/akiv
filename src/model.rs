use chrono::{DateTime, Local, Duration};
use rusqlite::{params, Connection, Row};
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
    )?;

    db.execute(
        "CREATE UNIQUE INDEX day_position ON task (day, position)",
        [],
    )?;

    db.execute(
        "CREATE TABLE if not exists work (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  day             TEXT NOT NULL,
                  timestamp       TEXT
                  )",
        [],
    )?;

    db.execute("CREATE INDEX day_index ON work (day)", [])?;

    Ok(())
}

pub fn tasks_count(db: &Connection) -> Result<u32> {
    let count = db.query_row(
        "SELECT count(*) from task where day = DATE('now', 'localtime')",
        [],
        |row| row.get::<_, u32>(0),
    )?;
    Ok(count)
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
