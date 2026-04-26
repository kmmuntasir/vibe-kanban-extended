use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct IssueAssignee {
    pub id: Uuid,
    pub issue_id: Uuid,
    pub user_id: Uuid,
    #[ts(type = "Date")]
    pub assigned_at: DateTime<Utc>,
}

impl IssueAssignee {
    pub async fn find_by_issue(
        pool: &SqlitePool,
        issue_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            IssueAssignee,
            r#"SELECT id as "id!: Uuid",
                      issue_id as "issue_id!: Uuid",
                      user_id as "user_id!: Uuid",
                      assigned_at as "assigned_at!: DateTime<Utc>"
               FROM issue_assignees
               WHERE issue_id = $1"#,
            issue_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        issue_id: Uuid,
        user_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            IssueAssignee,
            r#"INSERT INTO issue_assignees (id, issue_id, user_id)
               VALUES ($1, $2, $3)
               RETURNING id as "id!: Uuid",
                         issue_id as "issue_id!: Uuid",
                         user_id as "user_id!: Uuid",
                         assigned_at as "assigned_at!: DateTime<Utc>""#,
            Uuid::new_v4(),
            issue_id,
            user_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM issue_assignees WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
