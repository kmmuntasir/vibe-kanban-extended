use axum::{
    Router,
    extract::{Json, Path, Query, State},
    response::Json as ResponseJson,
    routing::{delete, get, patch, post},
};
use db::models::issue::{CreateIssue, Issue, UpdateIssue};
use serde::Deserialize;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/issues", get(list_issues).post(create_issue))
        .route("/issues/search", post(search_issues))
        .route("/issues/{issue_id}", get(get_issue).patch(update_issue).delete(delete_issue))
        .route("/issue_assignees", get(list_assignees).post(create_assignee))
        .route("/issue_assignees/{assignee_id}", delete(delete_assignee))
        .route("/issue_relationships", get(list_relationships).post(create_relationship))
        .route("/issue_relationships/{relationship_id}", delete(delete_relationship))
        .route("/issue_tags", get(list_issue_tags).post(create_issue_tag))
        .route("/issue_tags/{issue_tag_id}", delete(delete_issue_tag))
}

#[derive(Deserialize)]
struct ListIssuesQuery {
    project_id: Option<Uuid>,
}

async fn list_issues(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let project_id = query.project_id
        .ok_or_else(|| ApiError::BadRequest("project_id is required".into()))?;
    let issues = Issue::find_by_project(pool, project_id).await?;
    Ok(ResponseJson(serde_json::json!({
        "issues": issues,
        "total_count": issues.len(),
        "limit": 50,
        "offset": 0
    })))
}

#[derive(Deserialize)]
struct SearchIssuesRequest {
    project_id: Option<Uuid>,
    search: Option<String>,
    status_id: Option<Uuid>,
    limit: Option<i32>,
    offset: Option<i32>,
}

async fn search_issues(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<SearchIssuesRequest>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let project_id = req.project_id
        .ok_or_else(|| ApiError::BadRequest("project_id is required".into()))?;
    let mut issues = Issue::find_by_project(pool, project_id).await?;

    // Filter by status
    if let Some(status_id) = req.status_id {
        issues.retain(|i| i.status_id == status_id);
    }
    // Filter by search text
    if let Some(search) = &req.search {
        let search_lower = search.to_lowercase();
        issues.retain(|i| {
            i.title.to_lowercase().contains(&search_lower)
                || i.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&search_lower))
        });
    }

    let total_count = issues.len() as i32;
    let offset = req.offset.unwrap_or(0) as usize;
    let limit = req.limit.unwrap_or(50) as usize;
    let issues: Vec<_> = issues.into_iter().skip(offset).take(limit).collect();

    Ok(ResponseJson(serde_json::json!({
        "issues": issues,
        "total_count": total_count,
        "limit": limit as i32,
        "offset": offset as i32
    })))
}

async fn get_issue(
    State(deployment): State<DeploymentImpl>,
    Path(issue_id): Path<Uuid>,
) -> Result<ResponseJson<Issue>, ApiError> {
    let pool = &deployment.db().pool;
    let issue = Issue::find_by_id(pool, issue_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Issue not found".into()))?;
    Ok(ResponseJson(issue))
}

async fn create_issue(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateIssue>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let prefix = db::models::project::KanbanProject::get_issue_prefix(pool, req.project_id).await?;
    let issue = Issue::create(pool, &req, &prefix).await?;
    Ok(ResponseJson(serde_json::json!({ "data": issue, "txid": 0 })))
}

async fn update_issue(
    State(deployment): State<DeploymentImpl>,
    Path(issue_id): Path<Uuid>,
    Json(req): Json<UpdateIssue>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let issue = Issue::update(pool, issue_id, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": issue, "txid": 0 })))
}

async fn delete_issue(
    State(deployment): State<DeploymentImpl>,
    Path(issue_id): Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    Issue::delete(pool, issue_id).await?;
    Ok(ResponseJson(serde_json::json!({ "id": issue_id.to_string(), "txid": 0 })))
}

// Issue Assignees

use db::models::issue_assignee::{CreateIssueAssignee, IssueAssignee};

#[derive(Deserialize)]
struct ListAssigneesQuery {
    issue_id: Option<Uuid>,
}

async fn list_assignees(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListAssigneesQuery>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let issue_id = query.issue_id
        .ok_or_else(|| ApiError::BadRequest("issue_id is required".into()))?;
    let assignees = IssueAssignee::find_by_issue(pool, issue_id).await?;
    Ok(ResponseJson(serde_json::json!({ "issue_assignees": assignees })))
}

async fn create_assignee(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateIssueAssignee>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let assignee = IssueAssignee::create(pool, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": assignee, "txid": 0 })))
}

async fn delete_assignee(
    State(deployment): State<DeploymentImpl>,
    Path(assignee_id): Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    IssueAssignee::delete(pool, assignee_id).await?;
    Ok(ResponseJson(serde_json::json!({ "id": assignee_id.to_string(), "txid": 0 })))
}

// Issue Relationships

use db::models::issue_relationship::{CreateIssueRelationship, IssueRelationship};

#[derive(Deserialize)]
struct ListRelationshipsQuery {
    issue_id: Option<Uuid>,
}

async fn list_relationships(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListRelationshipsQuery>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let issue_id = query.issue_id
        .ok_or_else(|| ApiError::BadRequest("issue_id is required".into()))?;
    let relationships = IssueRelationship::find_by_issue(pool, issue_id).await?;
    Ok(ResponseJson(serde_json::json!({ "issue_relationships": relationships })))
}

async fn create_relationship(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateIssueRelationship>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let relationship = IssueRelationship::create(pool, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": relationship, "txid": 0 })))
}

async fn delete_relationship(
    State(deployment): State<DeploymentImpl>,
    Path(relationship_id): Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    IssueRelationship::delete(pool, relationship_id).await?;
    Ok(ResponseJson(serde_json::json!({ "id": relationship_id.to_string(), "txid": 0 })))
}

// Issue Tags

use db::models::issue_tag::{CreateIssueTag, IssueTag};

#[derive(Deserialize)]
struct ListIssueTagsQuery {
    issue_id: Option<Uuid>,
}

async fn list_issue_tags(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssueTagsQuery>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let issue_id = query.issue_id
        .ok_or_else(|| ApiError::BadRequest("issue_id is required".into()))?;
    let tags = IssueTag::find_by_issue(pool, issue_id).await?;
    Ok(ResponseJson(serde_json::json!({ "issue_tags": tags })))
}

async fn create_issue_tag(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateIssueTag>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let tag = IssueTag::create(pool, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": tag, "txid": 0 })))
}

async fn delete_issue_tag(
    State(deployment): State<DeploymentImpl>,
    Path(issue_tag_id): Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    IssueTag::delete(pool, issue_tag_id).await?;
    Ok(ResponseJson(serde_json::json!({ "id": issue_tag_id.to_string(), "txid": 0 })))
}
