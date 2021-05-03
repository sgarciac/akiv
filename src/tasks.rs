use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Task {
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub day: String,
    pub position: usize,
    pub estimated_time: usize, // in seconds
}

impl Task {
    pub fn new(description: String, estimated_time: usize) -> Task {
        let created_at: DateTime<Utc> = Utc::now();
        Task { description,
               created_at,
               estimated_time,
               day: "2021-04-02".to_string(),
               position: 1,
               started_at: None,
               finished_at: None
        }
    }
}
