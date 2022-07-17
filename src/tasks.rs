use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_postgres::Client;

#[derive(Serialize, Deserialize)]
pub struct Task {
    #[serde(default)]
    pub id: i32,
    pub person: String,
    pub description: String,
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
    let client = &*client;
    let task_vec = client
        .query("SELECT id, person, description FROM tasks", &[])
        .await?
        .iter()
        .map(|row| {
            let id: i32 = row.get(0);
            let person: String = row.get(1);
            let description: String = row.get(2);
            Task {
                id,
                person,
                description,
            }
        })
        .collect();
    Ok(task_vec)
}
