use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ProjectStatus {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub color: String,
    pub sort_order: i32,
    #[serde(default)]
    pub hidden: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

const DEFAULT_STATUSES: &[(&str, &str, i32, bool)] = &[
    ("Backlog", "220 9% 46%", 0, true),
    ("To do", "217 91% 60%", 1, false),
    ("In progress", "38 92% 50%", 2, false),
    ("In review", "258 90% 66%", 3, false),
    ("Done", "142 71% 45%", 4, false),
    ("Cancelled", "0 84% 60%", 5, true),
];

impl ProjectStatus {
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectStatus,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      color,
                      sort_order as "sort_order!: i32",
                      hidden as "hidden: bool",
                      created_at as "created_at!: DateTime<Utc>"
               FROM project_statuses
               WHERE project_id = $1
               ORDER BY sort_order ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        project_id: Uuid,
        name: &str,
        color: &str,
        sort_order: i32,
        hidden: bool,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            ProjectStatus,
            r#"INSERT INTO project_statuses (id, project_id, name, color, sort_order, hidden)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         color,
                         sort_order as "sort_order!: i32",
                         hidden as "hidden: bool",
                         created_at as "created_at!: DateTime<Utc>""#,
            Uuid::new_v4(),
            project_id,
            name,
            color,
            sort_order,
            hidden
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
        sort_order: Option<i32>,
        hidden: Option<bool>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            ProjectStatus,
            r#"UPDATE project_statuses
               SET name = COALESCE($2, name),
                   color = COALESCE($3, color),
                   sort_order = COALESCE($4, sort_order),
                   hidden = COALESCE($5, hidden)
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         color,
                         sort_order as "sort_order!: i32",
                         hidden as "hidden: bool",
                         created_at as "created_at!: DateTime<Utc>""#,
            id,
            name,
            color,
            sort_order,
            hidden
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM project_statuses WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn create_defaults_for_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let mut statuses = Vec::with_capacity(DEFAULT_STATUSES.len());
        for (name, color, sort_order, hidden) in DEFAULT_STATUSES {
            let status = Self::create(pool, project_id, name, color, *sort_order, *hidden).await?;
            statuses.push(status);
        }
        Ok(statuses)
    }
}
