# Local Kanban Implementation Plan

This document details the step-by-step technical plan to re-enable the Kanban board by transitioning from a cloud-reliant proxy architecture to a local SQLite-backed system. This implementation follows Option A (Implement Local SQLite Storage) from the Feasibility Report.

## User Review Required

> [!WARNING]
> **Data Migration Strategy:** The current plan focuses on creating a fresh local SQLite database schema. It does not include a migration script to pull old data from the remote PostgreSQL database. If users need to access historical Kanban data from the remote cloud server, we will need to build an explicit "Export from Cloud / Import to Local" tool as a separate task. Please confirm if starting with empty local projects is acceptable for this initial phase.

## Open Questions

> [!IMPORTANT]
> **Local User Identity:** The remote server handled user authentication and organizations. For a fully local setup, should we auto-create a default local user and default organization upon startup, and bypass authentication entirely, or should we still prompt the user for a username/identity? (The current plan assumes auto-creating a default "Local User" and "Local Organization" on first boot).
>
> **Project Synchronization:** If a user has an active remote session, do we want to sync local projects with remote projects, or will local and remote be strictly isolated environments? (The current plan assumes strict isolation for simplicity).

---

## Proposed Changes

We will divide the implementation into 5 logical phases.

### Phase 1: Frontend Restoration

Revert the sunsetting changes to bring back the UI.

#### [MODIFY] [packages/web-core/src/pages/kanban/ProjectKanban.tsx](file:///home/munna/sonic/localhost/vibe-kanban-extended/packages/web-core/src/pages/kanban/ProjectKanban.tsx)
- Revert the `ProjectSunsetPage` override and restore the `ProjectKanban` component rendering logic (using `Group`, `Layout`, `Panel`, `KanbanContainer`, etc.) from commit `97123d52^`.

#### [DELETE] [packages/web-core/src/pages/kanban/ProjectSunsetPage.tsx](file:///home/munna/sonic/localhost/vibe-kanban-extended/packages/web-core/src/pages/kanban/ProjectSunsetPage.tsx)
- Remove the sunsetting interstitial page since it is no longer needed.

---

### Phase 2: Database Schema (SQLite Migrations)

We need to mirror the PostgreSQL schema from `crates/remote/migrations/20260112000000_remote-projects.sql` to SQLite within `crates/db/migrations/`.

#### [NEW] `crates/db/migrations/20260426000000_local_kanban_tables.sql`
Add the following tables (adapted for SQLite syntax):
- `organizations` (if not already sufficient, ensure `issue_prefix` exists)
- `projects` (add `color`, `issue_counter`)
- `project_statuses`
- `issues` (adapt auto-increment `issue_number` logic—likely handled in Rust layer rather than triggers for SQLite)
- `issue_assignees`
- `issue_relationships`
- `tags`
- `issue_tags`
- `issue_comments`

---

### Phase 3: DB Rust Models & Repositories (`crates/db/src`)

Create new models and SQLite repositories that mirror the functionality of `crates/remote/src/db/`.

#### [NEW] `crates/db/src/models/kanban.rs` (or individual files)
- Port the data access logic from PostgreSQL to SQLite `sqlx::query!`.
- Specifically, port the following repositories to `crates/db/src/models/`:
  - `IssueRepository`
  - `ProjectRepository`
  - `ProjectStatusRepository`
  - `TagRepository`
  - `IssueTagRepository`
  - `IssueAssigneeRepository`
  - `IssueRelationshipRepository`
  - `IssueCommentRepository`
- **Note on Issues ID Generation:** Instead of PostgreSQL triggers, `IssueRepository::create` will manually fetch and increment the project's `issue_counter` in a single transaction to generate `issue_number` and `simple_id` (e.g., `ISS-1`).

---

### Phase 4: Route Handlers Rewrite (New `crates/server/src/routes/kanban_v1/`)

**CRITICAL FINDING:** The current local proxy routes in `crates/server/src/routes/remote/` wrap their responses in `ApiResponse` (e.g., `{ success: true, data: { ... } }`). However, the frontend's ElectricSQL `useShape` fallback logic and Kanban mutation requests (like `ISSUE_MUTATION`) expect **raw JSON unwrapped responses**, precisely matching how the remote server implemented them in `crates/remote/src/shape_routes.rs`. Furthermore, the frontend `makeRequest` sends requests directly to `/v1/...`. 

To fix this, we will NOT modify the existing remote proxy handlers. Instead, we will create a dedicated API space that strictly mirrors the remote server's unwrapped responses and paths.

#### [NEW] `crates/server/src/routes/kanban_v1/mod.rs`
- Create a new router mounted at `/v1` or `/api/remote/v1` (we will intercept `/v1/` calls in `remoteApi.ts` to map to `/api/remote/v1/`).
- This router must return raw `axum::Json` (unwrapped), NOT `ResponseJson(ApiResponse(...))`.

#### [NEW] `crates/server/src/routes/kanban_v1/shape_fallbacks.rs`
- Port the fallback handlers from `crates/remote/src/shape_routes.rs` (e.g., `fallback_list_issues`, `fallback_list_projects`, `fallback_list_project_statuses`, etc.) to query the local SQLite DB.
- Ensure they return exactly the shape the frontend expects (e.g., `Json(ListIssuesResponse)`).

#### [NEW] `crates/server/src/routes/kanban_v1/issues.rs` (and other entities)
- Port the Kanban mutations (create/update/delete) from `crates/remote/src/routes/issues.rs` and other files to use local SQLite repositories.
- These must also return unwrapped responses (e.g., `Json(MutationResponse { data, txid })`).

#### [MODIFY] `packages/web-core/src/shared/lib/remoteApi.ts`
- Update `makeRequest` to automatically prefix paths starting with `/v1/` with `/api/remote` when running locally (`BUILD_TIME_API_BASE`), so the frontend successfully hits the backend Vite proxy.

---

### Phase 5: Auth & Context Initialization

Ensure the local frontend can bypass strict remote authentication.

#### [MODIFY] [crates/local-deployment/src/lib.rs](file:///home/munna/sonic/localhost/vibe-kanban-extended/crates/local-deployment/src/lib.rs)
- Modify `get_login_status` so that if the user is running strictly locally (no remote client configured), it auto-authenticates them into a default "Local User" profile rather than returning `LoggedOut`.
- Ensure an initial setup hook auto-creates a default organization (e.g., "My Workspace") and default project so the Kanban board has data on first launch.

---

## Verification Plan

### Automated Tests
- Run `cargo check --workspace` to ensure all Rust types compile after the route modifications.
- Run `pnpm run check` in `packages/web-core` to ensure frontend type safety.
- Create unit tests in `crates/db` for the new SQLite kanban repositories (creating issues, generating auto-incrementing simple IDs).

### Manual Verification
1. **First Boot Initialization:** Start the server with `pnpm run dev` and an empty SQLite DB. Verify that a default organization and project are created automatically.
2. **UI Rendering:** Navigate to `http://localhost:<FRONTEND_PORT>`. Verify that the Kanban board renders (no sunset page).
3. **Data Operations:**
   - Create a new task (Verify `simple_id` like `VK-1` generates correctly).
   - Move task between columns (Status updates).
   - Add a comment to the task.
   - Assign the task to the default user.
   - Create a tag and assign it to the task.
4. **Persistence:** Restart the `pnpm run dev` process and verify all issues, columns, and comments remain exactly as left.
