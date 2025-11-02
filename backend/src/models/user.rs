use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::Type, serde::Serialize, serde::Deserialize)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl User {
    pub async fn find_by_username(pool: &PgPool, username: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn create(
        pool: &PgPool,
        username: &str,
        password_hash: &str,
        role: UserRole,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (username, password_hash, role)
             VALUES ($1, $2, $3)
             RETURNING id, username, password_hash, role, created_at",
        )
        .bind(username)
        .bind(password_hash)
        .bind(role)
        .fetch_one(pool)
        .await
    }

    #[allow(dead_code)]
    pub async fn update_password(
        pool: &PgPool,
        user_id: Uuid,
        new_password_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
            .bind(new_password_hash)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
