use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum IssuePriority {
    Urgent,
    High,
    Medium,
    Low,
}

impl IssuePriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Urgent => "urgent",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "urgent" => Some(Self::Urgent),
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Issue {
    pub id: Uuid,
    pub project_id: Uuid,
    pub issue_number: i32,
    pub simple_id: String,
    pub status_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub target_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sort_order: f64,
    pub parent_issue_id: Option<Uuid>,
    pub parent_issue_sort_order: Option<f64>,
    pub extension_metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIssue {
    pub id: Option<Uuid>,
    pub project_id: Uuid,
    pub status_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub target_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sort_order: f64,
    pub parent_issue_id: Option<Uuid>,
    pub parent_issue_sort_order: Option<f64>,
    pub extension_metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIssue {
    pub status_id: Option<Uuid>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub target_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sort_order: Option<f64>,
    pub parent_issue_id: Option<Uuid>,
    pub parent_issue_sort_order: Option<f64>,
    pub extension_metadata: Option<Value>,
}

impl Issue {
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Issue,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      issue_number as "issue_number!: i32",
                      simple_id,
                      status_id as "status_id!: Uuid",
                      title,
                      description,
                      priority,
                      start_date as "start_date: DateTime<Utc>",
                      target_date as "target_date: DateTime<Utc>",
                      completed_at as "completed_at: DateTime<Utc>",
                      sort_order as "sort_order!: f64",
                      parent_issue_id as "parent_issue_id: Uuid",
                      parent_issue_sort_order as "parent_issue_sort_order: f64",
                      extension_metadata as "extension_metadata!: Value",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM issues
               WHERE project_id = $1
               ORDER BY sort_order ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Issue,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      issue_number as "issue_number!: i32",
                      simple_id,
                      status_id as "status_id!: Uuid",
                      title,
                      description,
                      priority,
                      start_date as "start_date: DateTime<Utc>",
                      target_date as "target_date: DateTime<Utc>",
                      completed_at as "completed_at: DateTime<Utc>",
                      sort_order as "sort_order!: f64",
                      parent_issue_id as "parent_issue_id: Uuid",
                      parent_issue_sort_order as "parent_issue_sort_order: f64",
                      extension_metadata as "extension_metadata!: Value",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM issues
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateIssue, issue_prefix: &str) -> Result<Self, sqlx::Error> {
        let id = data.id.unwrap_or_else(Uuid::new_v4);
        let extension_metadata = data.extension_metadata.clone().unwrap_or(serde_json::json!({}));

        // Atomically increment issue_counter and generate simple_id
        let mut tx = pool.begin().await?;

        let row = sqlx::query_scalar!(
            r#"UPDATE projects
               SET issue_counter = issue_counter + 1
               WHERE id = $1
               RETURNING issue_counter as "issue_counter!: i32""#,
            data.project_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let issue_number = row;
        let simple_id = format!("{}-{}", issue_prefix, issue_number);

        let issue = sqlx::query_as!(
            Issue,
            r#"INSERT INTO issues (id, project_id, issue_number, simple_id, status_id, title,
                                  description, priority, start_date, target_date, completed_at,
                                  sort_order, parent_issue_id, parent_issue_sort_order, extension_metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         issue_number as "issue_number!: i32",
                         simple_id,
                         status_id as "status_id!: Uuid",
                         title,
                         description,
                         priority,
                         start_date as "start_date: DateTime<Utc>",
                         target_date as "target_date: DateTime<Utc>",
                         completed_at as "completed_at: DateTime<Utc>",
                         sort_order as "sort_order!: f64",
                         parent_issue_id as "parent_issue_id: Uuid",
                         parent_issue_sort_order as "parent_issue_sort_order: f64",
                         extension_metadata as "extension_metadata!: Value",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.project_id,
            issue_number,
            simple_id,
            data.status_id,
            data.title,
            data.description,
            data.priority,
            data.start_date,
            data.target_date,
            data.completed_at,
            data.sort_order,
            data.parent_issue_id,
            data.parent_issue_sort_order,
            extension_metadata
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(issue)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateIssue,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let status_id = data.status_id.unwrap_or(existing.status_id);
        let title = data.title.as_deref().unwrap_or(&existing.title);
        let description = data.description.as_ref().or(existing.description.as_ref());
        let priority = data.priority.as_ref().or(existing.priority.as_ref());
        let start_date = data.start_date.or(existing.start_date);
        let target_date = data.target_date.or(existing.target_date);
        let completed_at = data.completed_at.or(existing.completed_at);
        let sort_order = data.sort_order.unwrap_or(existing.sort_order);
        let parent_issue_id = data.parent_issue_id.or(existing.parent_issue_id);
        let parent_issue_sort_order = data.parent_issue_sort_order.or(existing.parent_issue_sort_order);
        let extension_metadata = data.extension_metadata.as_ref().unwrap_or(&existing.extension_metadata);

        sqlx::query_as!(
            Issue,
            r#"UPDATE issues
               SET status_id = $2, title = $3, description = $4, priority = $5,
                   start_date = $6, target_date = $7, completed_at = $8,
                   sort_order = $9, parent_issue_id = $10, parent_issue_sort_order = $11,
                   extension_metadata = $12, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         issue_number as "issue_number!: i32",
                         simple_id,
                         status_id as "status_id!: Uuid",
                         title,
                         description,
                         priority,
                         start_date as "start_date: DateTime<Utc>",
                         target_date as "target_date: DateTime<Utc>",
                         completed_at as "completed_at: DateTime<Utc>",
                         sort_order as "sort_order!: f64",
                         parent_issue_id as "parent_issue_id: Uuid",
                         parent_issue_sort_order as "parent_issue_sort_order: f64",
                         extension_metadata as "extension_metadata!: Value",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            status_id,
            title,
            description,
            priority,
            start_date,
            target_date,
            completed_at,
            sort_order,
            parent_issue_id,
            parent_issue_sort_order,
            extension_metadata
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM issues WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
