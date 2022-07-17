use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_postgres::Client;

#[derive(Serialize, Deserialize)]
pub struct Task {
    #[serde(default)]
    pub id: i32,
    pub person: String,
    pub description: String,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub async fn insert(client: Arc<Client>, task: Task) -> Result<(), tokio_postgres::Error> {
    client
        .query(
            "INSERT INTO tasks (person, description) VALUES ($1, $2)",
            &[&task.person, &task.description],
        )
        .await?;
    Ok(())
}

pub async fn select_all(client: Arc<Client>) -> Result<Vec<Task>, tokio_postgres::Error> {
    let task_vec = client
        .query("SELECT id, person, description, created_at, completed_at FROM tasks ORDER BY created_at DESC", &[])
        .await?
        .iter()
        .map(|row| {
            let id: i32 = row.get(0);
            let person: String = row.get(1);
            let description: String = row.get(2);
            let created_at: DateTime<Utc> = row.get(3);
            let completed_at: Option<DateTime<Utc>> = row.get(4);
            Task {
                id,
                person,
                description,
                created_at,
                completed_at,
            }
        })
        .collect();
    Ok(task_vec)
}
