# Local Kanban Fix — Task Breakdown

**Source plan:** `LOCAL_KANBAN_COMPLETE_FIX_PLAN.md`
**Date:** 2026-05-10
**Total tasks:** 8 across 4 batches

---

## Parallelization Strategy

### Batch Diagram

```
Batch 1 (Foundation)      Batch 2 (Route Files)     Batch 3 (Integration)    Batch 4 (Verify)
┌─────────────────┐       ┌──────────────────┐       ┌─────────────────┐      ┌──────────────┐
│ Task 1: option  │       │ Task 4: orgs.rs  │──┐     │                 │      │ Task 8:      │
│   _env removal  │       │ Task 5: comments │  ├────▶│ Task 7: mod.rs │─────▶│ wipe+build+  │
│ Task 2: local-  │       │ Task 6: fallback │──┘     │   wiring       │      │ verify       │
│   deploy build  │       └──────────────────┘       └─────────────────┘      └──────────────┘
│ Task 3: server  │                    │                       ▲
│   build.rs      │                    └─────── depends ───────┘
└─────────────────┘                                     (batch 3 needs
        │                                                batch 2 files)
        │
        └────────────── batch 4 depends on all prior ──────────▶
```

### Merge Order Rules

1. **Batch 1** → merge in any order (no file overlap between tasks)
2. **Batch 2** → merge in any order (no file overlap — each task touches different file)
3. **Batch 3** → merge AFTER all Batch 2 tasks merged (mod.rs needs new files to exist)
4. **Batch 4** → run AFTER all tasks merged (full build verification)

### Summary Table

| # | Batch | Target File(s) | Dependencies | Can Parallel With |
|---|-------|----------------|--------------|-------------------|
| 1 | 1 | lib.rs, analytics.rs, sentry.rs | None | Tasks 2, 3 |
| 2 | 1 | local-deployment/build.rs | None | Tasks 1, 3 |
| 3 | 1 | server/build.rs | None | Tasks 1, 2 |
| 4 | 2 | kanban_v1/organizations.rs (NEW) | None | Tasks 5, 6 |
| 5 | 2 | kanban_v1/issue_comments.rs (NEW) | None | Tasks 4, 6 |
| 6 | 2 | kanban_v1/shape_fallbacks.rs | None | Tasks 4, 5 |
| 7 | 3 | kanban_v1/mod.rs | Tasks 4, 5, 6 | None |
| 8 | 4 | terminal / build | Tasks 1, 2, 3, 7 | None |

### Developer Assignment Tracks

**Track A (Backend Routes):** Tasks 4 → 5 → 6 → 7 → 8
- Developer who knows axum patterns and the kanban_v1 codebase. Owns all route creation and wiring.

**Track B (Build System):** Tasks 2 → 3 → (help with Task 8)
- Developer focused on build infra and env var cleanup. Lighter workload, can assist with verification.

**Track C (Quick Fixes):** Task 1 → (help with Task 8)
- Developer doing the mechanical option_env! removal. Fastest task, can assist with testing.

Optimal team: 2 developers. Track A handles routes (4-7) sequentially since Task 7 gates on 4-6. Track B handles build cleanup (1-3) in parallel with Track A's route creation. Both converge on Task 8.

---

## Batch 1 — Foundation

### Task 1: Remove option_env! compile-time fallbacks from source files

**Description:**
Remove `option_env!()` calls from 3 files. These bake values at compile time, preventing local mode from working without env vars set.

Files to edit:

1. `crates/local-deployment/src/lib.rs` lines 176-181 — Replace:
```rust
let api_base = std::env::var("VK_SHARED_API_BASE")
    .ok()
    .or_else(|| option_env!("VK_SHARED_API_BASE").map(|s| s.to_string()));
let relay_api_base = std::env::var("VK_SHARED_RELAY_API_BASE")
    .ok()
    .or_else(|| option_env!("VK_SHARED_RELAY_API_BASE").map(|s| s.to_string()));
```
With:
```rust
let api_base = std::env::var("VK_SHARED_API_BASE").ok();
let relay_api_base = std::env::var("VK_SHARED_RELAY_API_BASE").ok();
```

