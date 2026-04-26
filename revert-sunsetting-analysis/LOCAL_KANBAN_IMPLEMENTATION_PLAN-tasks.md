# Local Kanban Implementation — Task Breakdown

Generated: 2026-04-26
Source plan: [LOCAL_KANBAN_IMPLEMENTATION_PLAN.md](LOCAL_KANBAN_IMPLEMENTATION_PLAN.md)

---

## Parallelization Strategy

### Batch Dependency Diagram

```
Batch 0: Foundation (Tasks 1-2, sequential)
  │
  ▼
Batch 1: Parallel Core (Tasks 3-5, fully parallel, zero merge conflicts)
  │
  ▼
Batch 2: Routes & API (Tasks 6-9, Tasks 6-8 parallel, Task 9 depends on all)
  │
  ▼
Batch 3: Integration (Tasks 10-12, Tasks 10-11 parallel, Task 12 is final gate)
```

Detailed task flow:

```
                    ┌─────────┐
                    │ Task 1  │
                    │ Schema  │
                    └────┬────┘
                         │
                    ┌────▼────┐
                    │ Task 2  │
                    │ Types   │
                    └────┬────┘
                         │
          ┌──────────────┼──────────────┐
          │              │              │
    ┌─────▼─────┐  ┌────▼─────┐  ┌────▼─────┐
    │  Task 3   │  │  Task 4  │  │  Task 5  │
    │  Revert   │  │  Rust    │  │ Frontend │
    │  Sunset   │  │  Repos   │  │ API Glue │
    └─────┬─────┘  └────┬─────┘  └────┬─────┘
          │              │              │
          └──────┬───────┴──────┬───────┘
                 │              │
          ┌──────▼──────┐ ┌────▼──────────┐
          │   Task 6    │ │    Task 7     │
          │   Router +  │ │   Issue       │
          │   Fallbacks │ │   Mutations   │
          └──────┬──────┘ └────┬──────────┘
                 │              │
          ┌──────▼──────┐      │
          │   Task 8    │      │
          │   Entity    │      │
          │   Mutations │      │
          └──────┬──────┘      │
                 │             │
                 └──────┬──────┘
                  ┌─────▼─────┐
                  │  Task 9   │
                  │  Wire     │
                  │  Routes   │
                  └─────┬─────┘
                        │
          ┌─────────────┼─────────────┐
          │                           │
    ┌─────▼──────┐            ┌───────▼──────┐
    │  Task 10   │            │   Task 11    │
    │  Auth      │            │   First-Boot │
    │  Bypass    │            │   Init       │
    └─────┬──────┘            └───────┬──────┘
          │                           │
          └─────────────┬─────────────┘
                  ┌─────▼─────┐
                  │  Task 12  │
                  │  E2E      │
                  │  Verify   │
                  └───────────┘
```

### Summary Table

| # | Batch | Title | Target Files | Deps | Parallel With |
|---|-------|-------|-------------|------|---------------|
| 1 | 0 | SQLite Migration | `crates/db/migrations/NNNN_local_kanban_tables.sql` | None | -- |
| 2 | 0 | Shared Types Audit | `crates/api-types/src/`, `shared/types.ts` | Task 1 | -- |
| 3 | 1 | Revert Frontend Sunsetting | `packages/web-core/src/pages/kanban/*`, `packages/local-web/src/pages/kanban/*` | Tasks 1, 2 | Tasks 4, 5 |
| 4 | 1 | Rust Kanban Repos | `crates/db/src/models/{organization,issue,project_status,issue_assignee,issue_relationship,kanban_tag,issue_tag,issue_comment}.rs` | Tasks 1, 2 | Tasks 3, 5 |
| 5 | 1 | Frontend API Path Mapping | `packages/web-core/src/shared/lib/remoteApi.ts` | Tasks 1, 2 | Tasks 3, 4 |
| 6 | 2 | Router + Shape Fallbacks | `crates/server/src/routes/kanban_v1/{mod,shape_fallbacks}.rs` | Tasks 3-5 | Tasks 7, 8 |
| 7 | 2 | Issue Mutation Routes | `crates/server/src/routes/kanban_v1/issues.rs` | Tasks 3-5 | Tasks 6, 8 |
| 8 | 2 | Entity Mutation Routes | `crates/server/src/routes/kanban_v1/{projects,project_statuses,tags,issue_tags,issue_assignees,issue_relationships}.rs` | Tasks 3-5 | Tasks 6, 7 |
| 9 | 2 | Wire Routes in Server | `crates/server/src/routes/mod.rs` | Tasks 6-8 | -- |
| 10 | 3 | Local Auth Bypass | `crates/local-deployment/src/lib.rs` | Task 9 | Task 11 |
| 11 | 3 | First-Boot Initialization | `crates/local-deployment/src/first_boot.rs` (NEW) | Task 9 | Task 10 |
| 12 | 3 | E2E Verification | All | Tasks 10, 11 | -- |

### Merge Order Rules

**Strict sequential merge between batches. Each batch must fully merge and pass `cargo check --workspace` before the next begins.**

