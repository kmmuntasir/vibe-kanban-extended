# Root Cause Analysis — Projects Menu & "+" Button Not Working in Local Mode

**Date:** 2026-05-10
**Branch:** `analysis/enable-local-backend-and-db`
**Server:** http://127.0.0.1:35187/ (the port may change in subsequent run)

---

## Symptoms

1. "Projects" menu in Settings is inaccessible (shows sign-in prompt)
2. "+" icon in left sidebar cannot add new projects (hidden/shows sign-in popover)
3. Backend crashes 2s after startup with 500 on `/api/containers/attempt-context`

---

## Root Cause #1 (CRITICAL): Double-Nesting Path Bug — `organizations.rs` Route Never Matches

### The Bug

**File:** [organizations.rs:10](crates/server/src/routes/kanban_v1/organizations.rs#L10)

```rust
// organizations.rs — registers route at "/organizations"
pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/organizations", get(list_organizations))
}
```

**File:** [mod.rs:45](crates/server/src/routes/kanban_v1/mod.rs#L45)

```rust
// mod.rs — nests under "/organizations" prefix
.nest("/organizations", organizations::router())
```

The `router()` function registers `GET /organizations`. mod.rs nests this router under the path `/organizations`. Axum's `.nest()` strips the prefix before passing to the inner router:

- Request: `GET /organizations`
- Axum strips prefix `/organizations` → remaining path: `/`
- Inner router looks for route matching `/`
- Inner router has route at `/organizations` → **NO MATCH**

The route only matches at the doubled path: `GET /organizations/organizations`.

### Proof

```bash
# Correct path (broken):
$ curl http://127.0.0.1:35187/api/remote/v1/organizations
# Returns: HTML (SPA catch-all) — 404 effectively

# Doubled path (accidentally works):
$ curl http://127.0.0.1:35187/api/remote/v1/organizations/organizations
# Returns: {"organizations":[{"id":"...","name":"My Workspace",...}]}
```

### Impact Chain

This single bug cascades into both user-facing failures:

1. Frontend loads → `/api/info` returns `login_status.status: "loggedin"` → `isSignedIn = true` ✓
2. `useUserOrganizations()` hook fires → calls `GET /v1/organizations`
3. Local mode rewrites to `GET /api/remote/v1/organizations`
4. Route doesn't match → returns HTML (catch-all) → **JSON parse error** in React Query
5. `organizations = []` (empty) → `useEffect` exits early (`if (organizations.length === 0) return`)
6. No `selectedOrgId` → projects shape sync disabled (`enabled: isSignedIn && !!selectedOrgId`)
7. Navbar/AppBar has no org context → "+" button hidden, project list empty
8. Settings → Projects section requires org context → shows sign-in prompt as fallback

### Fix

Change [organizations.rs:10](crates/server/src/routes/kanban_v1/organizations.rs#L10):

```rust
// WRONG:
Router::new().route("/organizations", get(list_organizations))

// CORRECT:
Router::new().route("/", get(list_organizations))
```

---

## Root Cause #2 (CRITICAL): Same Double-Nesting in `issue_comments.rs`

### The Bug

**File:** [issue_comments.rs:18-28](crates/server/src/routes/kanban_v1/issue_comments.rs#L18-L28)

```rust
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/issue_comments", get(...).post(...))        // line 21
        .route("/issue_comments/{id}", get(...).patch(...).delete(...))  // line 25
}
```

mod.rs nests at `.nest("/issue_comments", issue_comments::router())`.

Same double-nesting — routes only match at `/issue_comments/issue_comments` and `/issue_comments/issue_comments/{id}`.

### Fix

```rust
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(...).post(...))
        .route("/{id}", get(...).patch(...).delete(...))
}
```

---

## Root Cause #3 (HIGH): Backend Crash — `containers/attempt-context` Returns 500

### The Bug

**File:** [workspace.rs:350-372](crates/db/src/models/workspace.rs#L350-L372)

`resolve_container_ref_by_prefix()` returns `sqlx::Error::RowNotFound` when no workspace matches the container ref path. This is semantically wrong — "no match" is a normal outcome, not a data integrity error.

**File:** [containers.rs:37-48](crates/server/src/routes/containers.rs#L37-L48)

The handler maps ALL database errors to 500:
```rust
let info = Workspace::resolve_container_ref_by_prefix(...)
    .await
    .map_err(ApiError::Database)?;  // RowNotFound → 500
```

**File:** [error.rs:489](crates/server/src/error.rs#L489)

```rust
ApiError::Database(_) => ErrorInfo::internal("DatabaseError"),  // Always 500
```

### The Crash

```
16:18:03.310Z  INFO server: Opening browser...
16:18:05.368Z ERROR server::error: API request failed error_type="DatabaseError" status=500
16:18:05.382Z  INFO utils::process: Sending SIGINT to process group
16:18:07.384Z  INFO utils::process: Sending SIGTERM to process group
16:18:09.385Z  INFO utils::process: Sending SIGKILL to process group
```

The MCP client calls `containers/attempt-context` at startup. The 500 triggers a fatal error in scoped/orchestrator mode, killing the process. The npx-cli or user restarts the server.

### Fix

Option A — In the handler (minimal):
```rust
// containers.rs
let info = Workspace::resolve_container_ref_by_prefix(&deployment.db().pool, &payload.container_ref)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound("Container not found".into()),
        other => ApiError::Database(other),
    })?;
```

Option B — In the model (cleaner):
Change `resolve_container_ref_by_prefix` to return `Result<Option<ContainerInfo>>` instead of using `RowNotFound` as a sentinel.

---

## Root Cause #4 (MEDIUM): KANBAN_PATH_PREFIXES Missing Fallback Paths

### The Bug

**File:** [remoteApi.ts:47-58](packages/web-core/src/shared/lib/remoteApi.ts#L47-L58)

```typescript
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
```

The `isKanbanPath()` function checks `path.startsWith(prefix)`. Fallback URLs like `/v1/fallback/projects` do NOT start with `/v1/projects` (they start with `/v1/fallback/`), so they don't match any prefix.

In local mode, non-matching paths fall through to `makeAuthenticatedRequest()` which requires a remote API base URL and auth token — both unavailable locally.

### Fix

Add fallback prefixes:
```typescript
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
  '/v1/fallback',  // catches all fallback paths
];
```

---

## Verified Working vs Broken Routes

### Working (tested with curl)

| Endpoint | Status | Response |
|----------|--------|----------|
| `/api/info` | 200 | `login_status.status: "loggedin"`, `shared_api_base: null` |
| `/api/remote/v1/projects?organization_id=...` | 200 | `{"projects":[]}` |
| `/api/remote/v1/fallback/projects?organization_id=...` | 200 | `{"projects":[]}` |
| `/api/remote/v1/organizations/organizations` | 200 | Returns My Workspace org (doubled path — proves the bug) |

### Broken

| Endpoint | Status | Root Cause |
|----------|--------|-----------|
| `/api/remote/v1/organizations` | HTML (404) | Double-nesting: route is at `/organizations/organizations` |
| `/api/remote/v1/organizations/` | HTML (404) | Same — trailing slash doesn't help |
| `/api/remote/v1/issue_comments?issue_id=...` | HTML (404) | Double-nesting: route is at `/issue_comments/issue_comments` |
| `/api/containers/attempt-context` | 500 | RowNotFound mapped to 500, crashes server |

---

## Fix Priority

| # | Fix | Impact | Effort |
|---|-----|--------|--------|
| 1 | Fix organizations.rs path (`/organizations` → `/`) | Unblocks entire projects flow | 1 line |
| 2 | Fix issue_comments.rs paths (`/issue_comments` → `/`) | Unblocks comment CRUD | 2 lines |
| 3 | Fix containers/attempt-context 500 → 404 | Prevents backend crash | ~5 lines |
| 4 | Add `/v1/fallback` to KANBAN_PATH_PREFIXES | Enables fallback sync routing | 1 line |

Fixes 1 and 3 are independent and can be done in parallel. Fix 2 is independent. Fix 4 is independent.

---

## Why the Plan's Verification Missed These

The original plan (LOCAL_KANBAN_COMPLETE_FIX_PLAN.md) specified in organizations.rs:

```rust
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/organizations", get(list_organizations))
}
```

The plan used `"/organizations"` as the inner route path. But the existing pattern in the codebase (used by `projects.rs`, `tags.rs`, etc.) is:

```rust
// projects.rs — uses "/" because mod.rs nests at "/projects"
Router::new().route("/", get(list_projects).post(create_project))
```

The plan's code was copied verbatim without adapting to the Axum `.nest()` semantics. All 6 pre-existing route modules use `"/"` as the base path inside their routers. Both new modules (`organizations.rs`, `issue_comments.rs`) have the wrong inner path.

The plan also did not account for the `KANBAN_PATH_PREFIXES` routing gap in the frontend, nor the `containers/attempt-context` crash that only manifests when the DB is empty of workspaces.
