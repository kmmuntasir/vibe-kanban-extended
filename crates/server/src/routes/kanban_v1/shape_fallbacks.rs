use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use db::models::{
    issue::Issue, issue_assignee::IssueAssignee, issue_comment::IssueComment,
    issue_relationship::IssueRelationship, issue_tag::IssueTag, kanban_tag::KanbanTag,
    project::Project, project_status::ProjectStatus,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DeploymentImpl;

// ---------------------------------------------------------------------------
// Error type — raw JSON errors, NOT ApiResponse-wrapped
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ErrorResponse {
    status: StatusCode,
    message: String,
}

impl ErrorResponse {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

// ---------------------------------------------------------------------------
// Query param types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
pub struct ProjectFallbackQuery {
    pub project_id: Uuid,
}

#[derive(Debug, serde::Deserialize)]
pub struct OrgFallbackQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CommentFallbackQuery {
    pub issue_id: Uuid,
}

// ---------------------------------------------------------------------------
// Response wrappers — match the shape the frontend expects
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ProjectsResponse {
    projects: Vec<Project>,
}

#[derive(Debug, Serialize)]
pub struct ProjectStatusesResponse {
    project_statuses: Vec<ProjectStatus>,
}

#[derive(Debug, Serialize)]
pub struct TagsResponse {
    tags: Vec<KanbanTag>,
}

#[derive(Debug, Serialize)]
pub struct IssueAssigneesResponse {
    issue_assignees: Vec<IssueAssignee>,
}

#[derive(Debug, Serialize)]
pub struct IssueTagsResponse {
    issue_tags: Vec<IssueTag>,
}

#[derive(Debug, Serialize)]
pub struct IssueRelationshipsResponse {
    issue_relationships: Vec<IssueRelationship>,
}

#[derive(Debug, Serialize)]
pub struct CommentsResponse {
    pub issue_comments: Vec<api_types::IssueComment>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn fallback_list_projects(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<OrgFallbackQuery>,
) -> Result<Json<ProjectsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let projects = Project::find_by_organization(pool, query.organization_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, organization_id = %query.organization_id, "failed to list projects (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list projects")
        })?;

    Ok(Json(ProjectsResponse { projects }))
}

pub async fn fallback_list_project_statuses(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ProjectFallbackQuery>,
) -> Result<Json<ProjectStatusesResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let project_statuses = ProjectStatus::find_by_project(pool, query.project_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, project_id = %query.project_id, "failed to list project statuses (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list project statuses")
        })?;

    Ok(Json(ProjectStatusesResponse { project_statuses }))
}

pub async fn fallback_list_tags(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ProjectFallbackQuery>,
) -> Result<Json<TagsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let tags = KanbanTag::find_by_project(pool, query.project_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, project_id = %query.project_id, "failed to list tags (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list tags")
        })?;

    Ok(Json(TagsResponse { tags }))
}

pub async fn fallback_list_issues(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ProjectFallbackQuery>,
) -> Result<Json<db::models::issue::ListIssuesResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issues = Issue::find_by_project(pool, query.project_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, project_id = %query.project_id, "failed to list issues (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list issues")
        })?;
    let total_count = issues.len();
    Ok(Json(db::models::issue::ListIssuesResponse {
        issues,
        total_count,
        limit: total_count,
        offset: 0,
    }))
}

pub async fn fallback_list_issue_assignees(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ProjectFallbackQuery>,
) -> Result<Json<IssueAssigneesResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issue_assignees = IssueAssignee::find_by_project(pool, query.project_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, project_id = %query.project_id, "failed to list issue assignees (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list issue assignees")
        })?;

    Ok(Json(IssueAssigneesResponse { issue_assignees }))
}

pub async fn fallback_list_issue_tags(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ProjectFallbackQuery>,
) -> Result<Json<IssueTagsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issue_tags = IssueTag::find_by_project(pool, query.project_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, project_id = %query.project_id, "failed to list issue tags (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list issue tags")
        })?;

    Ok(Json(IssueTagsResponse { issue_tags }))
}

pub async fn fallback_list_issue_relationships(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ProjectFallbackQuery>,
) -> Result<Json<IssueRelationshipsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issue_relationships = IssueRelationship::find_by_project(pool, query.project_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, project_id = %query.project_id, "failed to list issue relationships (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list issue relationships")
        })?;

    Ok(Json(IssueRelationshipsResponse {
        issue_relationships,
    }))
}

