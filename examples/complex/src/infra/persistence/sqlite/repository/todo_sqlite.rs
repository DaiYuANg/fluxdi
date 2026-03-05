use std::sync::Arc;

use crate::core::domain::todo::{Todo, TodoRepository};
use crate::infra::persistence::sqlite::SqliteClient;
use rusqlite::{OptionalExtension, params};

pub struct TodoSqliteRepository {
    sqlite_client: Arc<SqliteClient>,
}

impl TodoSqliteRepository {
    pub fn new(sqlite_client: Arc<SqliteClient>) -> Self {
        Self { sqlite_client }
    }
}

#[async_trait::async_trait]
impl TodoRepository for TodoSqliteRepository {
    async fn get_all(&self) -> Result<Vec<Todo>, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let mut statement = connection
            .prepare("SELECT id, title, description, completed FROM todos")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = statement
            .query_map([], |row| {
                Ok(Todo {
                    id: row.get::<_, i64>(0)? as u32,
                    title: row.get::<_, String>(1)?,
                    description: row.get::<_, String>(2)?,
                    completed: row.get::<_, i64>(3)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query todos: {}", e))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read todos: {}", e))
    }

    async fn get_by_id(&self, id: u32) -> Result<Option<Todo>, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let mut statement = connection
            .prepare("SELECT id, title, description, completed FROM todos WHERE id = ?")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        statement
            .query_row(params![id as i64], |row| {
                Ok(Todo {
                    id: row.get::<_, i64>(0)? as u32,
                    title: row.get::<_, String>(1)?,
                    description: row.get::<_, String>(2)?,
                    completed: row.get::<_, i64>(3)? != 0,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to fetch todo by id: {}", e))
    }

    async fn create(
        &self,
        user_id: u32,
        title: String,
        description: String,
    ) -> Result<Todo, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        connection
            .execute(
                "INSERT INTO todos (user_id, title, description, completed) VALUES (?, ?, ?, 0)",
                params![user_id as i64, title.as_str(), description.as_str()],
            )
            .map_err(|e| format!("Failed to execute insert: {}", e))?;

        let id = connection.last_insert_rowid() as u32;

        Ok(Todo {
            id,
            title,
            description,
            completed: false,
        })
    }

    async fn update_status(&self, id: u32, completed: bool) -> Result<Option<Todo>, String> {
        let updated = {
            let connection = self
                .sqlite_client
                .connection()
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let affected = connection
                .execute(
                    "UPDATE todos SET completed = ? WHERE id = ?",
                    params![if completed { 1i64 } else { 0i64 }, id as i64],
                )
                .map_err(|e| format!("Failed to execute update: {}", e))?;

            affected > 0
        };

        if updated {
            self.get_by_id(id).await
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, id: u32) -> Result<bool, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let affected = connection
            .execute("DELETE FROM todos WHERE id = ?", params![id as i64])
            .map_err(|e| format!("Failed to execute delete: {}", e))?;

        Ok(affected > 0)
    }
}
