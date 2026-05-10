use api_types::{
    CreateIssueCommentRequest, IssueComment, ListIssueCommentsQuery, ListIssueCommentsResponse,
    UpdateIssueCommentRequest,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use db::models::issue_comment::IssueComment as DbComment;
use deployment::Deployment;
use uuid::Uuid;

use super::{DeleteResponse, ErrorResponse, MutationResponse, db_error, local_txid};
use crate::DeploymentImpl;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/issue_comments",
            get(list_issue_comments).post(create_issue_comment),
        )
        .route(
            "/issue_comments/{id}",
            get(get_issue_comment)
                .patch(update_issue_comment)
                .delete(delete_issue_comment),
        )
}

fn to_api(c: DbComment) -> IssueComment {
    IssueComment {
        id: c.id,
        issue_id: c.issue_id,
        author_id: c.author_id,
        parent_id: c.parent_id,
        message: c.message,
        created_at: c.created_at,
        updated_at: c.updated_at,
    }
}

async fn list_issue_comments(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssueCommentsQuery>,
) -> Result<Json<ListIssueCommentsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let comments = DbComment::find_by_issue(pool, query.issue_id)
        .await
        .map_err(|e| db_error(e, "failed to list issue comments"))?;

    Ok(Json(ListIssueCommentsResponse {
        issue_comments: comments.into_iter().map(to_api).collect(),
    }))
}

async fn get_issue_comment(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<IssueComment>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let comment = DbComment::find_by_id(pool, id)
        .await
        .map_err(|e| db_error(e, "failed to get issue comment"))?
        .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "comment not found"))?;

    Ok(Json(to_api(comment)))
}

async fn create_issue_comment(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateIssueCommentRequest>,
) -> Result<Json<MutationResponse<IssueComment>>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let local_user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let comment = DbComment::create(
        pool,
        body.issue_id,
        Some(local_user_id),
        body.parent_id,
        &body.message,
    )
    .await
    .map_err(|e| db_error(e, "failed to create issue comment"))?;

    Ok(Json(MutationResponse {
        data: to_api(comment),
        txid: local_txid(),
    }))
}

async fn update_issue_comment(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateIssueCommentRequest>,
) -> Result<Json<MutationResponse<IssueComment>>, ErrorResponse> {
    let pool = &deployment.db().pool;

    let original = DbComment::find_by_id(pool, id)
        .await
        .map_err(|e| db_error(e, "failed to find issue comment"))?
        .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "comment not found"))?;

    if let Some(ref parent_id) = body.parent_id {
        DbComment::update_parent_id(pool, id, *parent_id)
            .await
            .map_err(|e| db_error(e, "failed to update comment parent_id"))?;
    }

    let updated = if let Some(msg) = &body.message {
        DbComment::update(pool, id, msg)
            .await
            .map_err(|e| db_error(e, "failed to update issue comment"))?
    } else {
        original.clone()
    };

    let final_parent_id = body.parent_id.flatten().or(updated.parent_id);
    let final_message = body.message.unwrap_or(updated.message);
    let final_updated_at = updated.updated_at;

    Ok(Json(MutationResponse {
        data: IssueComment {
            id: updated.id,
            issue_id: updated.issue_id,
            author_id: updated.author_id,
            parent_id: final_parent_id,
            message: final_message,
            created_at: updated.created_at,
            updated_at: final_updated_at,
        },
        txid: local_txid(),
    }))
}

async fn delete_issue_comment(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    DbComment::delete(pool, id)
        .await
        .map_err(|e| db_error(e, "failed to delete issue comment"))?;

    Ok(Json(DeleteResponse { txid: local_txid() }))
}
