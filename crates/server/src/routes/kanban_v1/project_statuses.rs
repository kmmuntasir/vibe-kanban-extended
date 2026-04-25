use axum::{
    Router,
    extract::{Json, Path, Query, State},
    response::Json as ResponseJson,
    routing::{delete, get, patch, post},
};
use db::models::project_status::{CreateProjectStatus, ProjectStatus, UpdateProjectStatus};
use serde::Deserialize;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/project_statuses", get(list_statuses).post(create_status))
        .route("/project_statuses/{status_id}", get(get_status).patch(update_status).delete(delete_status))
}

#[derive(Deserialize)]
struct ListStatusesQuery {
    project_id: Option<Uuid>,
}

async fn list_statuses(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListStatusesQuery>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let project_id = query.project_id
        .ok_or_else(|| ApiError::BadRequest("project_id is required".into()))?;
    let statuses = ProjectStatus::find_by_project(pool, project_id).await?;
    Ok(ResponseJson(serde_json::json!({ "project_statuses": statuses })))
}

async fn get_status(
    State(deployment): State<DeploymentImpl>,
    Path(status_id): Path<Uuid>,
) -> Result<ResponseJson<ProjectStatus>, ApiError> {
    let pool = &deployment.db().pool;
    let status = ProjectStatus::find_by_id(pool, status_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Status not found".into()))?;
    Ok(ResponseJson(status))
}

async fn create_status(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateProjectStatus>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let status = ProjectStatus::create(pool, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": status, "txid": 0 })))
}

async fn update_status(
    State(deployment): State<DeploymentImpl>,
    Path(status_id): Path<Uuid>,
    Json(req): Json<UpdateProjectStatus>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let status = ProjectStatus::update(pool, status_id, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": status, "txid": 0 })))
}

async fn delete_status(
    State(deployment): State<DeploymentImpl>,
    Path(status_id): Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    ProjectStatus::delete(pool, status_id).await?;
    Ok(ResponseJson(serde_json::json!({ "id": status_id.to_string(), "txid": 0 })))
}