1. **Batch 0 → Batch 1**: Schema migration MUST land first — `sqlx::query!` macros in Batch 1 compile against migrated schema.
2. **Batch 1 → Batch 2**: Route handlers import repository types from Batch 1. Frontend changes (Task 5) are independent but should merge alongside routes to avoid broken intermediate state.
3. **Batch 2 → Batch 3**: Auth bypass and initialization depend on kanban routes being wired and accessible.

**Within-batch rules:**
- **Batch 1**: Tasks 3, 4, 5 can merge in any order (zero overlapping files). All must merge before Batch 2 starts.
- **Batch 2**: Tasks 6, 7, 8 can merge in any order. All must merge before Task 9.
- **Batch 3**: Tasks 10, 11 can merge in either order (independent codepaths). Task 12 runs after both.

### Developer Assignment Tracks (3 developers)

**Track A — Backend Core (DB Developer)**

| Phase | Tasks | Estimate |
|-------|-------|----------|
| Week 1 | Task 1 (schema) | 1-2 days |
| Week 1-2 | Task 4 (repos) | 3-4 days |
| Week 2-3 | Task 8 (entity mutations) | 2-3 days |
| Week 3 | Task 11 (first-boot init) | 1-2 days |

**Track B — Backend Integration (API Developer)**

| Phase | Tasks | Estimate |
|-------|-------|----------|
| Week 1 | Task 2 (types audit) | 1 day |
| Week 1-2 | Task 6 (router + fallbacks) | 2-3 days |
| Week 2 | Task 7 (issue mutations) | 2-3 days |
| Week 2-3 | Task 9 (route wiring) | 1 day |
| Week 3 | Task 10 (auth bypass) | 1 day |

**Track C — Frontend (UI Developer)**

| Phase | Tasks | Estimate |
|-------|-------|----------|
| Week 1 | Task 3 (sunset revert) | 1 day |
| Week 1 | Task 5 (API path mapping) | 1-2 days |
| Week 2-3 | Manual testing + iteration | 2-3 days |
| Week 3 | Task 12 (E2E verification) | 1-2 days |

**Critical path**: Task 1 → Task 2 → Task 4 → Task 6 → Task 9 → Task 10 → Task 12

**Risk areas:**
1. **Response shape mismatches** (Batch 2): Remote returns unwrapped JSON for `/v1/`. Local must NOT wrap in `ApiResponse`. Highest integration risk — prototype one endpoint early.
2. **SQLx compile-time checking** (Batch 1): `sqlx::query!` validates SQL at compile time. Must run `pnpm run prepare-db` after Batch 0 lands.
3. **SQLite vs PostgreSQL syntax** (Batch 0): UUID generation, timestamp types, and trigger logic must be handled in Rust, not SQL.

---

## Batch 0 — Foundation (Sequential)

### Task 1: SQLite Migration for Kanban Tables

**Title**: Create SQLite migration for kanban schema tables

**Description**:

Create `crates/db/migrations/20260426000000_local_kanban_tables.sql` adapting the PostgreSQL schema from `crates/remote/migrations/20260112000000_remote-projects.sql` to SQLite.

The local DB has NO kanban tables. The existing `projects` table has columns: `id BLOB, name TEXT, git_repo_path TEXT, setup_script TEXT, created_at TEXT, updated_at TEXT` plus later additions (`dev_script`, `cleanup_script`, `copy_files`, `remote_project_id BLOB`, `default_agent_working_dir TEXT`).

**New tables to create:**

1. **organizations** — `id BLOB PK, name TEXT, slug TEXT UNIQUE, is_personal INTEGER DEFAULT 0, issue_prefix TEXT NOT NULL DEFAULT 'VK', created_at TEXT, updated_at TEXT`
2. **project_statuses** — `id BLOB PK, project_id BLOB FK→projects, name TEXT, color TEXT, sort_order INTEGER, hidden INTEGER DEFAULT 0, created_at TEXT`
3. **issues** — `id BLOB PK, project_id BLOB FK→projects, issue_number INTEGER, simple_id TEXT UNIQUE, status_id BLOB FK→project_statuses, title TEXT, description TEXT, priority TEXT CHECK(priority IN ('urgent','high','medium','low')) DEFAULT 'medium', start_date TEXT, target_date TEXT, completed_at TEXT, sort_order INTEGER DEFAULT 0, parent_issue_id BLOB FK→issues, parent_issue_sort_order REAL, extension_metadata TEXT DEFAULT '{}', creator_user_id BLOB, created_at TEXT, updated_at TEXT`
4. **issue_assignees** — `id BLOB PK, issue_id BLOB FK→issues, user_id BLOB, assigned_at TEXT, UNIQUE(issue_id, user_id)`
5. **issue_relationships** — `id BLOB PK, issue_id BLOB FK→issues, related_issue_id BLOB FK→issues, relationship_type TEXT CHECK(relationship_type IN ('blocking','related','has_duplicate')), created_at TEXT, UNIQUE(issue_id, related_issue_id, relationship_type)`
6. **kanban_tags** — named `kanban_tags` (NOT `tags` — collision with existing [tag.rs](crates/db/src/models/tag.rs) which maps to `tags` table with `tag_name`/`content` columns). Columns: `id BLOB PK, project_id BLOB FK→projects, name TEXT, color TEXT, created_at TEXT, updated_at TEXT`
7. **issue_tags** — `id BLOB PK, issue_id BLOB FK→issues, tag_id BLOB FK→kanban_tags, UNIQUE(issue_id, tag_id)`
8. **issue_comments** — `id BLOB PK, issue_id BLOB FK→issues, author_id BLOB, parent_id BLOB FK→issue_comments, message TEXT, created_at TEXT, updated_at TEXT`

