# Local Kanban — Complete Fix Plan

**Date:** 2026-05-10
**Branch:** `analysis/find-out-feasibility-of-local-run`
**Goal:** Make the kanban board run completely locally (SQLite, no cloud dependency, no sign-in)

---

## Summary of Changes

| # | Change | Files | Why |
|---|--------|-------|-----|
| 1 | Remove `option_env!` from 6 sites | 3 files | Unblock auth bypass + local routing |
| 2 | Clean up build.rs injections | 2 files | Prevent compile-time env var leaking |
| 3 | Add `/v1/organizations` route | 2 files (1 new, 1 mod) | Frontend org discovery — currently 404 |
| 4 | Add `/v1/issue_comments` routes | 3 files (2 new, 1 mod) | Comment CRUD + fallback |
| 5 | Wipe local DB + rebuild | terminal | Activate all changes |

Total: **8 files changed, 3 new files created.**

---

## Step 1: Remove `option_env!` — Unblock Local Mode

### 1a. `crates/local-deployment/src/lib.rs` lines 176-181

**Before:**
```rust
let api_base = std::env::var("VK_SHARED_API_BASE")
    .ok()
    .or_else(|| option_env!("VK_SHARED_API_BASE").map(|s| s.to_string()));
let relay_api_base = std::env::var("VK_SHARED_RELAY_API_BASE")
    .ok()
    .or_else(|| option_env!("VK_SHARED_RELAY_API_BASE").map(|s| s.to_string()));
```

**After:**
```rust
let api_base = std::env::var("VK_SHARED_API_BASE").ok();
let relay_api_base = std::env::var("VK_SHARED_RELAY_API_BASE").ok();
```

**Effect:** When env var not set, `remote_client()` returns `Err(RemoteClientNotConfigured)`. Auth bypass activates. `shared_api_base` in `/api/info` returns `null`.

### 1b. `crates/services/src/services/analytics.rs` lines 24-29

**Before:**
```rust
let api_key = option_env!("POSTHOG_API_KEY")
    .map(|s| s.to_string())
    .or_else(|| std::env::var("POSTHOG_API_KEY").ok())?;
let api_endpoint = option_env!("POSTHOG_API_ENDPOINT")
    .map(|s| s.to_string())
    .or_else(|| std::env::var("POSTHOG_API_ENDPOINT").ok())?;
```

**After:**
```rust
let api_key = std::env::var("POSTHOG_API_KEY").ok()?;
let api_endpoint = std::env::var("POSTHOG_API_ENDPOINT").ok()?;
```

### 1c. `crates/utils/src/sentry.rs` lines 27-34

**Before:**
```rust
fn dsn(self) -> Option<String> {
    let value = match self {
        SentrySource::Remote => option_env!("SENTRY_DSN_REMOTE")
            .map(|s| s.to_string())
            .or_else(|| std::env::var("SENTRY_DSN_REMOTE").ok()),
        _ => option_env!("SENTRY_DSN")
            .map(|s| s.to_string())
            .or_else(|| std::env::var("SENTRY_DSN").ok()),
    };
    value.filter(|s| !s.is_empty())
}
```

**After:**
```rust
fn dsn(self) -> Option<String> {
    let value = match self {
        SentrySource::Remote => std::env::var("SENTRY_DSN_REMOTE").ok(),
        _ => std::env::var("SENTRY_DSN").ok(),
    };
    value.filter(|s| !s.is_empty())
}
```

### 1d. Keep `crates/remote/src/analytics.rs:15-16` (unchanged)

These `option_env!` calls in `crates/remote/` are the only source of `POSTHOG_API_KEY` and `POSTHOG_API_ENDPOINT` for the remote server package (a separate Docker-built workspace). Converting them would change the remote server's behavior. Leave them as-is. The `crates/remote/` workspace is NOT part of local kanban.

---

## Step 2: Clean Up build.rs Files

Remove `rustc-env` injection — prevents compile-time values from leaking into `option_env!`.

### 2a. `crates/local-deployment/build.rs`

**Before (full file):**
```rust
use std::path::Path;

fn main() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let env_file = workspace_root.join(".env");
    dotenv::from_path(&env_file).ok();

    println!("cargo:rerun-if-env-changed=VK_SHARED_API_BASE");
    if env_file.exists() {
        println!("cargo:rerun-if-changed={}", env_file.display());
    }

    if let Ok(val) = std::env::var("VK_SHARED_API_BASE") {
        println!("cargo:rustc-env=VK_SHARED_API_BASE={}", val);
    }
}
```

