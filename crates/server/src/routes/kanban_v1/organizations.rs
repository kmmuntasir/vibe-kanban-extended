use api_types::{ListOrganizationsResponse, MemberRole, OrganizationWithRole};
use axum::{Json, Router, extract::State, routing::get};
use db::models::organization::Organization;
use deployment::Deployment;
use crate::DeploymentImpl;
use super::shape_fallbacks::ErrorResponse;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/organizations", get(list_organizations))
}

pub async fn list_organizations(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ListOrganizationsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let orgs = Organization::find_all(pool)
        .await
        .map_err(|e| {
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