**ALTER existing table:**

- **projects** — add `color TEXT NOT NULL DEFAULT '0 0% 0%'`, `issue_counter INTEGER NOT NULL DEFAULT 0`, `organization_id BLOB` (nullable)

**SQLite-specific rules:**
- `PRAGMA foreign_keys = ON;` at top
- No triggers — `simple_id` auto-increment handled in Rust (Task 4)
- No ENUM types — TEXT + CHECK constraints
- UUID as BLOB (16 bytes), timestamps as TEXT, booleans as INTEGER
- JSONB as TEXT, parse with serde_json in Rust
- Skip: `users`, `notifications`, `pull_requests`, `issue_followers`, `issue_comment_reactions`, `project_notification_preferences`

**Indexes:** `idx_issues_project_id`, `idx_issues_status_id`, `idx_issues_parent_issue_id`, `idx_issues_simple_id`, `idx_issue_comments_issue_id`, `idx_issue_comments_parent_id`

**Acceptance Criteria:**

- [ ] File `crates/db/migrations/20260426000000_local_kanban_tables.sql` exists
- [ ] Migration runs cleanly against empty SQLite DB (`sqlx migrate run`)
- [ ] All 8 new tables created with correct column types
- [ ] `projects` table gains `color`, `issue_counter`, `organization_id` columns
- [ ] CHECK constraints for `issue_priority` and `issue_relationship_type`
- [ ] UNIQUE constraint on `issues(project_id, issue_number)`
- [ ] UNIQUE constraints on junction tables
- [ ] Foreign keys use `ON DELETE CASCADE` where appropriate
- [ ] No `tags` table created — kanban tags table named `kanban_tags`
- [ ] `cargo test --workspace` passes
- [ ] `pnpm run backend:check` passes

**Dependencies**: None

---

### Task 2: Shared Types Audit & Extension

**Title**: Audit and extend shared API types for local kanban

**Description**:

Audit `crates/api-types/src/` to confirm available types and fix gaps for local kanban.

**Already available in api-types:**

| Module | Types |
|--------|-------|
| `issue.rs` | `Issue`, `IssuePriority`, `CreateIssueRequest`, `UpdateIssueRequest`, `SearchIssuesRequest`, `ListIssuesResponse` |
| `project.rs` | `Project` (with `organization_id`, `color`), `CreateProjectRequest`, `ListProjectsResponse`, `BulkUpdateProjectsRequest/Response` |
| `project_status.rs` | `ProjectStatus`, `CreateProjectStatusRequest`, `UpdateProjectStatusRequest`, `ListProjectStatusesResponse` |
| `organizations.rs` | `Organization` (with `issue_prefix`), `ListOrganizationsResponse` |
| `tag.rs` | `Tag` (with `project_id`, `color`), `CreateTagRequest`, `ListTagsResponse` |
| `issue_assignee.rs` | `IssueAssignee`, `CreateIssueAssigneeRequest`, `ListIssueAssigneesResponse` |
| `issue_relationship.rs` | `IssueRelationship`, `IssueRelationshipType`, `CreateIssueRelationshipRequest` |
| `issue_comment.rs` | `IssueComment`, `CreateIssueCommentRequest`, `ListIssueCommentsResponse` |
| `issue_tag.rs` | `IssueTag`, `CreateIssueTagRequest`, `ListIssueTagsResponse` |
| `response.rs` | `MutationResponse<T>` (data + txid), `DeleteResponse` (txid) |

**Actions needed:**

1. **Verify `MutationResponse<T>` for SQLite**: `txid: i64` comes from PostgreSQL's `pg_current_xact_id()`. For local: always return `txid: 0` to maintain frontend compatibility. Document in code comment.

2. **Add missing bulk-update types**: Check if `BulkUpdateIssuesRequest/Response`, `BulkUpdateProjectStatusesRequest/Response` exist. If missing, add them following `BulkUpdateProjectsRequest/Response` pattern.

3. **Verify enum SQLite compatibility**: `IssuePriority` and `IssueRelationshipType` use `sqlx(type_name=...)` which is Postgres-specific. For SQLite, sqlx uses `FromStr`/`Display` for TEXT mapping. Verify this compiles or adjust annotations.

4. **Make `CreateOrganizationRequest.slug` optional** for local use (auto-generate from name).

