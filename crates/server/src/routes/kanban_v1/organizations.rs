use api_types::{ListOrganizationsResponse, MemberRole, OrganizationWithRole};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use db::models::organization::Organization;
use deployment::Deployment;
use uuid::Uuid;

use super::shape_fallbacks::ErrorResponse;
use crate::DeploymentImpl;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_organizations))
        .route("/{id}/members", get(list_organization_members))
}

pub async fn list_organization_members(
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    Ok(Json(serde_json::json!({ "members": [] })))
}

pub async fn list_organizations(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ListOrganizationsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let orgs = Organization::find_all(pool).await.map_err(|e| {
        tracing::error!(?e, "failed to list organizations");
        ErrorResponse::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "failed to list organizations",
        )
    })?;

    let organizations: Vec<OrganizationWithRole> = orgs
        .into_iter()
        .map(|org| OrganizationWithRole {
            id: org.id,
            name: org.name,
            slug: org.slug,
            is_personal: org.is_personal,
            issue_prefix: org.issue_prefix,
            created_at: org.created_at,
            updated_at: org.updated_at,
            user_role: MemberRole::Admin,
        })
        .collect();

    Ok(Json(ListOrganizationsResponse { organizations }))
}
