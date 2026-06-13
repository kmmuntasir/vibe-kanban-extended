use api_types::Workspace;
use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::workspace::WorkspaceError;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub(super) fn router() -> Router<DeploymentImpl> {
    Router::new().route(
        "/workspaces/by-local-id/{local_workspace_id}",
        get(get_workspace_by_local_id),
    )
}

async fn get_workspace_by_local_id(
    State(deployment): State<DeploymentImpl>,
    Path(local_workspace_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Workspace>>, ApiError> {
    // Local mode has no remote workspace registry. Return 404 (not 400) — the MCP
    // treats non-success here as "no remote context" and degrades gracefully.
    let workspace = match deployment.remote_client() {
        Ok(client) => client.get_workspace_by_local_id(local_workspace_id).await?,
        Err(_) => return Err(ApiError::Workspace(WorkspaceError::WorkspaceNotFound)),
    };
    Ok(ResponseJson(ApiResponse::success(workspace)))
}