5. Run `pnpm run generate-types` to regenerate `shared/types.ts`.

**Files to modify:**
- `crates/api-types/src/response.rs` — document txid:0 strategy
- `crates/api-types/src/issue.rs` — verify/fix SQLite enum compat
- `crates/api-types/src/organizations.rs` — make slug optional
- `shared/types.ts` — regenerated (do not edit manually)

**Acceptance Criteria:**

- [ ] All kanban types documented as "available" or "needs adaptation"
- [ ] `MutationResponse<T>` txid:0 strategy documented
- [ ] Missing bulk-update types added
- [ ] `IssuePriority`/`IssueRelationshipType` confirmed SQLite-compatible
- [ ] `pnpm run generate-types` succeeds
- [ ] `pnpm run check` passes
- [ ] `cargo check --workspace` passes

**Dependencies**: Task 1

---

## Batch 1 — Parallel Core (all depend on Batch 0, zero merge conflicts between tasks)

### Task 3: Revert Frontend Sunsetting

**Title**: Restore kanban UI by reverting sunset page override

**Description**:

Commit `97123d52` replaced the kanban rendering with `<ProjectSunsetPage />`. Restore the original UI.

**Changes:**

1. **Restore [ProjectKanban.tsx](packages/web-core/src/pages/kanban/ProjectKanban.tsx)** — Replace the final `return <ProjectSunsetPage projectName={project?.name} />;` with the original:
   ```tsx
   return (
     <OrgProvider organizationId={organizationId}>
       <ProjectKanbanInner projectId={projectId} />
     </OrgProvider>
   );
   ```
   The full component code (`ProjectMutationsRegistration`, `ProjectKanbanBoard`, `ProjectKanbanLayout`, `ProjectKanbanInner`) is already in the current file. Only the final `return` needs restoring.

2. **Delete [ProjectSunsetPage.tsx](packages/web-core/src/pages/kanban/ProjectSunsetPage.tsx)** — Remove the file and its import from `ProjectKanban.tsx`.

3. **Recreate `LocalProjectKanban.tsx`** at `packages/local-web/src/pages/kanban/LocalProjectKanban.tsx` using `git show 97123d52^:packages/local-web/src/pages/kanban/LocalProjectKanban.tsx`.

4. **Verify no other references** to `ProjectSunsetPage` exist: `grep -rn "ProjectSunsetPage\|SunsetPage" packages/`

**Files to modify:**
- `packages/web-core/src/pages/kanban/ProjectKanban.tsx` — restore original return
- `packages/web-core/src/pages/kanban/ProjectSunsetPage.tsx` — DELETE
- `packages/local-web/src/pages/kanban/LocalProjectKanban.tsx` — recreate from git

**Acceptance Criteria:**

- [ ] `ProjectSunsetPage.tsx` deleted
- [ ] `ProjectKanban.tsx` renders `<OrgProvider>` > `<ProjectKanbanInner>`
- [ ] `LocalProjectKanban.tsx` restored in `packages/local-web/`
- [ ] No remaining references to `ProjectSunsetPage` in any `.ts`/`.tsx` file
- [ ] `pnpm run check` passes
- [ ] `pnpm run lint` passes

**Dependencies**: Tasks 1, 2

---

### Task 4: Rust Kanban Models & SQLite Repositories

**Title**: Create local SQLite repository layer for all kanban entities

**Description**:

Create 8 new model files in `crates/db/src/models/` following the existing pattern from [project.rs](crates/db/src/models/project.rs): structs with `Debug, Clone, FromRow, Serialize, Deserialize, TS`, static async methods, `&SqlitePool` parameter, `sqlx::query_as!` with SQLite syntax.

**Files to create:**

#### 4a. [organization.rs](crates/db/src/models/organization.rs)
Port from: `crates/remote/src/db/organizations.rs`
- `Organization` struct with `ensure_default_org()`, `find_all`, `find_by_id`, `create`, `update_name`
- `ensure_default_org()` creates "My Workspace" org (slug: "local", issue_prefix: "VK")

#### 4b. [project_status.rs](crates/db/src/models/project_status.rs)
Port from: `crates/remote/src/db/project_statuses.rs`
- `ProjectStatus` struct with `find_by_project`, `create`, `update`, `delete`, `create_defaults_for_project`
- `create_defaults_for_project(project_id)` inserts 6 statuses: Backlog, To do, In progress, In review, Done, Cancelled

#### 4c. [issue.rs](crates/db/src/models/issue.rs) — **most complex**
Port from: `crates/remote/src/db/issues.rs` (705 lines)
- `Issue` struct with `create`, `find_by_id`, `find_by_project`, `search`, `update`, `delete`
- **Critical**: `create` method must generate `simple_id` in a transaction:
  ```rust
  pub async fn create(pool: &SqlitePool, req: &CreateIssueRequest) -> Result<Self, sqlx::Error> {
      let mut tx = pool.begin().await?;
      // 1. Atomic increment issue_counter
      let issue_number: i32 = sqlx::query_scalar!(
          r#"UPDATE projects SET issue_counter = issue_counter + 1
             WHERE id = $1 RETURNING issue_counter AS "n!: i32""#,
          req.project_id
      ).fetch_one(&mut *tx).await?;
      // 2. Get issue_prefix from org
      let prefix = // ... query organizations
      // 3. Build simple_id: "VK-{issue_number}"
      let simple_id = format!("{}-{}", prefix, issue_number);
      // 4. INSERT issue
      // 5. Commit
      tx.commit().await?;
  }
  ```
