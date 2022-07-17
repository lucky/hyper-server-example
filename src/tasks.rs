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

pub async fn insert_task(client: Arc<Client>, task: Task) -> Result<(), tokio_postgres::Error> {
    let client = &*client;
    client
        .query(
            "INSERT INTO tasks (person, description) VALUES ($1, $2)",
            &[&task.person, &task.description],
        )
        .await?;

    Ok(())
}

pub async fn select_all_tasks(client: Arc<Client>) -> Result<Vec<Task>, tokio_postgres::Error> {
    let client = &*client;
    let rows = client
        .query("SELECT id, person, description FROM tasks", &[])
        .await?;

    let mut tasks: Vec<Task> = Vec::new();
    for row in rows {
        let id: i32 = row.get(0);
        let person: String = row.get(1);
        let description: String = row.get(2);
        tasks.push(Task {
            id,
            person,
            description,
        });
    }

    Ok(tasks)
}
