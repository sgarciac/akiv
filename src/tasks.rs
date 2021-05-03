use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Task {
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub day: String,
    pub position: i32,
    pub estimated_time: usize, // in seconds
}
