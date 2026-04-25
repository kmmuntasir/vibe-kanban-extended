use axum::{
    Router,
    extract::{Json, Path, Query, State},
    response::Json as ResponseJson,
    routing::{delete, get, patch, post},
};
use db::models::kanban_tag::{CreateKanbanTag, KanbanTag, UpdateKanbanTag};
use serde::Deserialize;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/tags", get(list_tags).post(create_tag))
        .route("/tags/{tag_id}", get(get_tag).patch(update_tag).delete(delete_tag))
}

#[derive(Deserialize)]
struct ListTagsQuery {
    project_id: Option<Uuid>,
}

async fn list_tags(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListTagsQuery>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let project_id = query.project_id
        .ok_or_else(|| ApiError::BadRequest("project_id is required".into()))?;
    let tags = KanbanTag::find_by_project(pool, project_id).await?;
    Ok(ResponseJson(serde_json::json!({ "tags": tags })))
}

async fn get_tag(
    State(deployment): State<DeploymentImpl>,
    Path(tag_id): Path<Uuid>,
) -> Result<ResponseJson<KanbanTag>, ApiError> {
    let pool = &deployment.db().pool;
    let tag = KanbanTag::find_by_id(pool, tag_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Tag not found".into()))?;
    Ok(ResponseJson(tag))
}

async fn create_tag(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateKanbanTag>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let tag = KanbanTag::create(pool, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": tag, "txid": 0 })))
}

async fn update_tag(
    State(deployment): State<DeploymentImpl>,
    Path(tag_id): Path<Uuid>,
    Json(req): Json<UpdateKanbanTag>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let tag = KanbanTag::update(pool, tag_id, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": tag, "txid": 0 })))
}

async fn delete_tag(
    State(deployment): State<DeploymentImpl>,
    Path(tag_id): Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    KanbanTag::delete(pool, tag_id).await?;
    Ok(ResponseJson(serde_json::json!({ "id": tag_id.to_string(), "txid": 0 })))
}
