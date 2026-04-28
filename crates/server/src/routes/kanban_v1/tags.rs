use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use db::models::kanban_tag::KanbanTag;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    DeleteResponse, ErrorResponse, MutationResponse, db_error, is_valid_hsl_color, local_txid,
};
use crate::DeploymentImpl;

#[derive(Debug, Deserialize)]
pub struct ListTagsQuery {
    pub project_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ListTagsResponse {
    pub tags: Vec<KanbanTag>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub project_id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_tags).post(create_tag))
        .route(
            "/{tag_id}",
            get(get_tag).patch(update_tag).delete(delete_tag),
        )
}

async fn list_tags(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListTagsQuery>,
) -> Result<Json<ListTagsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let tags = KanbanTag::find_by_project(pool, query.project_id)
        .await
        .map_err(|e| db_error(e, "failed to list tags"))?;
    Ok(Json(ListTagsResponse { tags }))
}

async fn get_tag(
    State(deployment): State<DeploymentImpl>,
    Path(tag_id): Path<Uuid>,
) -> Result<Json<KanbanTag>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let tag = KanbanTag::find_by_id(pool, tag_id)
        .await
        .map_err(|e| db_error(e, "failed to get tag"))?
        .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "tag not found"))?;
    Ok(Json(tag))
}

async fn create_tag(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateTagRequest>,
) -> Result<Json<MutationResponse<KanbanTag>>, ErrorResponse> {
    if !is_valid_hsl_color(&payload.color) {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Invalid color format. Expected HSL format: 'H S% L%'",
        ));
    }

    let pool = &deployment.db().pool;
    let tag = KanbanTag::create(pool, payload.project_id, &payload.name, &payload.color)
        .await
        .map_err(|e| db_error(e, "failed to create tag"))?;

    Ok(Json(MutationResponse {
        data: tag,
        txid: local_txid(),
    }))
}

async fn update_tag(
    State(deployment): State<DeploymentImpl>,
    Path(tag_id): Path<Uuid>,
    Json(payload): Json<UpdateTagRequest>,
) -> Result<Json<MutationResponse<KanbanTag>>, ErrorResponse> {
    let pool = &deployment.db().pool;

    if let Some(ref color) = payload.color
        && !is_valid_hsl_color(color)
    {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Invalid color format. Expected HSL format: 'H S% L%'",
        ));
    }

    let tag = KanbanTag::update(
        pool,
        tag_id,
        payload.name.as_deref(),
        payload.color.as_deref(),
    )
    .await
    .map_err(|e| db_error(e, "failed to update tag"))?;

    Ok(Json(MutationResponse {
        data: tag,
        txid: local_txid(),
    }))
}

async fn delete_tag(
    State(deployment): State<DeploymentImpl>,
    Path(tag_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    KanbanTag::delete(pool, tag_id)
        .await
        .map_err(|e| db_error(e, "failed to delete tag"))?;
    Ok(Json(DeleteResponse { txid: local_txid() }))
}
