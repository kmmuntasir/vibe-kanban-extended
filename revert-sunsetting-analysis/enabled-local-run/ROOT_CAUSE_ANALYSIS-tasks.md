# Task Breakdown — Root Cause Analysis Fixes

**Source:** `ROOT_CAUSE_ANALYSIS.md` (2026-05-10)
**Branch:** `analysis/enable-local-backend-and-db`

---

## Parallelization Strategy

All 4 tasks are **independent** — zero shared files, zero merge conflicts. Single batch, all 4 can run in parallel.

```
Batch 1 (all parallel)
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│  Task 1  │  │  Task 2  │  │  Task 3  │  │  Task 4  │
│ org path │  │ comments │  │ 500→404  │  │ fallback │
│  fix     │  │ path fix │  │  fix     │  │ prefix   │
└──────────┘  └──────────┘  └──────────┘  └──────────┘
```

**Merge order:** All tasks merge into `analysis/enable-local-backend-and-db` with no ordering constraints.

| # | Batch | Target File | Dependencies | Can Parallel With |
|---|-------|-------------|--------------|-------------------|
| 1 | 1 | `organizations.rs` | None | 2, 3, 4 |
| 2 | 1 | `issue_comments.rs` | None | 1, 3, 4 |
| 3 | 1 | `containers.rs` | None | 1, 2, 4 |
| 4 | 1 | `remoteApi.ts` | None | 1, 2, 3 |

**Suggested tracks (2 developers):**
- Dev A: Tasks 1 + 2 (Rust route paths)
- Dev B: Tasks 3 + 4 (error handling + frontend prefix)

---

## Task 1: Fix organizations.rs double-nesting route path

**Title:** Fix organizations route path from `/organizations` to `/`

**Description:**

The `organizations::router()` defines its inner route as `"/organizations"`, but `mod.rs` nests this router under `"/organizations"` via `.nest("/organizations", organizations::router())`. Axum's `.nest()` strips the prefix before passing to the inner router, so the effective route becomes `/organizations/organizations` instead of `/organizations`.

This is the **root cause** of the empty projects menu and missing "+" button — the frontend's `useUserOrganizations()` hook calls `GET /v1/organizations`, which the backend rewrites to `/api/remote/v1/organizations`. The route doesn't match, returning HTML (SPA catch-all) instead of JSON, causing a parse error, leaving `organizations = []`, and short-circuiting the entire projects flow.

