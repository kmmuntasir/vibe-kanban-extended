use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct KanbanTag {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub color: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

const DEFAULT_TAGS: &[(&str, &str)] = &[
    ("bug", "355 65% 53%"),
    ("feature", "124 82% 30%"),
    ("documentation", "205 100% 40%"),
    ("enhancement", "181 72% 78%"),
];

impl KanbanTag {
    pub async fn create_defaults_for_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let mut tags = Vec::with_capacity(DEFAULT_TAGS.len());
        for (name, color) in DEFAULT_TAGS {
            let tag = Self::create(pool, project_id, name, color).await?;
            tags.push(tag);
        }
        Ok(tags)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanTag,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      color,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_tags
               WHERE project_id = $1
               ORDER BY name ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanTag,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      color,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_tags
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        project_id: Uuid,
        name: &str,
        color: &str,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            KanbanTag,
            r#"INSERT INTO kanban_tags (id, project_id, name, color)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         color,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            name,
            color
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            KanbanTag,
            r#"UPDATE kanban_tags
               SET name = COALESCE($2, name),
                   color = COALESCE($3, color),
                   updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         color,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            color
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM kanban_tags WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
