use std::sync::Arc;

use crate::core::domain::user::{User, UserRepository};
use crate::infra::persistence::sqlite::SqliteClient;
use rusqlite::{OptionalExtension, params};

pub struct UserSqliteRepository {
    sqlite_client: Arc<SqliteClient>,
}

impl UserSqliteRepository {
    pub fn new(sqlite_client: Arc<SqliteClient>) -> Self {
        Self { sqlite_client }
    }
}

#[async_trait::async_trait]
impl UserRepository for UserSqliteRepository {
    async fn get_all(&self) -> Result<Vec<User>, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let mut statement = connection
            .prepare("SELECT id, name, email FROM users")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = statement
            .query_map([], |row| {
                Ok(User {
                    id: row.get::<_, i64>(0)? as u32,
                    name: row.get::<_, String>(1)?,
                    email: row.get::<_, String>(2)?,
                })
            })
            .map_err(|e| format!("Failed to query users: {}", e))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read users: {}", e))
    }

    async fn get_by_id(&self, id: u32) -> Result<Option<User>, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let mut statement = connection
            .prepare("SELECT id, name, email FROM users WHERE id = ?")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        statement
            .query_row(params![id as i64], |row| {
                Ok(User {
                    id: row.get::<_, i64>(0)? as u32,
                    name: row.get::<_, String>(1)?,
                    email: row.get::<_, String>(2)?,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to fetch user by id: {}", e))
    }

    async fn create(&self, name: String, email: String) -> Result<User, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        connection
            .execute(
                "INSERT INTO users (name, email) VALUES (?, ?)",
                params![name.as_str(), email.as_str()],
            )
            .map_err(|e| format!("Failed to execute insert: {}", e))?;

        let id = connection.last_insert_rowid() as u32;

        Ok(User { id, name, email })
    }

    async fn delete(&self, id: u32) -> Result<bool, String> {
        let connection = self
            .sqlite_client
            .connection()
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let affected = connection
            .execute("DELETE FROM users WHERE id = ?", params![id as i64])
            .map_err(|e| format!("Failed to execute delete: {}", e))?;

        Ok(affected > 0)
    }
}
