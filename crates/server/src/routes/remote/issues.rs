use api_types::{
    CreateIssueRequest, Issue, ListIssuesQuery, ListIssuesResponse, MutationResponse,
    SearchIssuesRequest, UpdateIssueRequest,
};
use axum::{
    Router,
    extract::{Json, Path, Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use super::local_fallback;
use crate::{DeploymentImpl, error::ApiError};

pub(super) fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/issues", get(list_issues).post(create_issue))
        .route("/issues/search", post(search_issues))
        .route(
            "/issues/{issue_id}",
            get(get_issue).patch(update_issue).delete(delete_issue),
        )
}

async fn list_issues(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<ResponseJson<ApiResponse<ListIssuesResponse>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.list_issues(query.project_id).await?,
        Err(_) => local_fallback::list_issues(&deployment.db().pool, query.project_id).await?,
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn search_issues(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<SearchIssuesRequest>,
) -> Result<ResponseJson<ApiResponse<ListIssuesResponse>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.search_issues(&request).await?,
        Err(_) => local_fallback::search_issues(&deployment.db().pool, &request).await?,
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn get_issue(
    State(deployment): State<DeploymentImpl>,
    Path(issue_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Issue>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.get_issue(issue_id).await?,
        Err(_) => local_fallback::get_issue(&deployment.db().pool, issue_id).await?,
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn create_issue(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<CreateIssueRequest>,
) -> Result<ResponseJson<ApiResponse<MutationResponse<Issue>>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.create_issue(&request).await?,
        Err(_) => local_fallback::create_issue(&deployment.db().pool, &request).await?,
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn update_issue(
    State(deployment): State<DeploymentImpl>,
    Path(issue_id): Path<Uuid>,
    Json(request): Json<UpdateIssueRequest>,
) -> Result<ResponseJson<ApiResponse<MutationResponse<Issue>>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.update_issue(issue_id, &request).await?,
        Err(_) => local_fallback::update_issue(&deployment.db().pool, issue_id, &request).await?,
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn delete_issue(
    State(deployment): State<DeploymentImpl>,
    Path(issue_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    match deployment.remote_client() {
        Ok(client) => {
            client.delete_issue(issue_id).await?;
        }
        Err(_) => {
            local_fallback::delete_issue(&deployment.db().pool, issue_id).await?;
        }
    };
    Ok(ResponseJson(ApiResponse::success(())))
}
