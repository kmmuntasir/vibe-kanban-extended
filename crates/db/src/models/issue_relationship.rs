use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct IssueRelationship {
    pub id: Uuid,
    pub issue_id: Uuid,
    pub related_issue_id: Uuid,
    pub relationship_type: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl IssueRelationship {
    pub async fn find_by_issue(
        pool: &SqlitePool,
        issue_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            IssueRelationship,
            r#"SELECT id as "id!: Uuid",
                      issue_id as "issue_id!: Uuid",
                      related_issue_id as "related_issue_id!: Uuid",
                      relationship_type,
                      created_at as "created_at!: DateTime<Utc>"
               FROM issue_relationships
               WHERE issue_id = $1"#,
            issue_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        issue_id: Uuid,
        related_issue_id: Uuid,
        relationship_type: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            IssueRelationship,
            r#"INSERT INTO issue_relationships (id, issue_id, related_issue_id, relationship_type)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!: Uuid",
                         issue_id as "issue_id!: Uuid",
                         related_issue_id as "related_issue_id!: Uuid",
                         relationship_type,
                         created_at as "created_at!: DateTime<Utc>""#,
            Uuid::new_v4(),
            issue_id,
            related_issue_id,
            relationship_type
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM issue_relationships WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
