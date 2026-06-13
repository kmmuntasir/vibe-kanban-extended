use api_types::{
    CreateIssueRelationshipRequest, IssueRelationship, ListIssueRelationshipsQuery,
    ListIssueRelationshipsResponse, MutationResponse,
};
use axum::{
    Router,
    extract::{Json, Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use super::local_fallback;
use crate::{DeploymentImpl, error::ApiError};

pub(super) fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/issue-relationships",
            get(list_issue_relationships).post(create_issue_relationship),
        )
        .route(
            "/issue-relationships/{relationship_id}",
            axum::routing::delete(delete_issue_relationship),
        )
}

async fn list_issue_relationships(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssueRelationshipsQuery>,
) -> Result<ResponseJson<ApiResponse<ListIssueRelationshipsResponse>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.list_issue_relationships(query.issue_id).await?,
        Err(_) => {
            local_fallback::list_issue_relationships(&deployment.db().pool, query.issue_id).await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn create_issue_relationship(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<CreateIssueRelationshipRequest>,
) -> Result<ResponseJson<ApiResponse<MutationResponse<IssueRelationship>>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.create_issue_relationship(&request).await?,
        Err(_) => {
            local_fallback::create_issue_relationship(&deployment.db().pool, &request).await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn delete_issue_relationship(
    State(deployment): State<DeploymentImpl>,
    Path(relationship_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    match deployment.remote_client() {
        Ok(client) => client.delete_issue_relationship(relationship_id).await?,
        Err(_) => {
            local_fallback::delete_issue_relationship(&deployment.db().pool, relationship_id)
                .await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(())))
}
