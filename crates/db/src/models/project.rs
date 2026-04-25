use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub default_agent_working_dir: Option<String>,
    pub remote_project_id: Option<Uuid>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct KanbanProject {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub color: String,
    pub issue_counter: i32,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateKanbanProject {
    pub id: Option<Uuid>,
    pub organization_id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateKanbanProject {
    pub name: Option<String>,
    pub color: Option<String>,
}

impl Project {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Project,
            r#"SELECT id as "id!: Uuid",
                      name,
                      default_agent_working_dir,
                      remote_project_id as "remote_project_id: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM projects
               ORDER BY created_at DESC"#
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
}

impl KanbanProject {
    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanProject,
            r#"SELECT id as "id!: Uuid",
                      organization_id as "organization_id: Uuid",
                      name,
                      color,
                      issue_counter as "issue_counter!: i32",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM projects
               WHERE organization_id = $1
               ORDER BY created_at ASC"#,
            organization_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanProject,
            r#"SELECT id as "id!: Uuid",
                      organization_id as "organization_id: Uuid",
                      name,
                      color,
                      issue_counter as "issue_counter!: i32",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM projects
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateKanbanProject,
    ) -> Result<Self, sqlx::Error> {
        let id = data.id.unwrap_or_else(Uuid::new_v4);
        sqlx::query_as!(
            KanbanProject,
            r#"INSERT INTO projects (id, organization_id, name, color)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!: Uuid",
                         organization_id as "organization_id: Uuid",
                         name,
                         color,
                         issue_counter as "issue_counter!: i32",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.organization_id,
            data.name,
            data.color
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update_project(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateKanbanProject,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_deref().unwrap_or(&existing.name);
        let color = data.color.as_deref().unwrap_or(&existing.color);

        sqlx::query_as!(
            KanbanProject,
            r#"UPDATE projects SET name = $2, color = $3, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         organization_id as "organization_id: Uuid",
                         name,
                         color,
                         issue_counter as "issue_counter!: i32",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            color
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete_project(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM projects WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_issue_prefix(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<String, sqlx::Error> {
        let row = sqlx::query_scalar!(
            r#"SELECT o.issue_prefix as "issue_prefix!: String"
               FROM projects p
               JOIN organizations o ON o.id = p.organization_id
               WHERE p.id = $1"#,
            project_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row.unwrap_or_else(|| "ISS".to_string()))
    }
}
