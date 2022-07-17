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
        .execute(
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
        .map(|row| row.into())
        .collect();
    Ok(task_vec)
}

pub async fn get(id: i32, client: Arc<Client>) -> Result<Option<Task>, tokio_postgres::Error> {
    let row = client
        .query_opt("SELECT id, person, description, created_at, completed_at FROM tasks WHERE id = $1 LIMIT 1", &[&id])
        .await?;
    Ok(row.map(|row| (&row).into()))
}

impl From<&tokio_postgres::Row> for Task {
    fn from(row: &tokio_postgres::Row) -> Self {
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
    }
}

pub async fn mark_complete(id: i32, client: Arc<Client>) -> Result<bool, tokio_postgres::Error> {
    let result = client
        .execute(
            "UPDATE tasks SET completed_at = NOW() WHERE id = $1 AND completed_at IS NULL",
            &[&id],
        )
        .await?;
    Ok(1 == result)
}