2. `crates/services/src/services/analytics.rs` lines 24-29 — Replace:
```rust
let api_key = option_env!("POSTHOG_API_KEY")
    .map(|s| s.to_string())
    .or_else(|| std::env::var("POSTHOG_API_KEY").ok())?;
let api_endpoint = option_env!("POSTHOG_API_ENDPOINT")
    .map(|s| s.to_string())
    .or_else(|| std::env::var("POSTHOG_API_ENDPOINT").ok())?;
```
With:
```rust
let api_key = std::env::var("POSTHOG_API_KEY").ok()?;
let api_endpoint = std::env::var("POSTHOG_API_ENDPOINT").ok()?;
```

3. `crates/utils/src/sentry.rs` lines 27-34 — Replace:
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
With:
```rust
fn dsn(self) -> Option<String> {
    let value = match self {
        SentrySource::Remote => std::env::var("SENTRY_DSN_REMOTE").ok(),
        _ => std::env::var("SENTRY_DSN").ok(),
    };
    value.filter(|s| !s.is_empty())
}
```

**IMPORTANT:** Do NOT touch `crates/remote/src/analytics.rs` — that's a separate Docker-built workspace.

**Acceptance Criteria:**
- [ ] `grep -rn "option_env!" crates/local-deployment/src/ crates/services/src/services/analytics.rs crates/utils/src/sentry.rs` returns no matches
- [ ] `grep -n "option_env!" crates/remote/src/analytics.rs` still returns matches (unchanged)
- [ ] `cargo check -p local-deployment -p services -p utils` passes
- [ ] No env vars set → analytics and sentry gracefully disabled, remote_client returns Err

**Dependencies:** None

---

### Task 2: Simplify local-deployment build.rs

**Description:**
Remove `rustc-env` injection from `crates/local-deployment/build.rs`. The current file loads .env, sets rerun-if-env-changed, and injects VK_SHARED_API_BASE via rustc-env. After this task, env vars are read at runtime only.

Replace entire file with:
```rust
fn main() {}
```

**Acceptance Criteria:**
- [ ] File is 1 line: `fn main() {}`
- [ ] No `println!("cargo:rustc-env=` statements
- [ ] `cargo check -p local-deployment` passes
- [ ] Build.rs no longer depends on .env file existing

**Dependencies:** None

---

### Task 3: Simplify server build.rs

**Description:**
Remove env var injection from `crates/server/build.rs` (48 lines). Keep only the dist directory creation logic. Remove all `rustc-env` injections, dotenv loading, and `rerun-if-env-changed` directives.

