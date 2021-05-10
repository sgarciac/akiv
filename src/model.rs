use chrono::{DateTime, Local};
use humantime::format_duration;
use std::time::Duration as STDDuration;

#[derive(Debug)]
pub struct Task {
    pub id: u32,
    pub description: String,
    pub created_at: DateTime<Local>,
    pub started_at: Option<DateTime<Local>>,
    pub finished_at: Option<DateTime<Local>>,
    pub day: String,
    pub position: u32,
    pub estimated_time: u32, // in seconds
}

pub trait TaskLabels {
    fn fmt_position(&self) -> String;
    fn fmt_description(&self) -> String;
    fn fmt_estimated_time(&self) -> String;
    fn fmt_finished_at(&self) -> String;
}

impl TaskLabels for Task {
    fn fmt_position(&self) -> String {
        return self.position.to_string();
    }

    fn fmt_description(&self) -> String {
        return self.description.to_string();
    }

    fn fmt_estimated_time(&self) -> String {
        return format_duration(STDDuration::from_secs(u64::from(self.estimated_time))).to_string()
    }

    fn fmt_finished_at(&self) -> String {
        let mut finished_at_string : String = "".to_string();
        if let Some(finished_at) = self.finished_at {
            finished_at_string = finished_at.format("%T").to_string();
        }
        return finished_at_string;
    }
}

#[derive(Debug)]
pub enum WorkState {
    Running,
    Stopped,
    NoPendingTasks
}