**After:**
```rust
// No build.rs needed — env vars are read at runtime via std::env::var()
fn main() {}
```

### 2b. `crates/server/build.rs`

**Before:** 47 lines with `rustc-env` injections for POSTHOG_API_KEY, POSTHOG_API_ENDPOINT, VK_SHARED_API_BASE, VK_SHARED_RELAY_API_BASE (SENTRY_DSN is `rerun-if-env-changed` only, not injected).

**After:** Keep only the dummy `packages/local-web/dist` directory creation (lines 34-46). Remove lines 1-33 (env var loading, rerun-if-env-changed, rustc-env injections).

```rust
use std::{fs, path::Path};

fn main() {
    // Create packages/local-web/dist directory if it doesn't exist
    let dist_path = Path::new("../../packages/local-web/dist");
    if !dist_path.exists() {
        fs::create_dir_all(dist_path).unwrap();

        let dummy_html = r#"<!DOCTYPE html>
<html><head><title>Build web app first</title></head>
<body><h1>Please build @vibe/local-web first</h1></body></html>"#;

        fs::write(dist_path.join("index.html"), dummy_html).unwrap();
    }
}
```

---

## Step 3: Add `/v1/organizations` Route (CRITICAL)

Without this, the frontend can't discover any organization and the kanban UI is blocked.

### 3a. New file: `crates/server/src/routes/kanban_v1/organizations.rs`

**Verified type structure** (`crates/api-types/src/organizations.rs:33-42`):
`OrganizationWithRole` is a **flat** struct with all org fields inline + `user_role`:
```rust
pub struct OrganizationWithRole {
    pub id: Uuid, pub name: String, pub slug: String,
    pub is_personal: bool, pub issue_prefix: String,
    pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
    pub user_role: MemberRole,
}
```

```rust
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
```

### 3b. Edit: `crates/server/src/routes/kanban_v1/mod.rs`

Add at the top with other module declarations:
```rust
pub mod organizations;
```

Add in `router()` after existing fallback routes:
```rust
.nest("/organizations", organizations::router())
```

---

## Step 4: Add `/v1/issue_comments` Routes

Comment CRUD is needed for full kanban functionality. Frontend uses Electric shape sync, which falls back to HTTP when Electric is unavailable (always in local mode).

### 4a. New file: `crates/server/src/routes/kanban_v1/issue_comments.rs`

**Verified DB model signatures** (`crates/db/src/models/issue_comment.rs`):
- `create(pool, issue_id: Uuid, author_id: Option<Uuid>, parent_id: Option<Uuid>, message: &str) -> Result<Self>`
- `update(pool, id: Uuid, message: &str) -> Result<Self>` — message only, no parent_id
- `delete(pool, id: Uuid) -> Result<()>`
- `find_by_issue(pool, issue_id: Uuid) -> Result<Vec<Self>>`

**Verified API types** (`crates/api-types/src/issue_comment.rs`):
- `IssueComment` — id, issue_id, author_id, parent_id, message, created_at, updated_at (all timestamps as `DateTime<Utc>`)
- `CreateIssueCommentRequest` — id (Option), issue_id, message, parent_id
- `UpdateIssueCommentRequest` — message (Option<String>), parent_id (Option<Option<Uuid>>)
- `ListIssueCommentsQuery` — issue_id: Uuid
- `ListIssueCommentsResponse` — issue_comments: Vec<IssueComment>

