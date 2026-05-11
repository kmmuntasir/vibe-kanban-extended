use api_types::{CreateWorkspaceRequest, PullRequestStatus, UpsertPullRequestRequest};
use axum::{
    Extension, Json, Router,
    extract::{Path as AxumPath, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{delete, post},
};
use db::models::{merge::MergeStatus, pull_request::PullRequest, workspace::Workspace};
use deployment::Deployment;
use serde::Deserialize;
use services::services::{diff_stream, remote_client::RemoteClientError, remote_sync};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_workspace_middleware};

#[derive(Debug, Deserialize)]
pub struct LinkWorkspaceRequest {
    pub project_id: Uuid,
    pub issue_id: Uuid,
}

pub async fn link_workspace(
    Extension(workspace): Extension<Workspace>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<LinkWorkspaceRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    // If remote client is available, sync to remote API
    if let Ok(client) = deployment.remote_client() {
        let stats =
            diff_stream::compute_diff_stats(&deployment.db().pool, deployment.git(), &workspace)
                .await;

        client
            .create_workspace(CreateWorkspaceRequest {
                project_id: payload.project_id,
                local_workspace_id: workspace.id,
                issue_id: payload.issue_id,
                name: workspace.name.clone(),
                archived: Some(workspace.archived),
                files_changed: stats.as_ref().map(|s| s.files_changed as i32),
                lines_added: stats.as_ref().map(|s| s.lines_added as i32),
                lines_removed: stats.as_ref().map(|s| s.lines_removed as i32),
            })
            .await?;

        {
            let pool = deployment.db().pool.clone();
            let ws_id = workspace.id;
            let client = client.clone();
            tokio::spawn(async move {
                let pull_requests = match PullRequest::find_by_workspace_id(&pool, ws_id).await {
                    Ok(prs) => prs,
                    Err(e) => {
                        tracing::error!(
                            "Failed to fetch PRs for workspace {} during link: {}",
                            ws_id,
                            e
                        );
                        return;
                    }
                };
                for pr in pull_requests {
                    let pr_status = match pr.pr_status {
                        MergeStatus::Open => PullRequestStatus::Open,
                        MergeStatus::Merged => PullRequestStatus::Merged,
                        MergeStatus::Closed => PullRequestStatus::Closed,
                        MergeStatus::Unknown => continue,
                    };
                    remote_sync::sync_pr_to_remote(
                        &client,
                        UpsertPullRequestRequest {
                            url: pr.pr_url,
                            number: pr.pr_number as i32,
                            status: pr_status,
                            merged_at: pr.merged_at,
                            merge_commit_sha: pr.merge_commit_sha,
                            target_branch_name: pr.target_branch_name,
                            local_workspace_id: ws_id,
                        },
                    )
                    .await;
                }
            });
        }
    } else {
        // Local mode: store link in local DB
        sqlx::query(
            "UPDATE workspaces SET issue_id = $1, project_id = $2, updated_at = datetime('now') WHERE id = $3",
        )
        .bind(payload.issue_id)
        .bind(payload.project_id)
        .bind(workspace.id)
        .execute(&deployment.db().pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

        // Auto-move issue to "In progress" if currently in Backlog or To do
        auto_move_issue_to_in_progress(&deployment, payload.issue_id, payload.project_id).await?;
    }

    Ok(ResponseJson(ApiResponse::success(())))
}

pub async fn unlink_workspace(
    AxumPath(workspace_id): AxumPath<uuid::Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    if let Ok(client) = deployment.remote_client() {
        match client.delete_workspace(workspace_id).await {
            Ok(()) => return Ok(ResponseJson(ApiResponse::success(()))),
            Err(RemoteClientError::Http { status: 404, .. }) => {
                return Ok(ResponseJson(ApiResponse::success(())));
            }
            Err(e) => return Err(e.into()),
        }
    }

    // Local mode: clear the workspace-issue link in the local DB
    sqlx::query(
        "UPDATE workspaces SET issue_id = NULL, project_id = NULL, updated_at = datetime('now', 'subsec') WHERE id = $1",
    )
    .bind(workspace_id)
    .execute(&deployment.db().pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(ResponseJson(ApiResponse::success(())))
}

/// In local mode, move an issue to "In progress" when a workspace is linked,
/// mirroring the remote server's sync_issue_from_workspace_created logic.
async fn auto_move_issue_to_in_progress(
    deployment: &DeploymentImpl,
    issue_id: Uuid,
    project_id: Uuid,
) -> Result<(), ApiError> {
    let pool = &deployment.db().pool;

    // Find "In progress" status for this project
    #[derive(sqlx::FromRow)]
    struct StatusRow {
        id: Uuid,
        #[allow(dead_code)]
        name: String,
    }
    let in_progress: Option<StatusRow> = sqlx::query_as(
        r#"SELECT id, name FROM project_statuses
           WHERE project_id = $1 AND lower(name) = 'in progress'"#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let Some(in_progress) = in_progress else {
        return Ok(());
    };

    // Get current issue status name
    #[derive(sqlx::FromRow)]
    struct NameRow {
        name: String,
    }
    let current: Option<NameRow> = sqlx::query_as(
        r#"SELECT ps.name
           FROM issues i
           JOIN project_statuses ps ON ps.id = i.status_id
           WHERE i.id = $1"#,
    )
    .bind(issue_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let Some(current) = current else {
        return Ok(());
    };

    let current_lower = current.name.to_lowercase();
    if current_lower != "backlog" && current_lower != "to do" {
        return Ok(());
    };

    // Move to "In progress"
    sqlx::query(
        "UPDATE issues SET status_id = $1, updated_at = datetime('now', 'subsec') WHERE id = $2",
    )
    .bind(in_progress.id)
    .bind(issue_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(())
}

/// In local mode, move an issue to "In review" when a PR is created for its
/// workspace, mirroring the remote server's PR-open workflow signal.
pub(super) async fn auto_move_issue_to_in_review(
    deployment: &DeploymentImpl,
    issue_id: Uuid,
    project_id: Uuid,
) -> Result<(), ApiError> {
    let pool = &deployment.db().pool;

    #[derive(sqlx::FromRow)]
    struct StatusRow {
        id: Uuid,
        #[allow(dead_code)]
        name: String,
    }
    let in_review: Option<StatusRow> = sqlx::query_as(
        r#"SELECT id, name FROM project_statuses
           WHERE project_id = $1 AND lower(name) = 'in review'"#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let Some(in_review) = in_review else {
        return Ok(());
    };

    #[derive(sqlx::FromRow)]
    struct NameRow {
        name: String,
    }
    let current: Option<NameRow> = sqlx::query_as(
        r#"SELECT ps.name
           FROM issues i
           JOIN project_statuses ps ON ps.id = i.status_id
           WHERE i.id = $1"#,
    )
    .bind(issue_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let Some(current) = current else {
        return Ok(());
    };

    let current_lower = current.name.to_lowercase();
    if current_lower != "to do" && current_lower != "backlog" && current_lower != "in progress" {
        return Ok(());
    };

    sqlx::query(
        "UPDATE issues SET status_id = $1, updated_at = datetime('now', 'subsec') WHERE id = $2",
    )
    .bind(in_review.id)
    .bind(issue_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(())
}

/// In local mode, move an issue to "Done" when its workspace is merged,
/// mirroring the remote server's WorkMerged signal logic.
pub(super) async fn auto_move_issue_to_done(
    deployment: &DeploymentImpl,
    issue_id: Uuid,
    project_id: Uuid,
) -> Result<(), ApiError> {
    let pool = &deployment.db().pool;

    #[derive(sqlx::FromRow)]
    struct StatusRow {
        id: Uuid,
        #[allow(dead_code)]
        name: String,
    }
    let done: Option<StatusRow> = sqlx::query_as(
        r#"SELECT id, name FROM project_statuses
           WHERE project_id = $1 AND lower(name) = 'done'"#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let Some(done) = done else {
        return Ok(());
    };

    sqlx::query(
        "UPDATE issues SET status_id = $1, updated_at = datetime('now', 'subsec') WHERE id = $2",
    )
    .bind(done.id)
    .bind(issue_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(())
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let post_router = Router::new()
        .route("/", post(link_workspace))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_workspace_middleware,
        ));

    let delete_router = Router::new().route("/", delete(unlink_workspace));

    post_router.merge(delete_router)
}
