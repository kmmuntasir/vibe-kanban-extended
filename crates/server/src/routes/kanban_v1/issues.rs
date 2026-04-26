use api_types::{
    BulkUpdateIssuesRequest, BulkUpdateIssuesResponse, CreateIssueRequest, DeleteResponse,
    Issue, IssuePriority, ListIssuesQuery, ListIssuesResponse, MutationResponse,
    SearchIssuesRequest, UpdateIssueRequest,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use db::models::issue::{Issue as DbIssue, IssueSortField as DbSortField, SearchIssuesRequest as DbSearchRequest, SortDirection as DbSortDirection};
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub(super) fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/issues", get(list_issues).post(create_issue))
        .route("/issues/search", post(search_issues))
        .route("/issues/bulk", post(bulk_update_issues))
        .route(
            "/issues/{id}",
            get(get_issue).patch(update_issue).delete(delete_issue),
        )
}

fn txid() -> i64 {
    Utc::now().timestamp_millis()
}

fn to_api_issue(db: DbIssue) -> Issue {
    Issue {
        id: db.id,
        project_id: db.project_id,
        issue_number: db.issue_number,
        simple_id: db.simple_id,
        status_id: db.status_id,
        title: db.title,
        description: db.description,
        priority: db
            .priority
            .parse::<IssuePriority>()
            .ok(),
        start_date: db.start_date,
        target_date: db.target_date,
        completed_at: db.completed_at,
        sort_order: db.sort_order as f64,
        parent_issue_id: db.parent_issue_id,
        parent_issue_sort_order: db.parent_issue_sort_order,
        extension_metadata: serde_json::from_str(&db.extension_metadata).unwrap_or_default(),
        creator_user_id: db.creator_user_id,
        created_at: db.created_at,
        updated_at: db.updated_at,
    }
}

fn priority_to_str(p: &Option<Option<IssuePriority>>) -> Option<Option<&str>> {
    p.as_ref().map(|inner| inner.as_ref().map(|p| match p {
        IssuePriority::Urgent => "urgent",
        IssuePriority::High => "high",
        IssuePriority::Medium => "medium",
        IssuePriority::Low => "low",
    }))
}

fn to_db_search(req: &SearchIssuesRequest) -> DbSearchRequest {
    DbSearchRequest {
        project_id: req.project_id,
        status_id: req.status_id,
        status_ids: req.status_ids.clone(),
        priority: req.priority.as_ref().map(|p| match p {
            IssuePriority::Urgent => "urgent".to_string(),
            IssuePriority::High => "high".to_string(),
            IssuePriority::Medium => "medium".to_string(),
            IssuePriority::Low => "low".to_string(),
        }),
        parent_issue_id: req.parent_issue_id,
        search: req.search.clone(),
        simple_id: req.simple_id.clone(),
        assignee_user_id: req.assignee_user_id,
        tag_id: req.tag_id,
        tag_ids: req.tag_ids.clone(),
        sort_field: req.sort_field.map(|f| match f {
            api_types::IssueSortField::SortOrder => DbSortField::SortOrder,
            api_types::IssueSortField::Priority => DbSortField::Priority,
            api_types::IssueSortField::CreatedAt => DbSortField::CreatedAt,
            api_types::IssueSortField::UpdatedAt => DbSortField::UpdatedAt,
            api_types::IssueSortField::Title => DbSortField::Title,
        }),
        sort_direction: req.sort_direction.map(|d| match d {
            api_types::SortDirection::Asc => DbSortDirection::Asc,
            api_types::SortDirection::Desc => DbSortDirection::Desc,
        }),
        limit: req.limit,
        offset: req.offset,
    }
}

async fn list_issues(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<Json<ListIssuesResponse>, ApiError> {
    let pool = &deployment.db().pool;
    let issues = DbIssue::find_by_project(pool, query.project_id).await?;
    let total_count = issues.len();
    Ok(Json(ListIssuesResponse {
        issues: issues.into_iter().map(to_api_issue).collect(),
        total_count,
        limit: total_count,
        offset: 0,
    }))
}

async fn search_issues(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<SearchIssuesRequest>,
) -> Result<Json<ListIssuesResponse>, ApiError> {
    let pool = &deployment.db().pool;
    let db_request = to_db_search(&request);
    let response = DbIssue::search(pool, &db_request).await?;
    Ok(Json(ListIssuesResponse {
        issues: response.issues.into_iter().map(to_api_issue).collect(),
        total_count: response.total_count,
        limit: response.limit,
        offset: response.offset,
    }))
}

async fn get_issue(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<Issue>, ApiError> {
    let pool = &deployment.db().pool;
    let issue = DbIssue::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Issue not found".to_string()))?;
    Ok(Json(to_api_issue(issue)))
}

async fn create_issue(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<CreateIssueRequest>,
) -> Result<Json<MutationResponse<Issue>>, ApiError> {
    let pool = &deployment.db().pool;
    let priority_str = request.priority.as_ref().map(|p| match p {
        IssuePriority::Urgent => "urgent",
        IssuePriority::High => "high",
        IssuePriority::Medium => "medium",
        IssuePriority::Low => "low",
    });
    let issue = DbIssue::create(
        pool,
        request.id,
        request.project_id,
        request.status_id,
        &request.title,
        request.description.as_deref(),
        priority_str,
        request.start_date,
        request.target_date,
        request.completed_at,
        request.sort_order as i32,
        request.parent_issue_id,
        request.parent_issue_sort_order,
        &request.extension_metadata,
        None,
    )
    .await?;
    Ok(Json(MutationResponse {
        data: to_api_issue(issue),
        txid: txid(),
    }))
}

async fn update_issue(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateIssueRequest>,
) -> Result<Json<MutationResponse<Issue>>, ApiError> {
    let pool = &deployment.db().pool;
    let issue = DbIssue::update(
        pool,
        id,
        request.status_id,
        request.title.as_deref(),
        request.description.as_ref().map(|d| d.as_ref().map(|s| s.as_str())),
        priority_to_str(&request.priority),
        request.start_date,
        request.target_date,
        request.completed_at,
        request.sort_order.map(|s| s as i32),
        request.parent_issue_id,
        request.parent_issue_sort_order,
        request.extension_metadata.as_ref(),
    )
    .await?;
    Ok(Json(MutationResponse {
        data: to_api_issue(issue),
        txid: txid(),
    }))
}

async fn delete_issue(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ApiError> {
    let pool = &deployment.db().pool;
    DbIssue::delete(pool, id).await?;
    Ok(Json(DeleteResponse { txid: txid() }))
}

async fn bulk_update_issues(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<BulkUpdateIssuesRequest>,
) -> Result<Json<BulkUpdateIssuesResponse>, ApiError> {
    let pool = &deployment.db().pool;
    let mut updated = Vec::with_capacity(request.updates.len());
    for item in request.updates {
        let issue = DbIssue::update(
            pool,
            item.id,
            item.changes.status_id,
            item.changes.title.as_deref(),
            item.changes
                .description
                .as_ref()
                .map(|d| d.as_ref().map(|s| s.as_str())),
            priority_to_str(&item.changes.priority),
            item.changes.start_date,
            item.changes.target_date,
            item.changes.completed_at,
            item.changes.sort_order.map(|s| s as i32),
            item.changes.parent_issue_id,
            item.changes.parent_issue_sort_order,
            item.changes.extension_metadata.as_ref(),
        )
        .await?;
        updated.push(to_api_issue(issue));
    }
    Ok(Json(BulkUpdateIssuesResponse {
        data: updated,
        txid: txid(),
    }))
}
