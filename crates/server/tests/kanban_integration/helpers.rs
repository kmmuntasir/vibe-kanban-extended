use std::str::FromStr;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use db::{DBService, models::project::Project};
use local_deployment::first_boot::initialize_if_empty;
use serde::Deserialize;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Query / response types for test handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OrgQuery {
    organization_id: Uuid,
}

#[derive(Debug, serde::Serialize)]
struct ProjectsResponse {
    projects: Vec<Project>,
}

// ---------------------------------------------------------------------------
// Test handlers — mirror production DB queries, using DBService as state
// ---------------------------------------------------------------------------

async fn list_projects(
    State(db): State<DBService>,
    Query(query): Query<OrgQuery>,
) -> (StatusCode, Json<ProjectsResponse>) {
    let projects = Project::find_by_organization(&db.pool, query.organization_id)
        .await
        .expect("failed to list projects");
    (StatusCode::OK, Json(ProjectsResponse { projects }))
}

// ---------------------------------------------------------------------------
// Core test utilities
// ---------------------------------------------------------------------------

/// Creates an in-memory SQLite pool with all migrations applied.
/// Pattern from crates/local-deployment/src/first_boot.rs:99-112.
pub async fn setup_pool() -> SqlitePool {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .journal_mode(SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    let migrator = sqlx::migrate!("../db/migrations");
    migrator.run(&pool).await.unwrap();
    pool
}

/// Seeds first-boot data into the pool. Returns DBService wrapping the pool.
pub async fn seed_db(pool: SqlitePool) -> DBService {
    let db = DBService { pool };
    initialize_if_empty(&db).await.unwrap();
    db
}

/// Returns (organization_id, project_id, first_status_id) from seeded database.
pub async fn get_seeded_ids(pool: &SqlitePool) -> (Uuid, Uuid, Uuid) {
    let org_id: Uuid = sqlx::query_scalar("SELECT id FROM organizations LIMIT 1")
        .fetch_one(pool)
        .await
        .unwrap();
    let project_id: Uuid = sqlx::query_scalar("SELECT id FROM projects LIMIT 1")
        .fetch_one(pool)
        .await
        .unwrap();
    let first_status_id: Uuid =
        sqlx::query_scalar("SELECT id FROM project_statuses ORDER BY sort_order LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap();
    (org_id, project_id, first_status_id)
}

/// Builds test app: in-memory DB + seed + axum Router.
/// Returns Router<()> ready for tower::ServiceExt::oneshot().
pub async fn setup_test_app() -> Router {
    let pool = setup_pool().await;
    let db = seed_db(pool).await;
    Router::new()
        .route("/fallback/projects", get(list_projects))
        .with_state(db)
}