- `search` uses dynamic WHERE building (sqlx doesn't support dynamic `query_as!`)

#### 4d. [issue_assignee.rs](crates/db/src/models/issue_assignee.rs)
- `IssueAssignee` struct with `find_by_issue`, `create`, `delete`

#### 4e. [issue_relationship.rs](crates/db/src/models/issue_relationship.rs)
- `IssueRelationship` struct with `find_by_issue`, `create`, `delete`

#### 4f. [kanban_tag.rs](crates/db/src/models/kanban_tag.rs)
- `Tag` struct mapping to `kanban_tags` table (NOT `tags` — collision with existing)
- Methods: `find_by_project`, `create`, `update`, `delete`

#### 4g. [issue_tag.rs](crates/db/src/models/issue_tag.rs)
- `IssueTag` struct with `find_by_issue`, `create`, `delete`

#### 4h. [issue_comment.rs](crates/db/src/models/issue_comment.rs)
- `IssueComment` struct with `find_by_issue`, `create`, `update`, `delete`

#### 4i. Register all modules in [mod.rs](crates/db/src/models/mod.rs)
```rust
pub mod organization;
pub mod project_status;
pub mod issue;
pub mod issue_assignee;
pub mod issue_relationship;
pub mod kanban_tag;
pub mod issue_tag;
pub mod issue_comment;
```

#### 4j. Update existing [project.rs](crates/db/src/models/project.rs)
Add: `find_by_id`, `update_kanban_fields`, `find_by_organization`. Use `Option<T>` for new nullable columns.

**Important**: Run `pnpm run prepare-db` after creating models to update SQLx offline data.

**Acceptance Criteria:**

- [ ] All 8 new model files created in `crates/db/src/models/`
- [ ] All registered in `mod.rs`
- [ ] `organization.rs`: `ensure_default_org()`, CRUD methods
- [ ] `project_status.rs`: `create_defaults_for_project()` with 6 statuses
- [ ] `issue.rs`: `create` with transactional simple_id generation
- [ ] `issue.rs`: `search` with dynamic WHERE
- [ ] `kanban_tag.rs`: maps to `kanban_tags` table
- [ ] `project.rs`: new methods for kanban fields
- [ ] All `sqlx::query_as!` calls use SQLite syntax (no `$N::type` casts)
- [ ] `cargo test --workspace` passes
- [ ] `pnpm run prepare-db` succeeds

**Dependencies**: Tasks 1, 2

---

### Task 5: Frontend API Path Mapping

**Title**: Adapt frontend API client to route kanban requests locally

**Description**:

Modify [remoteApi.ts](packages/web-core/src/shared/lib/remoteApi.ts) so kanban API requests go to the local backend when no remote server is configured.

**Implementation:**

```typescript
const KANBAN_PATH_PREFIXES = [
  '/v1/organizations', '/v1/projects', '/v1/project_statuses',
  '/v1/issues', '/v1/tags', '/v1/issue_assignees',
  '/v1/issue_relationships', '/v1/issue_tags', '/v1/issue_comments',
  '/v1/workspaces',
];

function isLocalMode(): boolean {
  return !getRemoteApiUrl();
}

function isKanbanPath(path: string): boolean {
  return KANBAN_PATH_PREFIXES.some(prefix => path.startsWith(prefix));
}

async function localApiRequest(path: string, options: RequestInit = {}): Promise<Response> {
  const headers = new Headers(options.headers ?? {});
  if (!headers.has('Content-Type')) headers.set('Content-Type', 'application/json');
  return fetch(`/api/remote${path}`, { ...options, headers });
}

export const makeRequest = async (
  path: string, options: RequestInit = {}, retryOn401 = true
): Promise<Response> => {
  if (isLocalMode() && isKanbanPath(path)) {
    return localApiRequest(path, options);
  }
  return makeAuthenticatedRequest(getRemoteApiUrl(), path, options, retryOn401);
};
```

No auth headers for local requests. Existing `bulkUpdate*` functions already use `makeRequest` — no changes needed.

**Files to modify:**
- `packages/web-core/src/shared/lib/remoteApi.ts`

**Acceptance Criteria:**

- [ ] `isLocalMode()` returns `true` when no remote API base configured
- [ ] `isKanbanPath()` identifies all kanban API paths
- [ ] `localApiRequest()` sends to `/api/remote${path}` without auth headers
- [ ] `makeRequest()` delegates to `localApiRequest()` for kanban paths in local mode
- [ ] `makeRequest()` falls through to remote for non-kanban paths
- [ ] `pnpm run check` passes
- [ ] `pnpm run lint` passes

**Dependencies**: Tasks 1, 2

---

## Batch 2 — Routes & API (depends on Batch 1)

### Task 6: Kanban V1 Router Module & Shape Fallback Handlers

**Title**: Create kanban_v1 route module with shape fallback handlers returning raw JSON

**Description**:

Create `crates/server/src/routes/kanban_v1/` with router setup and fallback handlers that query local SQLite and return **raw `axum::Json`** — NOT wrapped in `ApiResponse`.

The frontend's `useShape` fallback mechanism (`extractFallbackRows` in `collections.ts`) expects responses like `{ "issues": [...] }`, not `{ success: true, data: { ... } }`.

**Fallback handlers (port from [shape_routes.rs](crates/remote/src/shape_routes.rs)):**

| Path | Response Type |
|------|--------------|
| `/fallback/projects` | `Json(ListProjectsResponse)` |
| `/fallback/project_statuses` | `Json(ListProjectStatusesResponse)` |
| `/fallback/tags` | `Json(ListTagsResponse)` |
| `/fallback/issues` | `Json(ListIssuesResponse)` |
| `/fallback/issue_assignees` | `Json(ListIssueAssigneesResponse)` |
| `/fallback/issue_tags` | `Json(ListIssueTagsResponse)` |
| `/fallback/issue_relationships` | `Json(ListIssueRelationshipsResponse)` |

**Handler pattern:**
```rust
async fn fallback_list_issues(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ProjectFallbackQuery>,
) -> Result<Json<ListIssuesResponse>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let response = IssueRepository::search(pool, &SearchIssuesRequest {
        project_id: query.project_id,
        ..Default::default()
    }).await.map_err(|e| ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list issues"))?;
    Ok(Json(response))
}
```

**Key differences from remote:**
- No `ShapeScope` authorization — single local user
- No `Extension<RequestContext>` auth extractors
- Use `State<DeploymentImpl>`, get pool from `deployment.db().pool`

**Files to create:**
- `crates/server/src/routes/kanban_v1/mod.rs` — router setup with `pub fn router() -> Router<DeploymentImpl>`
- `crates/server/src/routes/kanban_v1/shape_fallbacks.rs` — all 7 fallback handlers

**Acceptance Criteria:**

- [ ] `kanban_v1/mod.rs` exists and declares submodules
- [ ] `kanban_v1/shape_fallbacks.rs` implements all 7 fallback handlers
- [ ] All handlers return `Result<Json<T>, ErrorResponse>` (raw, NOT `ResponseJson<ApiResponse<...>>`)
- [ ] No auth extractors
- [ ] Routes match paths from `shared/remote-types.ts`
- [ ] `cargo check -p vibe-kanban-server` passes

**Dependencies**: Tasks 4, 5

---

### Task 7: Issue Mutation Routes

**Title**: Implement issue CRUD and bulk mutation routes

**Description**:

Create `crates/server/src/routes/kanban_v1/issues.rs` implementing all issue mutation handlers, ported from [crates/remote/src/routes/issues.rs](crates/remote/src/routes/issues.rs).

**Routes:**
```
POST   /issues              → create_issue
GET    /issues              → list_issues (query: project_id)
POST   /issues/search       → search_issues
GET    /issues/{id}         → get_issue
PATCH  /issues/{id}         → update_issue
DELETE /issues/{id}         → delete_issue
POST   /issues/bulk         → bulk_update_issues
```

**Response formats (from `api_types`):**
- Create/Update: `MutationResponse<Issue>` = `{ data: Issue, txid: i64 }`
- Delete: `DeleteResponse` = `{ txid: i64 }`
- Bulk: `BulkUpdateIssuesResponse` = `{ data: Vec<Issue>, txid: i64 }`
- List/Search: `ListIssuesResponse`
- Get: raw `Issue`

**Synthetic txid**: `chrono::Utc::now().timestamp_millis()` — frontend only uses txid for ElectricSQL sync (not used locally).

**Differences from remote:**
- No auth extractors → `State<DeploymentImpl>`
- No notifications, analytics, auto-follow
- No `ensure_project_access` / `ensure_member_access`
- SQLite pool from `deployment.db().pool`

**Files to create:**
- `crates/server/src/routes/kanban_v1/issues.rs`

**Acceptance Criteria:**

- [ ] All 7 issue handlers implemented
- [ ] Create accepts `Json<CreateIssueRequest>`, returns `Json<MutationResponse<Issue>>`
- [ ] Update accepts `Path<Uuid>` + `Json<UpdateIssueRequest>`
- [ ] Delete returns `Json<DeleteResponse>`
- [ ] Bulk update works
- [ ] All return raw `axum::Json`, NOT `ResponseJson<ApiResponse<...>>`
- [ ] Synthetic txid generated
- [ ] No notification/analytics code
- [ ] `cargo check -p vibe-kanban-server` passes

**Dependencies**: Tasks 4, 6

---

### Task 8: Project, Status, Tag, and Relationship Mutation Routes

**Title**: Implement mutation routes for all remaining kanban entities

**Description**:

Create 6 route files under `crates/server/src/routes/kanban_v1/`, one per entity. Each ports from the corresponding file in `crates/remote/src/routes/`.

**Files to create:**

| File | Source | Routes |
|------|--------|--------|
| `projects.rs` | `crates/remote/src/routes/projects.rs` | list, get, create, update, delete, bulk_update |
| `project_statuses.rs` | `crates/remote/src/routes/project_statuses.rs` | list, get, create, update, delete, bulk_update |
| `tags.rs` | `crates/remote/src/routes/tags.rs` | list, get, create, update, delete |
| `issue_tags.rs` | `crates/remote/src/routes/issue_tags.rs` | list, get, create, delete (no update) |
| `issue_assignees.rs` | `crates/remote/src/routes/issue_assignees.rs` | list, get, create, delete (no update) |
| `issue_relationships.rs` | `crates/remote/src/routes/issue_relationships.rs` | list, get, create, delete (no update) |

**Common patterns for all:**
- Use `State<DeploymentImpl>`, no auth extractors
- Return raw `axum::Json<T>`, NOT `ResponseJson<ApiResponse<...>>`
- Synthetic txid via `chrono::Utc::now().timestamp_millis()`
- Color validation (`is_valid_hsl_color`) on projects, statuses, tags
- No notifications, no analytics, no authorization checks
- Junction tables (issue_tags, issue_assignees, issue_relationships): create + delete only, no update

**Handler template:**
```rust
async fn create_entity(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateRequest>,
) -> Result<Json<MutationResponse<Entity>>, ErrorResponse> {
    let pool = &deployment.db().pool;
    let entity = Repository::create(pool, &payload).await.map_err(db_error)?;
    Ok(Json(MutationResponse { data: entity, txid: local_txid() }))
}
```

**Acceptance Criteria:**

- [ ] All 6 files created with correct handlers
- [ ] Projects: 6 handlers (list, get, create, update, delete, bulk)
- [ ] Project statuses: 7 handlers (list, get, create, update, delete, bulk)
- [ ] Tags: 5 handlers (list, get, create, update, delete)
- [ ] Issue tags: 4 handlers (list, get, create, delete)
- [ ] Issue assignees: 4 handlers (list, get, create, delete)
- [ ] Issue relationships: 4 handlers (list, get, create, delete)
- [ ] Color validation on projects/statuses/tags
- [ ] All return raw `axum::Json`
- [ ] `mod.rs` updated with all 6 submodules
- [ ] `cargo check -p vibe-kanban-server` passes

**Dependencies**: Tasks 4, 6

---

### Task 9: Register Kanban V1 Routes in Server

**Title**: Mount kanban_v1 routes in server router and verify proxy config

**Description**:

Wire the new `kanban_v1` module into the server's route tree.

**Step 1: Modify [routes/mod.rs](crates/server/src/routes/mod.rs)**
```rust
pub mod kanban_v1;

// In router() function, add AFTER existing .nest("/remote", remote::router()):
.nest("/remote/v1", kanban_v1::router())
```

The kanban_v1 routes must NOT be behind relay signature middleware (they serve local data, not proxied).

**Step 2: Verify Vite proxy**

The proxy in `packages/local-web/vite.config.ts` already forwards `/api/*` to the backend. After Task 5's `remoteApi.ts` changes, frontend sends:
- `GET /api/remote/v1/fallback/issues?project_id=...`
- `POST /api/remote/v1/issues`
- etc.

These must map to handlers registered in Step 1.

**Step 3: Verify route path consistency**

Cross-reference mutation URLs from `shared/remote-types.ts`:

| Frontend URL | Server Route | Handler File |
|-------------|-------------|-------------|
| `/v1/projects` | `/api/remote/v1/projects` | `projects.rs` |
| `/v1/project_statuses` | `/api/remote/v1/project_statuses` | `project_statuses.rs` |
| `/v1/tags` | `/api/remote/v1/tags` | `tags.rs` |
| `/v1/issues` | `/api/remote/v1/issues` | `issues.rs` |
| `/v1/issue_assignees` | `/api/remote/v1/issue_assignees` | `issue_assignees.rs` |
| `/v1/issue_tags` | `/api/remote/v1/issue_tags` | `issue_tags.rs` |
| `/v1/issue_relationships` | `/api/remote/v1/issue_relationships` | `issue_relationships.rs` |

**Acceptance Criteria:**

- [ ] `routes/mod.rs` includes `pub mod kanban_v1;`
- [ ] `kanban_v1::router()` mounted at correct path
- [ ] Kanban routes NOT behind relay signature middleware
- [ ] All mutation and fallback URLs from `shared/remote-types.ts` map to correct handlers
- [ ] `cargo check -p vibe-kanban-server` passes
- [ ] Manual test: `curl http://localhost:3001/api/remote/v1/fallback/projects?organization_id=<uuid>` returns raw JSON without `ApiResponse` wrapping

**Dependencies**: Tasks 6, 7, 8

---

## Batch 3 — Integration (depends on Batch 2)

### Task 10: Local Auth Bypass and Default User

**Title**: Auto-authenticate local user when no remote server configured

**Description**:

Modify [crates/local-deployment/src/lib.rs](crates/local-deployment/src/lib.rs) so `get_login_status()` returns `LoggedIn` with a synthetic profile when running purely locally.

**Current flow** (lines 405-452):
1. `get_credentials()` → no OAuth credentials locally → returns `LoggedOut`
2. If credentials exist, `remote_client.profile()` → `RemoteClientNotConfigured` → `LoggedOut`

**Fix**: Before the credential check (line 406), add early-return for local-only mode:

```rust
let Ok(_client) = self.remote_client() else {
    let local_profile = ProfileResponse {
        user_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        username: Some("local-user".to_string()),
        email: "local@localhost".to_string(),
        providers: vec![ProviderProfile {
            provider: "local".to_string(),
            username: Some("local-user".to_string()),
            display_name: Some("Local User".to_string()),
            email: Some("local@localhost".to_string()),
            avatar_url: None,
        }],
    };
    self.auth_context.set_profile(local_profile.clone()).await;
    return LoginStatus::LoggedIn { profile: Some(local_profile) };
};
```

Remote-enabled path remains unchanged.

**Acceptance Criteria:**

- [ ] `GET /api/info` returns `login_status.status: "loggedin"` when no `VK_SHARED_API_BASE` set
- [ ] Remote-enabled auth still works when `VK_SHARED_API_BASE` IS set
- [ ] Frontend `useAuth().isSignedIn === true` on local-only run
- [ ] `cargo check --workspace` passes

**Dependencies**: Task 9

---

### Task 11: First-Boot Initialization

**Title**: Seed default org, project, statuses, and tags on first boot

**Description**:

Create `crates/local-deployment/src/first_boot.rs` with `initialize_if_empty(pool)` that seeds an empty SQLite DB.

**Logic:**

1. **Check if needed**: Query `organizations` table. If count > 0, return early.
2. **Create default organization**: id = `Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"vibe-kanban-local")`, name = "My Workspace", slug = "local-workspace", issue_prefix = "VK"
3. **Create default project**: name = "Main Project", color = "217 91% 60%", issue_counter = 0
4. **Create 6 project statuses** (from remote `DEFAULT_STATUSES`):

| Name | Color | Sort | Hidden |
|------|-------|------|--------|
| Backlog | 220 9% 46% | 0 | true |
| To do | 217 91% 60% | 1 | false |
| In progress | 38 92% 50% | 2 | false |
| In review | 258 90% 66% | 3 | false |
| Done | 142 71% 45% | 4 | false |
| Cancelled | 0 84% 60% | 5 | true |

5. **Create 4 default tags** (from remote `DEFAULT_TAGS`):

| Name | Color |
|------|-------|
| bug | 355 65% 53% |
| feature | 124 82% 30% |
| documentation | 205 100% 40% |
| enhancement | 181 72% 78% |

All operations in a single SQLite transaction. Call from `initialize_deployment()` in startup after DB creation.

**Files to create:**
- `crates/local-deployment/src/first_boot.rs` (NEW)

**Files to modify:**
- `crates/local-deployment/src/lib.rs` — add `mod first_boot;`, call from init

**Acceptance Criteria:**

- [ ] Fresh DB: creates 1 org, 1 project, 6 statuses, 4 tags
- [ ] Existing DB: no duplicates
- [ ] Organization has `issue_prefix = "VK"`
- [ ] Project has `issue_counter = 0`
- [ ] All operations atomic (single transaction)
- [ ] `cargo check --workspace` passes
- [ ] Unit test: call on in-memory SQLite, verify created; call again, verify no duplicates

**Dependencies**: Task 9

---

### Task 12: End-to-End Integration Verification

**Title**: Verify all batches work together

**Description**:

Non-code quality gate task. Execute all steps in order; any failure blocks the merge.

**Automated checks:**

1. `cargo check --workspace` — zero errors
2. `cargo test --workspace` — all pass
3. `pnpm run check` — zero errors
4. `pnpm run format` — no changes
5. `pnpm run lint` — zero errors

**Manual verification (requires running server):**

6. Delete `db.v2.sqlite`, run `pnpm run dev` → verify init messages in logs
7. Navigate to app → no login prompt, user identity shown
8. Kanban board renders with columns (To do, In progress, In review, Done) — no sunset page
9. Create issue "Test issue" → `simple_id` = "VK-1", appears in "To do"
10. Drag VK-1 to "In progress" → refresh → persists
11. Open VK-1 → add comment → assign local user → verify
12. Create tag "urgent" → assign to issue → verify badge
13. Create second issue → `simple_id` = "VK-2"
14. Stop + restart server → verify all data persists
15. Restart again → no duplicate orgs/projects, counter not reset

**Acceptance Criteria:**

- [ ] All 15 steps pass without errors

**Dependencies**: Tasks 10, 11
