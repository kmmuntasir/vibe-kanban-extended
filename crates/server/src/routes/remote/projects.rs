use api_types::{ListProjectsResponse, Project};
use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use super::local_fallback;
use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub(super) struct ListRemoteProjectsQuery {
    pub organization_id: Uuid,
}

pub(super) fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/projects", get(list_remote_projects))
        .route("/projects/{project_id}", get(get_remote_project))
}

async fn list_remote_projects(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListRemoteProjectsQuery>,
) -> Result<ResponseJson<ApiResponse<ListProjectsResponse>>, ApiError> {
    let response = match deployment.remote_client() {
        Ok(client) => client.list_remote_projects(query.organization_id).await?,
        Err(_) => {
            local_fallback::list_projects(&deployment.db().pool, query.organization_id).await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn get_remote_project(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    let project = match deployment.remote_client() {
        Ok(client) => client.get_remote_project(project_id).await?,
        Err(_) => local_fallback::get_project(&deployment.db().pool, project_id).await?,
    };
    Ok(ResponseJson(ApiResponse::success(project)))
}