pub async fn fallback_list_issue_comments(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<CommentFallbackQuery>,
) -> Result<Json<CommentsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let comments = IssueComment::find_by_issue(pool, query.issue_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, issue_id = %query.issue_id, "failed to list issue comments (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list issue comments")
        })?;

    let api_comments: Vec<api_types::IssueComment> = comments
        .into_iter()
        .map(|c| api_types::IssueComment {
            id: c.id,
            issue_id: c.issue_id,
            author_id: c.author_id,
            parent_id: c.parent_id,
            message: c.message,
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .collect();

    Ok(Json(CommentsResponse {
        issue_comments: api_comments,
    }))
}

// ---------------------------------------------------------------------------
// Fallback handlers for shapes that have no local DB table.
// Return empty lists so the frontend doesn't get HTML fallback errors.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserFallbackQuery {
    user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserWorkspaceFallbackQuery {
    owner_user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OptionalOrgFallbackQuery {
    organization_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OptionalProjectFallbackQuery {
    project_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OptionalCommentFallbackQuery {
    issue_id: Option<Uuid>,
}

pub async fn fallback_list_notifications(
    Query(_query): Query<UserFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    Ok(Json(serde_json::json!({ "notifications": [] })))
}

pub async fn fallback_list_organization_members(
    Query(_query): Query<OptionalOrgFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    Ok(Json(serde_json::json!({ "organization_members": [] })))
}

pub async fn fallback_list_users(
    Query(_query): Query<OptionalOrgFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    Ok(Json(serde_json::json!({ "users": [] })))
}

#[derive(Debug, sqlx::FromRow)]
struct WorkspaceFallbackRow {
    id: Uuid,
    issue_id: Option<Uuid>,
    project_id: Option<Uuid>,
    name: Option<String>,
    archived: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub async fn fallback_list_project_workspaces(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<OptionalProjectFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let rows: Vec<WorkspaceFallbackRow> = sqlx::query_as(
        r#"SELECT id, issue_id, project_id, name,
                  archived, created_at, updated_at
           FROM workspaces
           WHERE project_id = $1
           ORDER BY updated_at DESC"#,
    )
    .bind(query.project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!(?e, "failed to list project workspaces (fallback)");
        ErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to list project workspaces",
        )
    })?;

    let local_user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let workspaces: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "project_id": r.project_id,
                "owner_user_id": local_user_id,
                "issue_id": r.issue_id,
                "local_workspace_id": r.id,
                "name": r.name,
                "archived": r.archived,
                "files_changed": null,
                "lines_added": null,
                "lines_removed": null,
                "created_at": r.created_at,
                "updated_at": r.updated_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "workspaces": workspaces })))
}

pub async fn fallback_list_user_workspaces(
    State(deployment): State<DeploymentImpl>,
    Query(_query): Query<UserWorkspaceFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let rows: Vec<WorkspaceFallbackRow> = sqlx::query_as(
        r#"SELECT id, issue_id, project_id, name,
                  archived, created_at, updated_at
           FROM workspaces
           ORDER BY updated_at DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!(?e, "failed to list user workspaces (fallback)");
        ErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to list user workspaces",
        )
    })?;

    let local_user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let workspaces: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "project_id": r.project_id,
                "owner_user_id": local_user_id,
                "issue_id": r.issue_id,
                "local_workspace_id": r.id,
                "name": r.name,
                "archived": r.archived,
                "files_changed": null,
                "lines_added": null,
                "lines_removed": null,
                "created_at": r.created_at,
                "updated_at": r.updated_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "workspaces": workspaces })))
}

pub async fn fallback_list_issue_followers(
    Query(_query): Query<OptionalProjectFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    Ok(Json(serde_json::json!({ "issue_followers": [] })))
}

pub async fn fallback_list_pull_requests(
    Query(_query): Query<OptionalProjectFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    Ok(Json(serde_json::json!({ "pull_requests": [] })))
}

pub async fn fallback_list_pull_request_issues(
    Query(_query): Query<OptionalProjectFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    Ok(Json(serde_json::json!({ "pull_request_issues": [] })))
}

pub async fn fallback_list_issue_comment_reactions(
    Query(_query): Query<OptionalCommentFallbackQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    Ok(Json(serde_json::json!({ "issue_comment_reactions": [] })))
}
