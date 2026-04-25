use axum::{
    Router,
    extract::{Json, Path, Query, State},
    response::Json as ResponseJson,
    routing::{delete, get, patch, post},
};
use db::models::project::{CreateKanbanProject, KanbanProject, UpdateKanbanProject};
use serde::Deserialize;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/{project_id}", get(get_project).patch(update_project).delete(delete_project))
}

#[derive(Deserialize)]
struct ListProjectsQuery {
    organization_id: Option<Uuid>,
}

async fn list_projects(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListProjectsQuery>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let projects = if let Some(org_id) = query.organization_id {
        KanbanProject::find_by_organization(pool, org_id).await?
    } else {
        // Return all projects with organization_id
        let orgs = db::models::organization::Organization::find_all(pool).await?;
        let mut all = Vec::new();
        for org in orgs {
            all.extend(KanbanProject::find_by_organization(pool, org.id).await?);
        }
        all
    };
    Ok(ResponseJson(serde_json::json!({ "projects": projects })))
}

async fn get_project(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<KanbanProject>, ApiError> {
    let pool = &deployment.db().pool;
    let project = KanbanProject::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Project not found".into()))?;
    Ok(ResponseJson(project))
}

async fn create_project(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateKanbanProject>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let project = KanbanProject::create(pool, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": project, "txid": 0 })))
}

async fn update_project(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<UpdateKanbanProject>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let project = KanbanProject::update_project(pool, project_id, &req).await?;
    Ok(ResponseJson(serde_json::json!({ "data": project, "txid": 0 })))
}

async fn delete_project(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    KanbanProject::delete_project(pool, project_id).await?;
    Ok(ResponseJson(serde_json::json!({ "id": project_id.to_string(), "txid": 0 })))
}
