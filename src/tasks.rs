use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Task {
    pub id: u32,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub day: String,
    pub position: u32,
    pub estimated_time: u32, // in seconds
}
