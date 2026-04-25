use axum::Router;

use crate::DeploymentImpl;

mod issues;
mod organizations;
mod project_statuses;
mod projects;
mod tags;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .merge(organizations::router())
        .merge(projects::router())
        .merge(project_statuses::router())
        .merge(issues::router())
        .merge(tags::router())
}
