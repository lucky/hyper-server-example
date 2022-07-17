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
        .map(task_from_row)
        .collect();
    Ok(task_vec)
}

pub async fn get(id: i32, client: Arc<Client>) -> Result<Option<Task>, tokio_postgres::Error> {
    // I was going to use `query_one`, but if it returns anything other than 1
    // row, it's considered a "programming error":
    // https://github.com/sfackler/rust-postgres/issues/790#issuecomment-863149386
    // While I understand the reasoning, it would make sense to have a method
    // like `try_query` that returns a `Result<Option<T>, Error>`. C'est la vie.
    let rows = client
        .query("SELECT id, person, description, created_at, completed_at FROM tasks WHERE id = $1 LIMIT 1", &[&id])
        .await?;

    if rows.is_empty() {
        Ok(None)
    } else {
        Ok(Some(task_from_row(&rows[0])))
    }
}

fn task_from_row(row: &tokio_postgres::Row) -> Task {
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

pub async fn mark_complete(id: i32, client: Arc<Client>) -> Result<bool, tokio_postgres::Error> {
    let result = client
        .execute(
            "UPDATE tasks SET completed_at = NOW() WHERE id = $1 AND completed_at IS NULL",
            &[&id],
        )
        .await?;
    Ok(1 == result)
}