```rust
use api_types::{
    CreateIssueCommentRequest, DeleteResponse, IssueComment,
    ListIssueCommentsQuery, ListIssueCommentsResponse, MutationResponse,
    UpdateIssueCommentRequest,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use db::models::issue_comment::IssueComment as DbComment;
use deployment::Deployment;
use uuid::Uuid;

use crate::DeploymentImpl;
use super::{local_txid, db_error};
use super::shape_fallbacks::ErrorResponse;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/issue_comments", get(list_issue_comments).post(create_issue_comment))
        .route(
            "/issue_comments/{id}",
            get(get_issue_comment).patch(update_issue_comment).delete(delete_issue_comment),
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
    // find_by_issue returns Vec; use raw query for single lookup
    let comment = sqlx::query_as!(
        DbComment,
        r#"SELECT id as "id!: Uuid", issue_id as "issue_id!: Uuid",
                  author_id as "author_id: Uuid", parent_id as "parent_id: Uuid",
                  message,
                  created_at as "created_at!: DateTime<Utc>",
                  updated_at as "updated_at!: DateTime<Utc>"
           FROM issue_comments WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
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
        pool, body.issue_id,
        body.author_id.or(Some(local_user_id)), // use local user if no author
        body.parent_id, &body.message,
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

    // Get existing comment first
    let comment = sqlx::query_as!(
        DbComment,
        r#"SELECT id as "id!: Uuid", issue_id as "issue_id!: Uuid",
                  author_id as "author_id: Uuid", parent_id as "parent_id: Uuid",
                  message,
                  created_at as "created_at!: DateTime<Utc>",
                  updated_at as "updated_at!: DateTime<Utc>"
           FROM issue_comments WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| db_error(e, "failed to find issue comment"))?
    .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "comment not found"))?;

    // Update message if provided
    let updated = if let Some(msg) = &body.message {
        DbComment::update(pool, id, msg)
            .await
            .map_err(|e| db_error(e, "failed to update issue comment"))?
    } else {
        comment.clone()
    };

    // Update parent_id if provided (raw SQL since DB model doesn't expose this)
    if let Some(parent_id) = &body.parent_id {
        sqlx::query!(
            "UPDATE issue_comments SET parent_id = ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
            parent_id,
            id
        )
        .execute(pool)
        .await
        .map_err(|e| db_error(e, "failed to update comment parent_id"))?;
    }

    Ok(Json(MutationResponse {
        data: IssueComment {
            id: updated.id,
            issue_id: updated.issue_id,
            author_id: updated.author_id,
            parent_id: body.parent_id.unwrap_or(updated.parent_id),
            message: body.message.unwrap_or(updated.message),
            created_at: updated.created_at,
            updated_at: updated.updated_at,
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
```

### 4b. New file: `crates/server/src/routes/kanban_v1/shape_fallbacks.rs` — add handler

Add at the bottom of the existing file:
```rust
#[derive(serde::Deserialize)]
pub struct CommentFallbackQuery {
    pub issue_id: Uuid,
}

#[derive(Serialize)]
pub struct CommentsResponse {
    pub issue_comments: Vec<api_types::IssueComment>,
}

pub async fn fallback_list_issue_comments(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<CommentFallbackQuery>,
) -> Result<Json<CommentsResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let comments = IssueComment::find_by_issue(pool, query.issue_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, issue_id = %query.issue_id, "failed to list issue comments (fallback)");
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list issue comments")
        })?;

    let api_comments: Vec<api_types::IssueComment> = comments.into_iter().map(|c| api_types::IssueComment {
        id: c.id,
        issue_id: c.issue_id,
        author_id: c.author_id,
        parent_id: c.parent_id,
        message: c.message,
        created_at: c.created_at,
        updated_at: c.updated_at,
    }).collect();

    Ok(Json(CommentsResponse { issue_comments: api_comments }))
}
```

Add the required imports at the top of shape_fallbacks.rs:
```rust
use db::models::issue_comment::IssueComment;
use serde::Deserialize; // if not already imported
```

### 4c. Edit: `crates/server/src/routes/kanban_v1/mod.rs`

Add module declaration:
```rust
pub mod issue_comments;
```

Add fallback route:
```rust
.route("/fallback/issue_comments", get(shape_fallbacks::fallback_list_issue_comments))
```

Add mutation routes:
```rust
.nest("/issue_comments", issue_comments::router())
```

---

## Step 5: Verify Local SQLite Seed Data

The first-boot seed creates "My Workspace" org with deterministic UUID. The new `/v1/organizations` route will return this org. No additional seeding changes needed.

---

## Step 5.5: Known Missing Routes (Non-Blocking)

These routes are referenced by the frontend but NOT implemented in kanban_v1. They will 404 in local mode. Basic kanban functionality does NOT depend on them:

