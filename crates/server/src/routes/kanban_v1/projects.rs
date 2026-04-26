use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Router,
};
use db::models::project::Project;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DeleteResponse, ErrorResponse, MutationResponse, db_error, is_valid_hsl_color, local_txid};
use crate::DeploymentImpl;

#[derive(Debug, Deserialize)]
pub struct ListProjectsQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub color: String,
    pub organization_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BulkUpdateProjectItem {
    pub id: Uuid,
    #[serde(flatten)]
    pub changes: UpdateProjectRequest,
}

#[derive(Debug, Deserialize)]
pub struct BulkUpdateProjectsRequest {
    pub updates: Vec<BulkUpdateProjectItem>,
}

#[derive(Debug, Serialize)]
pub struct BulkUpdateProjectsResponse {
    pub data: Vec<Project>,
    pub txid: i64,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_projects).post(create_project))
        .route("/bulk", post(bulk_update_projects))
        .route("/{project_id}", get(get_project).patch(update_project).delete(delete_project))
}

async fn list_projects(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListProjectsQuery>,
) -> Result<Json<ListProjectsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let projects = Project::find_by_organization(pool, query.organization_id)
        .await
        .map_err(|e| db_error(e, "failed to list projects"))?;
    Ok(Json(ListProjectsResponse { projects }))
}

async fn get_project(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Project>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let project = Project::find_by_id(pool, project_id)
        .await
        .map_err(|e| db_error(e, "failed to get project"))?
        .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "project not found"))?;
    Ok(Json(project))
}

async fn create_project(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<Json<MutationResponse<Project>>, ErrorResponse> {
    if !is_valid_hsl_color(&payload.color) {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Invalid color format. Expected HSL format: 'H S% L%'",
        ));
    }

    let pool = &deployment.db().pool;

    let project = sqlx::query_as!(
        Project,
        r#"INSERT INTO projects (id, name, color, organization_id)
           VALUES ($1, $2, $3, $4)
           RETURNING id as "id!: Uuid",
                     name,
                     default_agent_working_dir,
                     remote_project_id as "remote_project_id: Uuid",
                     color,
                     issue_counter as "issue_counter!: i32",
                     organization_id as "organization_id: Uuid",
                     created_at as "created_at!: DateTime<Utc>",
                     updated_at as "updated_at!: DateTime<Utc>""#,
        Uuid::new_v4(),
        payload.name,
        payload.color,
        payload.organization_id,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| db_error(e, "failed to create project"))?;

    db::models::project_status::ProjectStatus::create_defaults_for_project(pool, project.id)
        .await
        .map_err(|e| db_error(e, "failed to create default statuses"))?;

    Ok(Json(MutationResponse {
        data: project,
        txid: local_txid(),
    }))
}

async fn update_project(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<UpdateProjectRequest>,
) -> Result<Json<MutationResponse<Project>>, ErrorResponse> {
    let pool = &deployment.db().pool;

    Project::find_by_id(pool, project_id)
        .await
        .map_err(|e| db_error(e, "failed to get project"))?
        .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "project not found"))?;

    if let Some(ref color) = payload.color && !is_valid_hsl_color(color) {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Invalid color format. Expected HSL format: 'H S% L%'",
        ));
    }

    let project = Project::update_kanban_fields(pool, project_id, payload.name.as_deref(), payload.color.as_deref(), None)
        .await
        .map_err(|e| db_error(e, "failed to update project"))?;

    Ok(Json(MutationResponse {
        data: project,
        txid: local_txid(),
    }))
}

async fn delete_project(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;

    Project::find_by_id(pool, project_id)
        .await
        .map_err(|e| db_error(e, "failed to get project"))?
        .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "project not found"))?;

    sqlx::query!("DELETE FROM projects WHERE id = $1", project_id)
        .execute(pool)
        .await
        .map_err(|e| db_error(e, "failed to delete project"))?;

    Ok(Json(DeleteResponse { txid: local_txid() }))
}

async fn bulk_update_projects(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<BulkUpdateProjectsRequest>,
) -> Result<Json<BulkUpdateProjectsResponse>, ErrorResponse> {
    if payload.updates.is_empty() {
        return Ok(Json(BulkUpdateProjectsResponse {
            data: vec![],
            txid: 0,
        }));
    }

    let pool = &deployment.db().pool;
    let mut results = Vec::with_capacity(payload.updates.len());

    for item in payload.updates {
        Project::find_by_id(pool, item.id)
            .await
            .map_err(|e| db_error(e, "failed to find project"))?
            .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "project not found"))?;

        if let Some(ref color) = item.changes.color && !is_valid_hsl_color(color) {
            return Err(ErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "Invalid color format. Expected HSL format: 'H S% L%'",
            ));
        }

        let project = Project::update_kanban_fields(
            pool,
            item.id,
            item.changes.name.as_deref(),
            item.changes.color.as_deref(),
            None,
        )
        .await
        .map_err(|e| db_error(e, "failed to update project"))?;

        results.push(project);
    }

    Ok(Json(BulkUpdateProjectsResponse {
        data: results,
        txid: local_txid(),
    }))
}
