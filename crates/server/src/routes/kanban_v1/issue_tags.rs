use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use db::models::issue_tag::IssueTag;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DeleteResponse, ErrorResponse, MutationResponse, db_error, local_txid};
use crate::DeploymentImpl;

#[derive(Debug, Deserialize)]
pub struct ListIssueTagsQuery {
    pub issue_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ListIssueTagsResponse {
    pub issue_tags: Vec<IssueTag>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIssueTagRequest {
    pub issue_id: Uuid,
    pub tag_id: Uuid,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_issue_tags).post(create_issue_tag))
        .route(
            "/{issue_tag_id}",
            get(get_issue_tag).delete(delete_issue_tag),
        )
}

async fn list_issue_tags(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssueTagsQuery>,
) -> Result<Json<ListIssueTagsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issue_tags = IssueTag::find_by_issue(pool, query.issue_id)
        .await
        .map_err(|e| db_error(e, "failed to list issue tags"))?;
    Ok(Json(ListIssueTagsResponse { issue_tags }))
}

async fn get_issue_tag(
    State(deployment): State<DeploymentImpl>,
    Path(issue_tag_id): Path<Uuid>,
) -> Result<Json<IssueTag>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issue_tag = IssueTag::find_by_id(pool, issue_tag_id)
        .await
        .map_err(|e| db_error(e, "failed to get issue tag"))?
        .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "issue tag not found"))?;
    Ok(Json(issue_tag))
}

async fn create_issue_tag(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateIssueTagRequest>,
) -> Result<Json<MutationResponse<IssueTag>>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issue_tag = IssueTag::create(pool, None, payload.issue_id, payload.tag_id)
        .await
        .map_err(|e| db_error(e, "failed to create issue tag"))?;
    Ok(Json(MutationResponse {
        data: issue_tag,
        txid: local_txid(),
    }))
}

async fn delete_issue_tag(
    State(deployment): State<DeploymentImpl>,
    Path(issue_tag_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    IssueTag::delete(pool, issue_tag_id)
        .await
        .map_err(|e| db_error(e, "failed to delete issue tag"))?;
    Ok(Json(DeleteResponse { txid: local_txid() }))
}
