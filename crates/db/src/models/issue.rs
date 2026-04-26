use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

use super::project_status::ProjectStatus;

#[derive(FromRow)]
struct CountRow {
    count: i64,
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
    pub priority: String,
    pub start_date: Option<DateTime<Utc>>,
    pub target_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sort_order: i32,
    pub parent_issue_id: Option<Uuid>,
    pub parent_issue_sort_order: Option<f64>,
    pub extension_metadata: String,
    pub creator_user_id: Option<Uuid>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ListIssuesResponse {
    pub issues: Vec<Issue>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum IssueSortField {
    SortOrder,
    Priority,
    CreatedAt,
    UpdatedAt,
    Title,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SearchIssuesRequest {
    pub project_id: Uuid,
    pub status_id: Option<Uuid>,
    pub status_ids: Option<Vec<Uuid>>,
    pub priority: Option<String>,
    pub parent_issue_id: Option<Uuid>,
    pub search: Option<String>,
    pub simple_id: Option<String>,
    pub assignee_user_id: Option<Uuid>,
    pub tag_id: Option<Uuid>,
    pub tag_ids: Option<Vec<Uuid>>,
    pub sort_field: Option<IssueSortField>,
    pub sort_direction: Option<SortDirection>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl Issue {
    pub async fn create(
        pool: &SqlitePool,
        id: Option<Uuid>,
        project_id: Uuid,
        status_id: Uuid,
        title: &str,
        description: Option<&str>,
        priority: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        target_date: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
        sort_order: i32,
        parent_issue_id: Option<Uuid>,
        parent_issue_sort_order: Option<f64>,
        extension_metadata: &JsonValue,
        creator_user_id: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        let mut tx = pool.begin().await?;

        let id = id.unwrap_or_else(Uuid::new_v4);
        let priority = priority.unwrap_or("medium");
        let extension_metadata = extension_metadata.to_string();

        // Atomic increment issue_counter and get new number
        let issue_number: i32 = sqlx::query_scalar!(
            r#"UPDATE projects SET issue_counter = issue_counter + 1
               WHERE id = $1
               RETURNING issue_counter as "n!: i32""#,
            project_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // Get issue_prefix from organization
        let prefix: Option<String> = sqlx::query_scalar!(
            r#"SELECT o.issue_prefix
               FROM organizations o
               JOIN projects p ON p.organization_id = o.id
               WHERE p.id = $1"#,
            project_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let prefix = prefix.as_deref().unwrap_or("VK");
        let simple_id = format!("{}-{}", prefix, issue_number);

        let issue = sqlx::query_as!(
            Issue,
            r#"INSERT INTO issues (
                   id, project_id, issue_number, simple_id, status_id, title, description,
                   priority, start_date, target_date, completed_at, sort_order,
                   parent_issue_id, parent_issue_sort_order, extension_metadata, creator_user_id
               ) VALUES (
                   $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16
               )
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
                         sort_order as "sort_order!: i32",
                         parent_issue_id as "parent_issue_id: Uuid",
                         parent_issue_sort_order,
                         extension_metadata,
                         creator_user_id as "creator_user_id: Uuid",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            issue_number,
            simple_id,
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
            extension_metadata,
            creator_user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(issue)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
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
                      sort_order as "sort_order!: i32",
                      parent_issue_id as "parent_issue_id: Uuid",
                      parent_issue_sort_order,
                      extension_metadata,
                      creator_user_id as "creator_user_id: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM issues
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

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
                      sort_order as "sort_order!: i32",
                      parent_issue_id as "parent_issue_id: Uuid",
                      parent_issue_sort_order,
                      extension_metadata,
                      creator_user_id as "creator_user_id: Uuid",
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

    pub async fn search(
        pool: &SqlitePool,
        query: &SearchIssuesRequest,
    ) -> Result<ListIssuesResponse, sqlx::Error> {
        let mut conditions = vec![format!("i.project_id = '{}'", query.project_id)];

        if let Some(ref status_id) = query.status_id {
            conditions.push(format!("i.status_id = '{}'", status_id));
        }
        if let Some(ref status_ids) = query.status_ids {
            let ids: Vec<String> = status_ids.iter().map(|id| format!("'{}'", id)).collect();
            conditions.push(format!("i.status_id IN ({})", ids.join(", ")));
        }
        if let Some(ref priority) = query.priority {
            conditions.push(format!("i.priority = '{}'", priority));
        }
        if let Some(ref parent_id) = query.parent_issue_id {
            conditions.push(format!("i.parent_issue_id = '{}'", parent_id));
        }
        if let Some(ref search) = query.search {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(i.title LIKE '%{escaped}%' OR COALESCE(i.description, '') LIKE '%{escaped}%')"
            ));
        }
        if let Some(ref simple_id) = query.simple_id {
            let escaped = simple_id.replace('\'', "''");
            conditions.push(format!("i.simple_id LIKE '%{escaped}%'"));
        }
        if let Some(ref assignee_id) = query.assignee_user_id {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM issue_assignees ia WHERE ia.issue_id = i.id AND ia.user_id = '{}')",
                assignee_id
            ));
        }
        if let Some(ref tag_id) = query.tag_id {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM issue_tags it WHERE it.issue_id = i.id AND it.tag_id = '{}')",
                tag_id
            ));
        }
        if let Some(ref tag_ids) = query.tag_ids {
            let ids: Vec<String> = tag_ids.iter().map(|id| format!("'{}'", id)).collect();
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM issue_tags it WHERE it.issue_id = i.id AND it.tag_id IN ({}))",
                ids.join(", ")
            ));
        }

        let where_clause = conditions.join(" AND ");

        let sort_field = match query.sort_field.unwrap_or(IssueSortField::SortOrder) {
            IssueSortField::SortOrder => "i.sort_order",
            IssueSortField::Priority => "i.priority",
            IssueSortField::CreatedAt => "i.created_at",
            IssueSortField::UpdatedAt => "i.updated_at",
            IssueSortField::Title => "i.title",
        };
        let sort_dir = match query.sort_direction.unwrap_or(SortDirection::Asc) {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        let offset = query.offset.unwrap_or(0).max(0) as i64;
        let limit = query
            .limit
            .map(|v| v.max(0) as i64)
            .unwrap_or(i64::MAX);

        let count_sql = format!(
            "SELECT COUNT(*) as count FROM issues i WHERE {where_clause}"
        );
        let row: CountRow = sqlx::query_as(&count_sql)
            .fetch_one(pool)
            .await?;
        let total_count = row.count;

        let data_sql = format!(
            r#"SELECT i.id, i.project_id, i.issue_number, i.simple_id, i.status_id,
                      i.title, i.description, i.priority, i.start_date, i.target_date,
                      i.completed_at, i.sort_order, i.parent_issue_id,
                      i.parent_issue_sort_order, i.extension_metadata, i.creator_user_id,
                      i.created_at, i.updated_at
               FROM issues i
               LEFT JOIN project_statuses ps ON ps.id = i.status_id
               WHERE {where_clause}
               ORDER BY ps.sort_order ASC, i.sort_order ASC, {sort_field} {sort_dir}, i.issue_number ASC
               LIMIT {limit} OFFSET {offset}"#
        );
        let issues: Vec<Issue> = sqlx::query_as(&data_sql).fetch_all(pool).await?;

        let limit_usize = limit as usize;
        let offset_usize = offset as usize;

        Ok(ListIssuesResponse {
            issues,
            total_count: total_count as usize,
            limit: limit_usize,
            offset: offset_usize,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        status_id: Option<Uuid>,
        title: Option<&str>,
        description: Option<Option<&str>>,
        priority: Option<Option<&str>>,
        start_date: Option<Option<DateTime<Utc>>>,
        target_date: Option<Option<DateTime<Utc>>>,
        completed_at: Option<Option<DateTime<Utc>>>,
        sort_order: Option<i32>,
        parent_issue_id: Option<Option<Uuid>>,
        parent_issue_sort_order: Option<Option<f64>>,
        extension_metadata: Option<&JsonValue>,
    ) -> Result<Self, sqlx::Error> {
        let update_description = description.is_some();
        let description_value = description.flatten();
        let update_priority = priority.is_some();
        let priority_value = priority.flatten().unwrap_or("medium");
        let update_start_date = start_date.is_some();
        let start_date_value = start_date.flatten();
        let update_target_date = target_date.is_some();
        let target_date_value = target_date.flatten();
        let update_completed_at = completed_at.is_some();
        let completed_at_value = completed_at.flatten();
        let update_parent_issue_id = parent_issue_id.is_some();
        let parent_issue_id_value = parent_issue_id.flatten();
        let update_parent_issue_sort_order = parent_issue_sort_order.is_some();
        let parent_issue_sort_order_value = parent_issue_sort_order.flatten();
        let extension_metadata_str = extension_metadata.map(|v| v.to_string());

        sqlx::query_as!(
            Issue,
            r#"UPDATE issues
               SET status_id = COALESCE($2, status_id),
                   title = COALESCE($3, title),
                   description = CASE WHEN $4 THEN $5 ELSE description END,
                   priority = CASE WHEN $6 THEN $7 ELSE priority END,
                   start_date = CASE WHEN $8 THEN $9 ELSE start_date END,
                   target_date = CASE WHEN $10 THEN $11 ELSE target_date END,
                   completed_at = CASE WHEN $12 THEN $13 ELSE completed_at END,
                   sort_order = COALESCE($14, sort_order),
                   parent_issue_id = CASE WHEN $15 THEN $16 ELSE parent_issue_id END,
                   parent_issue_sort_order = CASE WHEN $17 THEN $18 ELSE parent_issue_sort_order END,
                   extension_metadata = COALESCE($19, extension_metadata),
                   updated_at = datetime('now', 'subsec')
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
                         sort_order as "sort_order!: i32",
                         parent_issue_id as "parent_issue_id: Uuid",
                         parent_issue_sort_order,
                         extension_metadata,
                         creator_user_id as "creator_user_id: Uuid",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            status_id,
            title,
            update_description,
            description_value,
            update_priority,
            priority_value,
            update_start_date,
            start_date_value,
            update_target_date,
            target_date_value,
            update_completed_at,
            completed_at_value,
            sort_order,
            update_parent_issue_id,
            parent_issue_id_value,
            update_parent_issue_sort_order,
            parent_issue_sort_order_value,
            extension_metadata_str,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM issues WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Move issue to a named status (e.g. "In progress", "Done") if it exists for the project.
    pub async fn move_to_named_status(
        pool: &SqlitePool,
        issue_id: Uuid,
        status_name: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let issue = match Self::find_by_id(pool, issue_id).await? {
            Some(i) => i,
            None => return Ok(None),
        };

        let target = sqlx::query_as!(
            ProjectStatus,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      color,
                      sort_order as "sort_order!: i32",
                      hidden as "hidden: bool",
                      created_at as "created_at!: DateTime<Utc>"
               FROM project_statuses
               WHERE project_id = $1 AND name = $2"#,
            issue.project_id,
            status_name
        )
        .fetch_optional(pool)
        .await?;

        let target = match target {
            Some(t) => t,
            None => return Ok(Some(issue)),
        };

        if issue.status_id == target.id {
            return Ok(Some(issue));
        }

        let updated = Self::update(
            pool,
            issue_id,
            Some(target.id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        Ok(Some(updated))
    }
}
