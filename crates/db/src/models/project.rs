use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub default_agent_working_dir: Option<String>,
    pub remote_project_id: Option<Uuid>,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub issue_counter: i32,
    pub organization_id: Option<Uuid>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

impl Project {
    pub async fn create(
        conn: &mut SqliteConnection,
        id: Uuid,
        name: &str,
        color: &str,
        organization_id: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Project,
            r#"INSERT INTO projects (id, name, color, issue_counter, organization_id)
               VALUES ($1, $2, $3, 0, $4)
               RETURNING id as "id!: Uuid",
                         name,
                         default_agent_working_dir,
                         remote_project_id as "remote_project_id: Uuid",
                         color,
                         issue_counter as "issue_counter!: i32",
                         organization_id as "organization_id: Uuid",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            color,
            organization_id
        )
        .fetch_one(conn)
        .await
    }

    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Project,
            r#"SELECT id as "id!: Uuid",
                      name,
                      default_agent_working_dir,
                      remote_project_id as "remote_project_id: Uuid",
                      color,
                      issue_counter as "issue_counter!: i32",
                      organization_id as "organization_id: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM projects
               ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Project,
            r#"SELECT id as "id!: Uuid",
                      name,
                      default_agent_working_dir,
                      remote_project_id as "remote_project_id: Uuid",
                      color,
                      issue_counter as "issue_counter!: i32",
                      organization_id as "organization_id: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM projects
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Project,
            r#"SELECT id as "id!: Uuid",
                      name,
                      default_agent_working_dir,
                      remote_project_id as "remote_project_id: Uuid",
                      color,
                      issue_counter as "issue_counter!: i32",
                      organization_id as "organization_id: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM projects
               WHERE organization_id = $1
               ORDER BY created_at DESC"#,
            organization_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn set_remote_project_id(
        pool: &SqlitePool,
        id: Uuid,
        remote_project_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE projects
               SET remote_project_id = $2
               WHERE id = $1"#,
            id,
            remote_project_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_kanban_fields(
        pool: &SqlitePool,
        id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
        organization_id: Option<Option<Uuid>>,
    ) -> Result<Self, sqlx::Error> {
        let update_org = organization_id.is_some();
        let org_value = organization_id.flatten();

        sqlx::query_as!(
            Project,
            r#"UPDATE projects
               SET name = COALESCE($2, name),
                   color = COALESCE($3, color),
                   organization_id = CASE WHEN $4 THEN $5 ELSE organization_id END,
                   updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         name,
                         default_agent_working_dir,
                         remote_project_id as "remote_project_id: Uuid",
                         color,
                         issue_counter as "issue_counter!: i32",
                         organization_id as "organization_id: Uuid",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            color,
            update_org,
            org_value
        )
        .fetch_one(pool)
        .await
    }
}
