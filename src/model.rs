use chrono::{DateTime, Local};

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