| Route | Frontend Use | Impact |
|-------|-------------|--------|
| `/v1/workspaces` (CRUD) | KANBAN_PATH_PREFIXES | Workspaces have separate routes at `/api/workspaces` level — NOT blocked |
| `/v1/fallback/user_workspaces` | USER_WORKSPACES_SHAPE | Shape sync for workspaces fails, but workspace UI uses `/api/workspaces` directly |
| `/v1/fallback/project_workspaces` | PROJECT_WORKSPACES_SHAPE | Same as above |
| `/v1/fallback/notifications` | NOTIFICATIONS_SHAPE | No notification feature in local mode |
| `/v1/fallback/organization_members` | ORGANIZATION_MEMBERS_SHAPE | Single-user local, no members to sync |
| `/v1/fallback/users` | USERS_SHAPE | Single local user, no sync needed |
| `/v1/fallback/issue_followers` | Not in KANBAN_PATH_PREFIXES | Following not critical for kanban |
| `/v1/fallback/issue_comment_reactions` | Not in KANBAN_PATH_PREFIXES | Reactions not critical for kanban |
| `/v1/fallback/pull_requests` | Not in KANBAN_PATH_PREFIXES | PR linking not applicable locally |
| `/v1/fallback/pull_request_issues` | Not in KANBAN_PATH_PREFIXES | Same as above |

**Recommendation:** Add workspace fallback routes if workspace data needs to appear in the kanban sidebar. Other routes can be deferred.

---

## Step 6: Wipe Local DB + Build

Delete local DB to get fresh seed with the new code:
```bash
rm -f ~/Library/Application\ Support/ai.bloop.vibe-kanban/db.v2.sqlite*
```

Clean Rust build caches (required because old `option_env!` values are baked into `.rlib` artifacts):
```bash
cargo clean
```

Build:
```bash
pnpm run build:npx
```

Alternative — dev mode (run from source):
```bash
pnpm run dev
```

---

## Step 7: Verification Checklist

After build + run:

- [ ] `/api/info` returns `shared_api_base: null`
- [ ] `/api/info` returns `login_status.status: "loggedin"` (auto-local-user)
- [ ] `curl localhost:BACKEND_PORT/api/remote/v1/organizations` returns `{ "organizations": [{ ... "name": "My Workspace" ... }] }`
- [ ] `curl localhost:BACKEND_PORT/api/remote/v1/organizations` — `is_personal` field is boolean (not integer)
- [ ] Kanban board renders (no sign-in prompt, no sunset page)
- [ ] Sidebar shows "My Workspace" org and "Main Project"
- [ ] Can create issue → gets VK-1 simple_id
- [ ] Can drag issue between columns
- [ ] Can add comments
- [ ] Data persists across app restart
- [ ] No connection to api.vibekanban.com (verify with `lsof -i TCP -P | grep vibe-kanb`)
- [ ] Browser console: no 404s for `/v1/organizations` or `/v1/issue_comments`
- [ ] Browser console: expected 404s for `/v1/fallback/user_workspaces` etc. (non-blocking, see Step 5.5)

---

## Files Changed Summary

| File | Action | Description |
|------|--------|-------------|
| `crates/local-deployment/src/lib.rs` | Edit L176-181 | Remove option_env! fallbacks |
| `crates/local-deployment/build.rs` | Simplify | Remove env injection, keep empty main |
| `crates/server/build.rs` | Edit L1-33 | Remove env injection, keep dist dir creation (with cargo:warning) |
| `crates/services/src/services/analytics.rs` | Edit L24-29 | Remove option_env! |
| `crates/utils/src/sentry.rs` | Edit L27-34 | Remove option_env! |
| `crates/server/src/routes/kanban_v1/organizations.rs` | **NEW** | GET /organizations handler |
| `crates/server/src/routes/kanban_v1/issue_comments.rs` | **NEW** | Comment CRUD handlers |
| `crates/server/src/routes/kanban_v1/shape_fallbacks.rs` | Edit | Add fallback_list_issue_comments |
| `crates/server/src/routes/kanban_v1/mod.rs` | Edit | Add 2 modules + 3 routes |

### Corrections Applied (2026-05-10 verification)

| Issue | Fix |
|-------|-----|
| Step 3: `is_personal: org.is_personal == 1` won't compile — DB model has `bool`, not `i32` | Changed to `is_personal: org.is_personal` |
| Step 4b: `#[derive(Deserialize)]` — `Deserialize` not imported in shape_fallbacks.rs | Changed to `#[derive(serde::Deserialize)]` |
| Step 2b: Description claimed SENTRY_DSN is `rustc-env` injected | Corrected: only `rerun-if-env-changed`, not injected |
| Missing: workspace shape fallback routes | Documented in Step 5.5 as non-blocking |

**8 files edited, 2 new files created, 2 build.rs simplified.**
