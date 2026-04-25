use axum::{
    Router,
    extract::{Json, Path, State},
    response::Json as ResponseJson,
    routing::{get, patch, post},
};
use db::models::organization::Organization;
use serde::Deserialize;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/organizations", get(list_organizations).post(create_organization))
        .route("/organizations/{org_id}", get(get_organization).patch(update_organization))
}

#[derive(Deserialize)]
struct CreateOrganizationRequest {
    name: String,
}

async fn list_organizations(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let organizations = Organization::find_all(pool).await?;
    Ok(ResponseJson(serde_json::json!({ "organizations": organizations })))
}

async fn get_organization(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
) -> Result<ResponseJson<Organization>, ApiError> {
    let pool = &deployment.db().pool;
    let org = Organization::find_by_id(pool, org_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Organization not found".into()))?;
    Ok(ResponseJson(org))
}

async fn create_organization(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    let slug = req.name.to_lowercase().replace(' ', "-");
    let org = Organization::create(pool, id, &req.name, &slug, "ISS").await?;
    Ok(ResponseJson(serde_json::json!({ "data": org, "txid": 0 })))
}

#[derive(Deserialize)]
struct UpdateOrganizationRequest {
    name: Option<String>,
}

async fn update_organization(
    State(deployment): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
    Json(_req): Json<UpdateOrganizationRequest>,
) -> Result<ResponseJson<serde_json::Value>, ApiError> {
    // For local mode, just return the existing org
    let pool = &deployment.db().pool;
    let org = Organization::find_by_id(pool, org_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Organization not found".into()))?;
    Ok(ResponseJson(serde_json::json!({ "data": org, "txid": 0 })))
}