**File:** [crates/server/src/routes/kanban_v1/organizations.rs:10](crates/server/src/routes/kanban_v1/organizations.rs#L10)

**Change:**

```rust
// BEFORE (broken — route only matches at /organizations/organizations):
Router::new().route("/organizations", get(list_organizations))

// AFTER (correct — route matches at /organizations):
Router::new().route("/", get(list_organizations))
```

**Why single `/` is correct:** All other nested route modules in kanban_v1 use `"/"` as their base path because `mod.rs` applies the prefix via `.nest()`. See [projects.rs](crates/server/src/routes/kanban_v1/projects.rs), [tags.rs](crates/server/src/routes/kanban_v1/tags.rs), [project_statuses.rs](crates/server/src/routes/kanban_v1/project_statuses.rs), etc. for the established pattern.

**Acceptance Criteria:**
- [ ] `curl http://127.0.0.1:PORT/api/remote/v1/organizations` returns JSON (not HTML)
- [ ] Response contains `{"organizations": [...]}` with the "My Workspace" org
- [ ] `pnpm run check` passes (backend type check)
- [ ] `cargo test --workspace` passes

**Dependencies:** None

---

## Task 2: Fix issue_comments.rs double-nesting route paths

**Title:** Fix issue_comments route paths from `/issue_comments` to `/` and `/{id}`

**Description:**

Same double-nesting bug as Task 1. `issue_comments::router()` defines routes at `"/issue_comments"` and `"/issue_comments/{id}"`, but `mod.rs` nests this router under `"/issue_comments"` via `.nest("/issue_comments", issue_comments::router())`. The effective routes become `/issue_comments/issue_comments` and `/issue_comments/issue_comments/{id}`.

No user-facing symptom is directly linked to this in the RCA, but it blocks all comment CRUD operations through the local backend proxy.

**File:** [crates/server/src/routes/kanban_v1/issue_comments.rs:18-29](crates/server/src/routes/kanban_v1/issue_comments.rs#L18-L29)

**Change:**

```rust
// BEFORE (broken):
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/issue_comments", get(list_issue_comments).post(create_issue_comment))
        .route("/issue_comments/{id}", get(get_issue_comment).patch(update_issue_comment).delete(delete_issue_comment))
}

// AFTER (correct):
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_issue_comments).post(create_issue_comment))
        .route("/{id}", get(get_issue_comment).patch(update_issue_comment).delete(delete_issue_comment))
}
```

**Why single `/` is correct:** Same Axum `.nest()` semantics as Task 1. The prefix `/issue_comments` is already applied by [mod.rs:46](crates/server/src/routes/kanban_v1/mod.rs#L46).

**Acceptance Criteria:**
- [ ] `curl "http://127.0.0.1:PORT/api/remote/v1/issue_comments?issue_id=..."` returns JSON (not HTML)
- [ ] `curl http://127.0.0.1:PORT/api/remote/v1/issue_comments/{id}` returns JSON for a valid ID
- [ ] `pnpm run check` passes
- [ ] `cargo test --workspace` passes

**Dependencies:** None

---

## Task 3: Fix containers/attempt-context 500 crash — map RowNotFound to 404

**Title:** Map RowNotFound to 404 in containers/attempt-context handler

**Description:**

`Workspace::resolve_container_ref_by_prefix()` is the **only** function in the entire `crates/db/src/models/` layer that returns `Err(sqlx::Error::RowNotFound)` when no match is found — all 18 other `find_by_*` functions return `Result<Option<T>, sqlx::Error>`. The `containers.rs` handler maps ALL database errors to 500 via `ApiError::Database`, which at [error.rs:489](crates/server/src/error.rs#L489) becomes `ErrorInfo::internal("DatabaseError")` — HTTP 500.

The MCP client calls `GET /api/containers/attempt-context?ref=...` at startup. When no workspace matches (normal state: empty DB, or no workspace tied to that container), the 500 triggers a fatal error in the orchestrator, killing the backend process. The npx-cli then restarts the server.

**Files:**
- [crates/server/src/routes/containers.rs:37-48](crates/server/src/routes/containers.rs#L37-L48) — handler
- [crates/db/src/models/workspace.rs:350-372](crates/db/src/models/workspace.rs#L350-L372) — `resolve_container_ref_by_prefix`
- [crates/server/src/error.rs:489](crates/server/src/error.rs#L489) — `ApiError::Database` → 500

**Change (minimal fix in handler):**

In [containers.rs:41-44](crates/server/src/routes/containers.rs#L41-L44), replace the error mapping:

```rust
// BEFORE:
let info = Workspace::resolve_container_ref_by_prefix(&deployment.db().pool, &payload.container_ref)
    .await
    .map_err(ApiError::Database)?;

// AFTER:
let info = Workspace::resolve_container_ref_by_prefix(&deployment.db().pool, &payload.container_ref)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound("Container not found".into()),
        other => ApiError::Database(other),
    })?;
```

**Acceptance Criteria:**
- [ ] `curl "http://127.0.0.1:PORT/api/containers/attempt-context?ref=nonexistent"` returns **404** (not 500)
- [ ] Backend does **not** crash after receiving this request
- [ ] Valid container ref still returns 200 with `WorkspaceContext` JSON
- [ ] `pnpm run check` passes
- [ ] `cargo test --workspace` passes

**Dependencies:** None

---

## Task 4: Add `/v1/fallback` to KANBAN_PATH_PREFIXES in frontend

**Title:** Add fallback path prefix to frontend routing table

**Description:**

The `KANBAN_PATH_PREFIXES` array in `remoteApi.ts` controls which API paths are routed to the local backend (`/api/remote${path}`) vs. the remote API. Fallback paths like `/v1/fallback/projects` start with `/v1/fallback/`, not `/v1/projects`, so `isKanbanPath()` returns `false` for them. In local mode, non-matching paths fall through to `makeAuthenticatedRequest()` which requires a remote API base URL and auth token — neither available locally.

The backend already has fallback routes defined at [mod.rs:19-38](crates/server/src/routes/kanban_v1/mod.rs#L19-L38) for all 8 shape types. Adding one prefix entry enables routing for all fallback endpoints.

**File:** [packages/web-core/src/shared/lib/remoteApi.ts:47-58](packages/web-core/src/shared/lib/remoteApi.ts#L47-L58)

**Change:**

```typescript
// BEFORE:
const KANBAN_PATH_PREFIXES = [
  '/v1/organizations',
  '/v1/projects',
  '/v1/project_statuses',
  '/v1/issues',
  '/v1/tags',
  '/v1/issue_assignees',
  '/v1/issue_relationships',
  '/v1/issue_tags',
  '/v1/issue_comments',
  '/v1/workspaces',
];

// AFTER:
const KANBAN_PATH_PREFIXES = [
  '/v1/organizations',
  '/v1/projects',
  '/v1/project_statuses',
  '/v1/issues',
  '/v1/tags',
  '/v1/issue_assignees',
  '/v1/issue_relationships',
  '/v1/issue_tags',
  '/v1/issue_comments',
  '/v1/workspaces',
  '/v1/fallback',  // catches all fallback shape sync paths
];
```

`'/v1/fallback'` must be added **after** more specific prefixes like `'/v1/issues'` and `'/v1/tags'` because `isKanbanPath()` uses `path.startsWith(prefix)` and checks in array order. Placing it last ensures specific routes match first.

**Acceptance Criteria:**
- [ ] `isKanbanPath('/v1/fallback/projects')` returns `true`
- [ ] Fallback shape sync requests route to local backend instead of failing
- [ ] `pnpm run check` passes (frontend type check)
- [ ] `pnpm run lint` passes

**Dependencies:** None

---

## Verification After All Fixes

Run the dev server and confirm the full flow:

```bash
pnpm run dev
```

Then:

1. `curl http://127.0.0.1:PORT/api/info` — verify `login_status.status: "loggedin"`
2. `curl http://127.0.0.1:PORT/api/remote/v1/organizations` — returns JSON with My Workspace org
3. `curl "http://127.0.0.1:PORT/api/remote/v1/projects?organization_id=ORG_ID"` — returns JSON (may be empty)
4. `curl "http://127.0.0.1:PORT/api/containers/attempt-context?ref=nonexistent"` — returns 404, server stays up
5. Open browser — "+" button visible, Projects section in Settings accessible
