mod issues;
pub mod shape_fallbacks;

use axum::{Router, routing::get};
use crate::DeploymentImpl;
use shape_fallbacks::*;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/fallback/projects", get(fallback_list_projects))
        .route("/fallback/project_statuses", get(fallback_list_project_statuses))
        .route("/fallback/tags", get(fallback_list_tags))
        .route("/fallback/issues", get(fallback_list_issues))
        .route("/fallback/issue_assignees", get(fallback_list_issue_assignees))
        .route("/fallback/issue_tags", get(fallback_list_issue_tags))
        .route("/fallback/issue_relationships", get(fallback_list_issue_relationships))
        .merge(issues::router())
}
