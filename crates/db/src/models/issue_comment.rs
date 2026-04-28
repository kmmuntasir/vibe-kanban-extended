use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct IssueComment {
    pub id: Uuid,
    pub issue_id: Uuid,
    pub author_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub message: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

impl IssueComment {
    pub async fn find_by_issue(
        pool: &SqlitePool,
        issue_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            IssueComment,
            r#"SELECT id as "id!: Uuid",
                      issue_id as "issue_id!: Uuid",
                      author_id as "author_id: Uuid",
                      parent_id as "parent_id: Uuid",
                      message,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM issue_comments
               WHERE issue_id = $1
               ORDER BY created_at ASC"#,
            issue_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        issue_id: Uuid,
        author_id: Option<Uuid>,
        parent_id: Option<Uuid>,
        message: &str,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            IssueComment,
            r#"INSERT INTO issue_comments (id, issue_id, author_id, parent_id, message)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id as "id!: Uuid",
                         issue_id as "issue_id!: Uuid",
                         author_id as "author_id: Uuid",
                         parent_id as "parent_id: Uuid",
                         message,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            issue_id,
            author_id,
            parent_id,
            message
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(pool: &SqlitePool, id: Uuid, message: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            IssueComment,
            r#"UPDATE issue_comments
               SET message = $2, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         issue_id as "issue_id!: Uuid",
                         author_id as "author_id: Uuid",
                         parent_id as "parent_id: Uuid",
                         message,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            message
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM issue_comments WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
