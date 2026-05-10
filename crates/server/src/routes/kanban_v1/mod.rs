pub mod issue_assignees;
pub mod issue_comments;
pub mod issue_relationships;
pub mod issue_tags;
mod issues;
pub mod organizations;
pub mod project_statuses;
pub mod projects;
pub mod shape_fallbacks;
pub mod tags;

use axum::{Router, http::StatusCode, response::Json as ResponseJson, routing::get};
use serde::Serialize;
use shape_fallbacks::*;

use crate::DeploymentImpl;

#[derive(Debug, Serialize)]
struct ListHostsResponse {
    hosts: Vec<String>,
}

async fn list_hosts() -> ResponseJson<ListHostsResponse> {
    ResponseJson(ListHostsResponse { hosts: vec![] })
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/hosts", get(list_hosts))
        .route("/fallback/projects", get(fallback_list_projects))
        .route(
            "/fallback/project_statuses",
            get(fallback_list_project_statuses),
        )
        .route("/fallback/tags", get(fallback_list_tags))
        .route("/fallback/issues", get(fallback_list_issues))
        .route(
            "/fallback/issue_assignees",
            get(fallback_list_issue_assignees),
        )
        .route("/fallback/issue_tags", get(fallback_list_issue_tags))
        .route(
            "/fallback/issue_relationships",
            get(fallback_list_issue_relationships),
        )
        .route(
            "/fallback/issue_comments",
            get(fallback_list_issue_comments),
        )
        .nest("/projects", projects::router())
        .nest("/project_statuses", project_statuses::router())
        .nest("/tags", tags::router())
        .nest("/issue_tags", issue_tags::router())
        .nest("/issue_assignees", issue_assignees::router())
        .nest("/issue_relationships", issue_relationships::router())
        .nest("/organizations", organizations::router())
        .nest("/issue_comments", issue_comments::router())
        .merge(issues::router())
}

// ---------------------------------------------------------------------------
// Shared types for entity mutation routes
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct MutationResponse<T> {
    pub data: T,
    pub txid: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct DeleteResponse {
    pub txid: i64,
}

pub fn local_txid() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn db_error(error: sqlx::Error, fallback: &str) -> shape_fallbacks::ErrorResponse {
    match &error {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            shape_fallbacks::ErrorResponse::new(StatusCode::CONFLICT, "resource already exists")
        }
        sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
            shape_fallbacks::ErrorResponse::new(StatusCode::NOT_FOUND, "related resource not found")
        }
        _ => shape_fallbacks::ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, fallback),
    }
}

pub fn is_valid_hsl_color(color: &str) -> bool {
    let parts: Vec<&str> = color.split(' ').collect();
    if parts.len() != 3 {
        return false;
    }
    let Ok(h) = parts[0].parse::<u16>() else {
        return false;
    };
    if h > 360 {
        return false;
    }
    let Some(s_str) = parts[1].strip_suffix('%') else {
        return false;
    };
    let Ok(s) = s_str.parse::<u8>() else {
        return false;
    };
    if s > 100 {
        return false;
    }
    let Some(l_str) = parts[2].strip_suffix('%') else {
        return false;
    };
    let Ok(l) = l_str.parse::<u8>() else {
        return false;
    };
    if l > 100 {
        return false;
    }
    true
}
