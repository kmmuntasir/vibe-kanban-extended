use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use db::models::issue_relationship::IssueRelationship;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DeleteResponse, ErrorResponse, MutationResponse, db_error, local_txid};
use crate::DeploymentImpl;

#[derive(Debug, Deserialize)]
pub struct ListIssueRelationshipsQuery {
    pub issue_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ListIssueRelationshipsResponse {
    pub issue_relationships: Vec<IssueRelationship>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIssueRelationshipRequest {
    pub issue_id: Uuid,
    pub related_issue_id: Uuid,
    pub relationship_type: String,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/",
            get(list_issue_relationships).post(create_issue_relationship),
        )
        .route(
            "/{relationship_id}",
            get(get_issue_relationship).delete(delete_issue_relationship),
        )
}

async fn list_issue_relationships(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssueRelationshipsQuery>,
) -> Result<Json<ListIssueRelationshipsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let issue_relationships = IssueRelationship::find_by_issue(pool, query.issue_id)
        .await
        .map_err(|e| db_error(e, "failed to list issue relationships"))?;
    Ok(Json(ListIssueRelationshipsResponse {
        issue_relationships,
    }))
}

async fn get_issue_relationship(
    State(deployment): State<DeploymentImpl>,
    Path(relationship_id): Path<Uuid>,
) -> Result<Json<IssueRelationship>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let relationship = sqlx::query_as!(
        IssueRelationship,
        r#"SELECT id as "id!: Uuid",
                  issue_id as "issue_id!: Uuid",
                  related_issue_id as "related_issue_id!: Uuid",
                  relationship_type,
                  created_at as "created_at!: DateTime<Utc>"
           FROM issue_relationships
           WHERE id = $1"#,
        relationship_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| db_error(e, "failed to get issue relationship"))?
    .ok_or_else(|| {
        ErrorResponse::new(StatusCode::NOT_FOUND, "issue relationship not found")
    })?;
    Ok(Json(relationship))
}

async fn create_issue_relationship(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateIssueRelationshipRequest>,
) -> Result<Json<MutationResponse<IssueRelationship>>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let relationship = IssueRelationship::create(
        pool,
        payload.issue_id,
        payload.related_issue_id,
        &payload.relationship_type,
    )
    .await
    .map_err(|e| db_error(e, "failed to create issue relationship"))?;
    Ok(Json(MutationResponse {
        data: relationship,
        txid: local_txid(),
    }))
}

async fn delete_issue_relationship(
    State(deployment): State<DeploymentImpl>,
    Path(relationship_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    IssueRelationship::delete(pool, relationship_id)
        .await
        .map_err(|e| db_error(e, "failed to delete issue relationship"))?;
    Ok(Json(DeleteResponse { txid: local_txid() }))
}
