use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct IssueTag {
    pub id: Uuid,
    pub issue_id: Uuid,
    pub tag_id: Uuid,
}

impl IssueTag {
    pub async fn find_by_issue(
        pool: &SqlitePool,
        issue_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            IssueTag,
            r#"SELECT id as "id!: Uuid",
                      issue_id as "issue_id!: Uuid",
                      tag_id as "tag_id!: Uuid"
               FROM issue_tags
               WHERE issue_id = $1"#,
            issue_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        issue_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            IssueTag,
            r#"INSERT INTO issue_tags (id, issue_id, tag_id)
               VALUES ($1, $2, $3)
               RETURNING id as "id!: Uuid",
                         issue_id as "issue_id!: Uuid",
                         tag_id as "tag_id!: Uuid""#,
            Uuid::new_v4(),
            issue_id,
            tag_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM issue_tags WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
