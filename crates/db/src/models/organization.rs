use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub is_personal: bool,
    pub issue_prefix: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    pub async fn ensure_default_org(pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        let existing = sqlx::query_as!(
            Organization,
            r#"SELECT id as "id!: Uuid",
                      name,
                      slug,
                      is_personal as "is_personal: bool",
                      issue_prefix,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM organizations
               WHERE slug = 'local'"#
        )
        .fetch_optional(pool)
        .await?;

        if let Some(org) = existing {
            return Ok(org);
        }

        let id = Uuid::new_v4();
        let org = sqlx::query_as!(
            Organization,
            r#"INSERT INTO organizations (id, name, slug, is_personal, issue_prefix)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id as "id!: Uuid",
                         name,
                         slug,
                         is_personal as "is_personal: bool",
                         issue_prefix,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            "My Workspace",
            "local",
            false,
            "VK"
        )
        .fetch_one(pool)
        .await?;

        Ok(org)
    }

    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Organization,
            r#"SELECT id as "id!: Uuid",
                      name,
                      slug,
                      is_personal as "is_personal: bool",
                      issue_prefix,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM organizations
               ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Organization,
            r#"SELECT id as "id!: Uuid",
                      name,
                      slug,
                      is_personal as "is_personal: bool",
                      issue_prefix,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM organizations
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        name: &str,
        slug: &str,
        issue_prefix: &str,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Organization,
            r#"INSERT INTO organizations (id, name, slug, issue_prefix)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!: Uuid",
                         name,
                         slug,
                         is_personal as "is_personal: bool",
                         issue_prefix,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            slug,
            issue_prefix
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update_name(pool: &SqlitePool, id: Uuid, name: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Organization,
            r#"UPDATE organizations
               SET name = $2, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         name,
                         slug,
                         is_personal as "is_personal: bool",
                         issue_prefix,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name
        )
        .fetch_one(pool)
        .await
    }
}
