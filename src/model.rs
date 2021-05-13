use chrono::{DateTime, Local, Duration};
use humantime::format_duration;
use std::time::Duration as STDDuration;

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