Replace entire file with:
```rust
use std::{fs, path::Path};

fn main() {
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

**Acceptance Criteria:**
- [ ] No `println!("cargo:rustc-env=` statements remain
- [ ] No `dotenv::from_path` call
- [ ] No `rerun-if-env-changed` directives
- [ ] Dist directory creation logic preserved
- [ ] `cargo check -p server` passes

**Dependencies:** None

---

## Batch 2 — Route Files

### Task 4: Create organizations route handler

**Description:**
Create new file `crates/server/src/routes/kanban_v1/organizations.rs` implementing GET /v1/organizations.

This route queries all organizations from SQLite, maps them to `OrganizationWithRole` (flat struct with `user_role: MemberRole::Admin`), and returns `ListOrganizationsResponse`.

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

Note: `is_personal` is `bool` in DB model (not `i32`). `MemberRole::Admin` is correct for single-user local mode. `ErrorResponse` is defined in `super::shape_fallbacks`.

**Acceptance Criteria:**
- [ ] File created at `crates/server/src/routes/kanban_v1/organizations.rs`
- [ ] `pub fn router()` returns `Router<DeploymentImpl>`
- [ ] `OrganizationWithRole` mapping uses all 8 fields including `user_role: MemberRole::Admin`
- [ ] `cargo check -p server` passes (after Task 7 wires mod.rs)

**Dependencies:** None (compiles after Task 7 wires mod.rs)

---

### Task 5: Create issue_comments route handlers

**Description:**
Create new file `crates/server/src/routes/kanban_v1/issue_comments.rs` implementing full CRUD for issue comments.

Handlers:
- `GET /issue_comments` — list by issue_id query param
- `GET /issue_comments/{id}` — single comment lookup via raw SQL (no find_by_id method exists)
- `POST /issue_comments` — create with hardcoded local user UUID as author_id
- `PATCH /issue_comments/{id}` — update message and/or parent_id
- `DELETE /issue_comments/{id}` — delete

Key implementation details:
- `CreateIssueCommentRequest` has NO `author_id` field. Hardcode `Some(local_user_id)` where local_user_id = "00000000-0000-0000-0000-000000000001"
- `UpdateIssueCommentRequest.parent_id` is `Option<Option<Uuid>>` — outer Option = not in request, inner Option = set to null
- DB `update()` only handles message. Parent_id update requires raw SQL
- Update order: parent_id raw SQL first, then message via `DbComment::update()` (which returns freshest row via RETURNING)
- Use `$1`/`$2` placeholder style (not `?`)
- Import shared types from `super`: `DeleteResponse`, `ErrorResponse`, `MutationResponse`, `db_error`, `local_txid`

```rust
use api_types::{
    CreateIssueCommentRequest, IssueComment,
    ListIssueCommentsQuery, ListIssueCommentsResponse,
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
use super::{DeleteResponse, ErrorResponse, MutationResponse, db_error, local_txid};

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
    let comment = sqlx::query_as!(
        DbComment,
        r#"SELECT id as "id!: Uuid", issue_id as "issue_id!: Uuid",
                  author_id as "author_id: Uuid", parent_id as "parent_id: Uuid",
                  message,
                  created_at as "created_at!: DateTime<Utc>",
                  updated_at as "updated_at!: DateTime<Utc>"
           FROM issue_comments WHERE id = $1"#,
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
        Some(local_user_id),
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

    let comment = sqlx::query_as!(
        DbComment,
        r#"SELECT id as "id!: Uuid", issue_id as "issue_id!: Uuid",
                  author_id as "author_id: Uuid", parent_id as "parent_id: Uuid",
                  message,
                  created_at as "created_at!: DateTime<Utc>",
                  updated_at as "updated_at!: DateTime<Utc>"
           FROM issue_comments WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| db_error(e, "failed to find issue comment"))?
    .ok_or_else(|| ErrorResponse::new(StatusCode::NOT_FOUND, "comment not found"))?;

    if let Some(parent_id) = &body.parent_id {
        sqlx::query!(
            "UPDATE issue_comments SET parent_id = $1, updated_at = datetime('now', 'subsec') WHERE id = $2",
            parent_id,
            id
        )
        .execute(pool)
        .await
        .map_err(|e| db_error(e, "failed to update comment parent_id"))?;
    }

    let updated = if let Some(msg) = &body.message {
        DbComment::update(pool, id, msg)
            .await
            .map_err(|e| db_error(e, "failed to update issue comment"))?
    } else {
        comment.clone()
    };

    let final_parent_id = body.parent_id.unwrap_or(updated.parent_id);
    let final_message = body.message.unwrap_or(updated.message);

    let final_updated_at = if body.parent_id.is_some() && body.message.is_none() {
        sqlx::query_scalar!(
            r#"SELECT updated_at as "updated_at!: DateTime<Utc>" FROM issue_comments WHERE id = $1"#,
            id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| db_error(e, "failed to re-fetch updated_at"))?
    } else {
        updated.updated_at
    };

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
```

**Acceptance Criteria:**
- [ ] File created at `crates/server/src/routes/kanban_v1/issue_comments.rs`
- [ ] 5 handler functions + router + to_api helper
- [ ] create_issue_comment hardcodes local user UUID, does NOT reference body.author_id
- [ ] update_issue_comment handles parent_id update via raw SQL before message update
- [ ] All SQL uses `$1`/`$2` placeholder style
- [ ] `cargo check -p server` passes (after Task 7 wires mod.rs)

**Dependencies:** None (compiles after Task 7 wires mod.rs)

---

### Task 6: Add comment fallback handler to shape_fallbacks.rs

**Description:**
Add `fallback_list_issue_comments` handler to existing `crates/server/src/routes/kanban_v1/shape_fallbacks.rs`. This handles the Electric shape sync fallback path for comments in local mode.

Add these imports at top (merge with existing imports):
```rust
use db::models::issue_comment::IssueComment;  // add to existing db model imports
use serde::{Deserialize, Serialize};  // change from just Serialize to both
```

Add at bottom of file:
```rust
#[derive(Deserialize)]
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

Note: `Uuid` and `StatusCode` are already imported in the file. `State`, `Query`, `Json` are already imported. Only need to add `IssueComment` to db imports and `Deserialize` to serde import.

**Acceptance Criteria:**
- [ ] `use serde::{Deserialize, Serialize};` replaces `use serde::Serialize;`
- [ ] `use db::models::issue_comment::IssueComment;` added
- [ ] `CommentFallbackQuery`, `CommentsResponse`, `fallback_list_issue_comments` added
- [ ] Follows same pattern as existing fallback handlers (fallback_list_issues, etc.)
- [ ] `cargo check -p server` passes

**Dependencies:** None

---

## Batch 3 — Integration

### Task 7: Wire organizations + issue_comments into mod.rs

**Description:**
Edit `crates/server/src/routes/kanban_v1/mod.rs` to register the new modules and routes from Tasks 4-6.

This is the integration point — all 3 new route files come together here. Edit ONLY mod.rs.

Changes:

1. Add module declarations (near existing `pub mod` declarations):
```rust
pub mod issue_comments;
pub mod organizations;
```

2. Add routes in `router()` function. Add the nest call after existing nested routes:
```rust
.nest("/organizations", organizations::router())
```

3. Add issue_comments nest route:
```rust
.nest("/issue_comments", issue_comments::router())
```

4. Add fallback route for comments (with other fallback routes):
```rust
.route("/fallback/issue_comments", get(shape_fallbacks::fallback_list_issue_comments))
```

Note: mod.rs already has these imports available: `use axum::routing::get;`, `Router`, `State`, etc. The `shape_fallbacks` module is already declared.

**Acceptance Criteria:**
- [ ] `pub mod organizations;` and `pub mod issue_comments;` added
- [ ] `/organizations` nest route added in router()
- [ ] `/issue_comments` nest route added in router()
- [ ] `/fallback/issue_comments` fallback route added
- [ ] `cargo check -p server` passes — this is the compilation gate
- [ ] No duplicate module declarations or routes

**Dependencies:** Tasks 4, 5, 6

---

## Batch 4 — Verification

### Task 8: Wipe DB, build, and verify

**Description:**
Clean build artifacts (required because old `option_env!` values are baked into .rlib files), wipe local SQLite DB for fresh seed, then build and run verification.

Steps in order:

1. Clean Rust build caches:
```bash
cargo clean
```

2. Wipe local DB:
```bash
rm -f ~/Library/Application\ Support/ai.bloop.vibe-kanban/db.v2.sqlite*
```

3. Build:
```bash
pnpm run build:npx
```

4. Run dev mode:
```bash
pnpm run dev
```

5. Verification checklist:
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
- [ ] Browser console: expected 404s for `/v1/fallback/user_workspaces` etc. (non-blocking)

**Acceptance Criteria:**
- [ ] `cargo clean` completed
- [ ] Local DB wiped
- [ ] `pnpm run build:npx` succeeds with 0 errors
- [ ] All verification checklist items pass

**Dependencies:** Tasks 1, 2, 3, 7
