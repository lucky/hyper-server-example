use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Pool};
use serde::{Deserialize, Serialize};

use crate::errors::Errors;

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

pub struct TaskDAO {
    pool: Pool,
}

impl TaskDAO {
    pub fn new(pool: Pool) -> TaskDAO {
        TaskDAO { pool }
    }

    async fn get_client(&self) -> Result<Client, Errors> {
        self.pool.get().await.map_err(Errors::Pool)
    }

    pub async fn insert(&self, task: Task) -> Result<(), Errors> {
        let client = self.get_client().await?;
        client
            .execute(
                "INSERT INTO tasks (person, description) VALUES ($1, $2)",
                &[&task.person, &task.description],
            )
            .await
            .map_err(Errors::Database)?;
        Ok(())
    }

    pub async fn select_all(&self) -> Result<Vec<Task>, Errors> {
        let client = self.get_client().await?;
        let task_vec = client
        .query("SELECT id, person, description, created_at, completed_at FROM tasks ORDER BY created_at DESC", &[])
        .await.map_err(Errors::Database)?
        .iter()
        .map(std::convert::Into::into)
        .collect();
        Ok(task_vec)
    }

    pub async fn get(&self, id: i32) -> Result<Option<Task>, Errors> {
        let client = self.get_client().await?;
        let row = client
        .query_opt("SELECT id, person, description, created_at, completed_at FROM tasks WHERE id = $1 LIMIT 1", &[&id])
        .await.map_err(Errors::Database)?;
        Ok(row.map(|row| (&row).into()))
    }

    pub async fn mark_complete(&self, id: i32) -> Result<bool, Errors> {
        let client = self.get_client().await?;
        let result = client
            .execute(
                "UPDATE tasks SET completed_at = NOW() WHERE id = $1 AND completed_at IS NULL",
                &[&id],
            )
            .await
            .map_err(Errors::Database)?;
        Ok(1 == result)
    }
}
