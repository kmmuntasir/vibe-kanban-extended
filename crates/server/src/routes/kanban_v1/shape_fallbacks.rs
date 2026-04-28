use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use db::models::{
    issue::Issue, issue_assignee::IssueAssignee, issue_relationship::IssueRelationship,
    issue_tag::IssueTag, kanban_tag::KanbanTag, project::Project, project_status::ProjectStatus,
};
use deployment::Deployment;
use serde::Serialize;
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
    let response = Issue::search(
        pool,
        &db::models::issue::SearchIssuesRequest {
            project_id: query.project_id,
            status_id: None,
            status_ids: None,
            priority: None,
            parent_issue_id: None,
            search: None,
            simple_id: None,
            assignee_user_id: None,
            tag_id: None,
            tag_ids: None,
            sort_field: None,
            sort_direction: None,
            limit: None,
            offset: None,
        },
    )
    .await
    .map_err(|e| {
        tracing::error!(?e, project_id = %query.project_id, "failed to list issues (fallback)");
        ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list issues")
    })?;

    Ok(Json(response))
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
