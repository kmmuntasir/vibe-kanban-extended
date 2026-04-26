use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use db::models::issue_assignee::IssueAssignee;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DeleteResponse, ErrorResponse, MutationResponse, db_error, local_txid};
use crate::DeploymentImpl;

#[derive(Debug, Deserialize)]
pub struct ListIssueAssigneesQuery {
    pub issue_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ListIssueAssigneesResponse {
    pub issue_assignees: Vec<IssueAssignee>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIssueAssigneeRequest {
    pub issue_id: Uuid,
    pub user_id: Uuid,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_issue_assignees).post(create_issue_assignee))
        .route("/{assignee_id}", get(get_issue_assignee).delete(delete_issue_assignee))
}

async fn list_issue_assignees(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssueAssigneesQuery>,
) -> Result<Json<ListIssueAssigneesResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issue_assignees = IssueAssignee::find_by_issue(pool, query.issue_id)
        .await
        .map_err(|e| db_error(e, "failed to list issue assignees"))?;
    Ok(Json(ListIssueAssigneesResponse { issue_assignees }))
}

async fn get_issue_assignee(
    State(deployment): State<DeploymentImpl>,
    Path(assignee_id): Path<Uuid>,
) -> Result<Json<IssueAssignee>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let assignee = sqlx::query_as!(
        IssueAssignee,
        r#"SELECT id as "id!: Uuid",
                  issue_id as "issue_id!: Uuid",
                  user_id as "user_id!: Uuid",
                  assigned_at as "assigned_at!: DateTime<Utc>"
           FROM issue_assignees
           WHERE id = $1"#,
        assignee_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| db_error(e, "failed to get issue assignee"))?
    .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "issue assignee not found"))?;
    Ok(Json(assignee))
}

async fn create_issue_assignee(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateIssueAssigneeRequest>,
) -> Result<Json<MutationResponse<IssueAssignee>>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let assignee = IssueAssignee::create(pool, payload.issue_id, payload.user_id)
        .await
        .map_err(|e| db_error(e, "failed to create issue assignee"))?;
    Ok(Json(MutationResponse {
        data: assignee,
        txid: local_txid(),
    }))
}

async fn delete_issue_assignee(
    State(deployment): State<DeploymentImpl>,
    Path(assignee_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    IssueAssignee::delete(pool, assignee_id)
        .await
        .map_err(|e| db_error(e, "failed to delete issue assignee"))?;
    Ok(Json(DeleteResponse { txid: local_txid() }))
}
