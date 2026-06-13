use api_types::{
    CreateIssueAssigneeRequest, IssueAssignee, ListIssueAssigneesResponse, MutationResponse,
};
use axum::{
    Router,
    extract::{Json, Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use super::local_fallback;
use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub(super) struct ListIssueAssigneesQuery {
    pub issue_id: Uuid,
}

pub(super) fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/issue-assignees",
            get(list_issue_assignees).post(create_issue_assignee),
        )
        .route(
            "/issue-assignees/{issue_assignee_id}",
            get(get_issue_assignee).delete(delete_issue_assignee),
        )
}

async fn list_issue_assignees(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssueAssigneesQuery>,
) -> Result<ResponseJson<ApiResponse<ListIssueAssigneesResponse>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.list_issue_assignees(query.issue_id).await?,
        Err(_) => {
            local_fallback::list_issue_assignees(&deployment.db().pool, query.issue_id).await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn get_issue_assignee(
    State(deployment): State<DeploymentImpl>,
    Path(issue_assignee_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<IssueAssignee>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.get_issue_assignee(issue_assignee_id).await?,
        Err(_) => {
            local_fallback::get_issue_assignee(&deployment.db().pool, issue_assignee_id).await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn create_issue_assignee(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<CreateIssueAssigneeRequest>,
) -> Result<ResponseJson<ApiResponse<MutationResponse<IssueAssignee>>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.create_issue_assignee(&request).await?,
        Err(_) => local_fallback::create_issue_assignee(&deployment.db().pool, &request).await?,
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn delete_issue_assignee(
    State(deployment): State<DeploymentImpl>,
    Path(issue_assignee_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    match deployment.remote_client() {
        Ok(client) => {
            client.delete_issue_assignee(issue_assignee_id).await?;
        }
        Err(_) => {
            local_fallback::delete_issue_assignee(&deployment.db().pool, issue_assignee_id).await?;
        }
    };
    Ok(ResponseJson(ApiResponse::success(())))
}
