use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Router,
};
use db::models::project_status::ProjectStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DeleteResponse, ErrorResponse, MutationResponse, db_error, is_valid_hsl_color, local_txid};
use crate::DeploymentImpl;

#[derive(Debug, Deserialize)]
pub struct ListProjectStatusesQuery {
    pub project_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ListProjectStatusesResponse {
    pub project_statuses: Vec<ProjectStatus>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectStatusRequest {
    pub project_id: Uuid,
    pub name: String,
    pub color: String,
    pub sort_order: i32,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectStatusRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i32>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BulkUpdateProjectStatusItem {
    pub id: Uuid,
    #[serde(flatten)]
    pub changes: UpdateProjectStatusRequest,
}

#[derive(Debug, Deserialize)]
pub struct BulkUpdateProjectStatusesRequest {
    pub updates: Vec<BulkUpdateProjectStatusItem>,
}

#[derive(Debug, Serialize)]
pub struct BulkUpdateProjectStatusesResponse {
    pub data: Vec<ProjectStatus>,
    pub txid: i64,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_project_statuses).post(create_project_status))
        .route("/bulk", post(bulk_update_project_statuses))
        .route(
            "/{status_id}",
            get(get_project_status)
                .patch(update_project_status)
                .delete(delete_project_status),
        )
}

async fn list_project_statuses(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListProjectStatusesQuery>,
) -> Result<Json<ListProjectStatusesResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let statuses = ProjectStatus::find_by_project(pool, query.project_id)
        .await
        .map_err(|e| db_error(e, "failed to list project statuses"))?;
    Ok(Json(ListProjectStatusesResponse {
        project_statuses: statuses,
    }))
}

async fn get_project_status(
    State(deployment): State<DeploymentImpl>,
    Path(status_id): Path<Uuid>,
) -> Result<Json<ProjectStatus>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let status = sqlx::query_as!(
        ProjectStatus,
        r#"SELECT id as "id!: Uuid",
                  project_id as "project_id!: Uuid",
                  name,
                  color,
                  sort_order as "sort_order!: i32",
                  hidden as "hidden: bool",
                  created_at as "created_at!: DateTime<Utc>"
           FROM project_statuses
           WHERE id = $1"#,
        status_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| db_error(e, "failed to get project status"))?
    .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "project status not found"))?;
    Ok(Json(status))
}

async fn create_project_status(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateProjectStatusRequest>,
) -> Result<Json<MutationResponse<ProjectStatus>>, ErrorResponse> {
    if !is_valid_hsl_color(&payload.color) {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Invalid color format. Expected HSL format: 'H S% L%'",
        ));
    }

    let pool = &deployment.db().pool;
    let status = ProjectStatus::create(
        pool,
        payload.project_id,
        &payload.name,
        &payload.color,
        payload.sort_order,
        payload.hidden,
    )
    .await
    .map_err(|e| db_error(e, "failed to create project status"))?;

    Ok(Json(MutationResponse {
        data: status,
        txid: local_txid(),
    }))
}

async fn update_project_status(
    State(deployment): State<DeploymentImpl>,
    Path(status_id): Path<Uuid>,
    Json(payload): Json<UpdateProjectStatusRequest>,
) -> Result<Json<MutationResponse<ProjectStatus>>, ErrorResponse> {
    let pool = &deployment.db().pool;

    if let Some(ref color) = payload.color && !is_valid_hsl_color(color) {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Invalid color format. Expected HSL format: 'H S% L%'",
        ));
    }

    let status = ProjectStatus::update(
        pool,
        status_id,
        payload.name.as_deref(),
        payload.color.as_deref(),
        payload.sort_order,
        payload.hidden,
    )
    .await
    .map_err(|e| db_error(e, "failed to update project status"))?;

    Ok(Json(MutationResponse {
        data: status,
        txid: local_txid(),
    }))
}

async fn delete_project_status(
    State(deployment): State<DeploymentImpl>,
    Path(status_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    ProjectStatus::delete(pool, status_id)
        .await
        .map_err(|e| db_error(e, "failed to delete project status"))?;
    Ok(Json(DeleteResponse { txid: local_txid() }))
}

async fn bulk_update_project_statuses(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<BulkUpdateProjectStatusesRequest>,
) -> Result<Json<BulkUpdateProjectStatusesResponse>, ErrorResponse> {
    if payload.updates.is_empty() {
        return Ok(Json(BulkUpdateProjectStatusesResponse {
            data: vec![],
            txid: 0,
        }));
    }

    let pool = &deployment.db().pool;
    let mut results = Vec::with_capacity(payload.updates.len());

    for item in payload.updates {
        if let Some(ref color) = item.changes.color && !is_valid_hsl_color(color) {
            return Err(ErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "Invalid color format. Expected HSL format: 'H S% L%'",
            ));
        }

        let status = ProjectStatus::update(
            pool,
            item.id,
            item.changes.name.as_deref(),
            item.changes.color.as_deref(),
            item.changes.sort_order,
            item.changes.hidden,
        )
        .await
        .map_err(|e| db_error(e, "failed to update project status"))?;

        results.push(status);
    }

    Ok(Json(BulkUpdateProjectStatusesResponse {
        data: results,
        txid: local_txid(),
    }))
}
