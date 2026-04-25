# Kanban Board Local Re-Enablement Feasibility Report

**Date:** 2026-04-26
**Fork:** vibe-kanban-extended (from main branch, post-sunset)

---

## Executive Summary

Re-enabling the Kanban Board, Projects, and Issues for local-only use is **possible but requires significant engineering effort**. The sunset was a frontend-only change — all backend code and UI components remain intact. However, the Project/Issue/Kanban backend runs on the **remote (cloud) server** with **PostgreSQL**, while the local server uses **SQLite** and only handles workspaces/execution. To run these features locally, you must either:

1. **Option A (Recommended):** Implement local SQLite storage for Projects/Issues and rewrite the proxy routes to query the local DB directly.
2. **Option B:** Self-host the `crates/remote` server with a local PostgreSQL instance and point the local server to it.

---

## What Was Sunsetted

### Commit `97123d52` (April 24, 2026)
Changed exactly **3 files**, replacing functional kanban with a sunset page:

| File | Change |
|------|--------|
| `packages/web-core/src/pages/kanban/ProjectKanban.tsx` | 226 lines removed → renders `<ProjectSunsetPage />` |
| `packages/web-core/src/pages/kanban/LocalProjectKanban.tsx` | 25 lines removed → passthrough to `ProjectKanban` |
| `packages/web-core/src/pages/kanban/ProjectSunsetPage.tsx` | 63 lines added — "Project retired" export-only message |

### What's Disabled
- Kanban board view (drag-and-drop issue management)
- Issue creation, editing, deletion
- Issue detail panel (comments, relationships, sub-issues, workspaces)
- Project management (create, configure statuses, tags)
- All project routes now show the sunset page

### What Still Exists (Unused)
- **100% of UI components** remain in the codebase:
  - `packages/ui/src/components/KanbanBoard.tsx` — drag-and-drop board (@hello-pangea/dnd)
  - `packages/ui/src/components/KanbanCardContent.tsx` — issue card rendering
  - `packages/ui/src/components/KanbanIssuePanel.tsx` — issue create/edit panel
  - `packages/ui/src/components/KanbanFilterBar.tsx` — filters and search
  - `packages/web-core/src/features/kanban/ui/KanbanContainer.tsx` — 1,153 lines, full kanban logic
  - `packages/web-core/src/pages/kanban/KanbanIssuePanelContainer.tsx` — issue panel state management
  - Plus: assignee dialogs, filter dialogs, bulk actions, WYSIWYG editor, etc.
- **100% of backend code** remains:
  - `crates/remote/src/routes/issues.rs` — full issue CRUD API
  - `crates/remote/src/routes/projects.rs` — project CRUD API
  - `crates/remote/src/routes/tags.rs` — tag management API
  - `crates/remote/src/db/issues.rs` — issue repository
  - `crates/remote/src/db/projects.rs` — project repository
  - All migrations in `crates/remote/migrations/`
- **100% of API types** in `crates/api-types/src/` (Project, Issue, Tag, etc.)
- **100% of state management** (Zustand stores, hooks, contexts)
- **Route definitions** still exist and are active (just render sunset page)
- **MCP tools** for issues/projects still function (when remote server is available)

### What Was NOT Touched by Sunset
- Workspace management — fully functional
- Session/executor management — fully functional
- Container management — fully functional
- MCP server — still provides `list_issues`, `create_issue`, etc. (requires remote)
- Export functionality — added for data download

---

## Architecture: Why It Doesn't "Just Work" Locally

### The Dual-Server Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    FRONTEND (React)                       │
│  packages/local-web / packages/web-core                   │
└────────────┬────────────────────────────┬────────────────┘
             │                            │
             │ Local APIs                 │ Remote APIs (proxy)
             │ (/api/workspaces, etc.)    │ (/api/remote/issues, etc.)
             ▼                            ▼
┌────────────────────────┐    ┌──────────────────────────────┐
│   LOCAL SERVER         │    │   REMOTE SERVER (Cloud)       │
│   crates/server        │───▶│   crates/remote               │
│                        │    │                                │
│   SQLite (local DB)    │    │   PostgreSQL (remote DB)       │
│   - workspaces         │    │   - organizations              │
│   - sessions           │    │   - projects                   │
│   - execution_processes│    │   - issues                     │
│   - repos              │    │   - tags                       │
│   - files              │    │   - statuses                   │
│   - attachments        │    │   - assignees                  │
│                        │    │   - relationships              │
│   RemoteClient ────────│───▶│   - comments                   │
│   (optional HTTP proxy)│    │   - notifications              │
└────────────────────────┘    │   - workspaces (remote copy)   │
                              └──────────────────────────────┘
```

### Key Problem
All Project/Issue/Kanban API calls from the frontend go through:

1. **Frontend** → calls `/api/remote/issues` on the local server
2. **Local server** → uses `RemoteClient` to proxy to the remote cloud server
3. **Remote server** → queries PostgreSQL and returns data

When `VK_SHARED_API_BASE` is not set (no cloud server), the `RemoteClient` initialization fails, and ALL `/api/remote/*` routes return an error. The local SQLite database has **no tables** for projects, issues, tags, organizations, etc.

### Specific Missing Local Tables

The local SQLite DB (`crates/db/migrations/`) has these tables:
- workspaces, sessions, execution_processes, repos, project_repos
- workspace_repos, merges, attachments, files, scratches, tracked_prs

It does NOT have:
- organizations, projects, project_statuses
- issues, issue_assignees, issue_tags, issue_relationships
- tags, issue_comments, issue_comment_reactions
- notifications, workspaces (remote), pull_requests

---

## Option A: Implement Local SQLite Storage (Recommended)

### Approach
Create local DB tables for Projects/Issues, implement local route handlers that query SQLite directly instead of proxying to remote.

### Required Changes

#### 1. Database Migrations (SQLite)
Add new migrations in `crates/db/migrations/`:

```sql
-- Simplified schema (single-user, no multi-tenancy)
CREATE TABLE organizations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    issue_prefix TEXT NOT NULL DEFAULT 'VK',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '217 91% 60%',
    sort_order INTEGER NOT NULL DEFAULT 0,
    issue_counter INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE project_statuses (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    hidden INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE issues (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    issue_number INTEGER NOT NULL,
    simple_id TEXT NOT NULL,
    status_id TEXT NOT NULL REFERENCES project_statuses(id),
    title TEXT NOT NULL,
    description TEXT,
    priority TEXT, -- 'urgent', 'high', 'medium', 'low'
    start_date TEXT,
    target_date TEXT,
    completed_at TEXT,
    sort_order REAL NOT NULL DEFAULT 0,
    parent_issue_id TEXT REFERENCES issues(id) ON DELETE SET NULL,
    parent_issue_sort_order REAL,
    extension_metadata TEXT NOT NULL DEFAULT '{}',
    creator_user_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (project_id, issue_number)
);

CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    UNIQUE (project_id, name)
);

CREATE TABLE issue_tags (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    UNIQUE (issue_id, tag_id)
);

CREATE TABLE issue_assignees (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    assigned_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (issue_id, user_id)
);

CREATE TABLE issue_relationships (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    related_issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    relationship_type TEXT NOT NULL, -- 'blocking', 'related', 'has_duplicate'
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (issue_id, related_issue_id, relationship_type)
);

CREATE TABLE issue_comments (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    author_id TEXT,
    parent_id TEXT REFERENCES issue_comments(id) ON DELETE SET NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Note: UUIDs stored as TEXT in SQLite. No multi-user features (followers, notifications, reactions) needed for single-user local mode.

#### 2. DB Models (Rust)
Add models in `crates/db/src/`:

- `organization.rs` — CRUD for organizations (auto-create a default org on first run)
- `project.rs` — CRUD for projects with status management
- `issue.rs` — CRUD for issues with search, filtering, sort
- `tag.rs` — CRUD for tags
- `issue_tag.rs` — Issue-tag association
- `issue_assignee.rs` — Issue-assignee association
- `issue_relationship.rs` — Issue relationships
- `issue_comment.rs` — Issue comments

These models mirror the existing remote DB models in `crates/remote/src/db/` but use SQLite-compatible SQL.

#### 3. Rewrite Route Handlers
Replace proxy routes in `crates/server/src/routes/remote/` with direct DB queries:

**Current (proxy):**
```rust
// routes/remote/issues.rs
async fn list_issues(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<ResponseJson<ApiResponse<ListIssuesResponse>>, ApiError> {
    let client = deployment.remote_client()?; // FAILS without remote
    let response = client.list_issues(query.project_id).await?;
    Ok(ResponseJson(ApiResponse::success(response)))
}
```

**New (local DB):**
```rust
async fn list_issues(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<ResponseJson<ApiResponse<ListIssuesResponse>>, ApiError> {
    let db = deployment.db();
    let issues = IssueRepository::list_by_project(db, query.project_id).await?;
    Ok(ResponseJson(ApiResponse::success(issues)))
}
```

Files to modify:
- `crates/server/src/routes/remote/issues.rs`
- `crates/server/src/routes/remote/projects.rs`
- `crates/server/src/routes/remote/project_statuses.rs`
- `crates/server/src/routes/remote/tags.rs`
- `crates/server/src/routes/remote/issue_assignees.rs`
- `crates/server/src/routes/remote/issue_relationships.rs`
- `crates/server/src/routes/remote/issue_tags.rs`
- `crates/server/src/routes/remote/pull_requests.rs` (optional)
- `crates/server/src/routes/organizations.rs`

#### 4. Organization Handling
Single-user local mode needs a default organization. Options:
- Auto-create a default org on first startup
- Make organization_id optional in project creation
- Create a migration that inserts a default org

#### 5. Authentication Simplification
Local mode doesn't need OAuth. The frontend currently checks `isSignedIn` before showing kanban. Options:
- Auto-authenticate in local mode (always return a local user)
- Modify `useAuth` hook to return "signed in" for local mode
- Create a local user record in SQLite on first run

#### 6. Restore Frontend Components
Revert commit `97123d52` for these files:
```bash
# Restore the full kanban page
git show 97123d52^:packages/web-core/src/pages/kanban/ProjectKanban.tsx \
  > packages/web-core/src/pages/kanban/ProjectKanban.tsx

# Restore the local wrapper (with or without guide dialog)
git show 97123d52^:packages/web-core/src/pages/kanban/LocalProjectKanban.tsx \
  > packages/web-core/src/pages/kanban/LocalProjectKanban.tsx
```

Remove or hide `ProjectSunsetPage.tsx`.

#### 7. Frontend API Layer Adjustments
The frontend makes API calls through shared hooks and providers. Key files:
- `packages/web-core/src/shared/providers/remote/OrgProvider.tsx` — provides org context
- `packages/web-core/src/shared/providers/remote/ProjectProvider.tsx` — provides project context
- `packages/web-core/src/shared/hooks/useUserOrganizations.ts` — fetches orgs
- `packages/web-core/src/shared/hooks/useOrganizationProjects.ts` — fetches projects

These should work as-is if the local server returns the correct response shapes (which it will if route handlers return the same API types from `crates/api-types/`).

#### 8. Issue Number Generation
PostgreSQL uses triggers for auto-incrementing issue numbers and generating simple IDs (e.g., "VK-1"). SQLite needs this logic in application code (Rust) since SQLite triggers are less feature-rich.

### Estimated Effort: Option A
| Task | Effort |
|------|--------|
| SQLite migrations | 1 day |
| DB models (Rust) | 3-4 days |
| Route handler rewrites | 2-3 days |
| Organization/auth simplification | 1-2 days |
| Frontend restoration | 0.5 day |
| Testing and debugging | 2-3 days |
| **Total** | **~10-14 days** |

---

## Option B: Self-Host Remote Server with Local PostgreSQL

### Approach
Run the existing `crates/remote` server locally with a PostgreSQL database, and configure the local server to proxy to it via `VK_SHARED_API_BASE`.

### Advantages
- No Rust code changes needed for backend
- Reuses battle-tested remote server code
- Frontend works with zero changes (once kanban components are restored)
- All features work (notifications, comments, etc.)

### Required Setup

1. **Install PostgreSQL** locally
2. **Run remote server migrations:**
   ```bash
   sqlx migrate run --source crates/remote/migrations
   ```
3. **Build and run remote server** — this requires a binary entry point. Currently `crates/remote/src/bin/` only has `generate_types.rs`. The remote server was likely deployed as a separate service. You'd need to create a binary or use the existing server infrastructure.

4. **Create initial data:**
   - Create a user record
   - Create an organization
   - Set up OAuth or bypass auth

5. **Configure local server:**
   ```
   VK_SHARED_API_BASE=http://localhost:REMOTE_PORT
   ```

6. **Restore frontend components** (same as Option A, step 6)

### Challenges with Option B
- The remote server was designed for cloud deployment (OAuth, billing, ElectricSQL sync)
- Auth system expects OAuth providers (Google, GitHub) — needs bypass
- Missing binary entry point for standalone remote server
- PostgreSQL dependency adds operational complexity
- Remote server may have dependencies on cloud services (Azure blobs for attachments, etc.)

### Estimated Effort: Option B
| Task | Effort |
|------|--------|
| PostgreSQL setup + migrations | 0.5 day |
| Create remote server binary | 1-2 days |
| Auth bypass/simplification | 1-2 days |
| Cloud service stubs (Azure, etc.) | 1-2 days |
| Frontend restoration | 0.5 day |
| Integration testing | 2-3 days |
| **Total** | **~7-11 days** |

---

## Comparison

| Criteria | Option A (Local SQLite) | Option B (Local PostgreSQL) |
|----------|------------------------|----------------------------|
| Code changes | Significant (new DB layer) | Moderate (auth bypass) |
| Operational complexity | Low (SQLite, no extra process) | Higher (PostgreSQL + remote server) |
| Feature completeness | May need iteration | All features work |
| Maintenance burden | Custom code to maintain | Reuses existing remote code |
| Data portability | SQLite file | PostgreSQL dump |
| Multi-user potential | None (single-user) | Possible with more work |
| Startup time | Fast | Slower (two servers) |

---

## Recommended Approach

**Option A** (Local SQLite) is recommended for a fork because:
1. Single binary, single database — simple to run
2. No external dependencies (no PostgreSQL)
3. Fits the "local-first" use case perfectly
4. The remote server has cloud-specific dependencies that are hard to stub out
5. SQLite is sufficient for single-user kanban usage

### Implementation Sequence

1. **Restore frontend** (revert sunset commit for kanban pages)
2. **Add SQLite migrations** for project/issue tables
3. **Create local DB models** in `crates/db/src/`
4. **Rewrite route handlers** to use local DB instead of RemoteClient
5. **Handle organization** — auto-create default org on startup
6. **Handle authentication** — auto-login for local mode
7. **Test end-to-end** — create project, add statuses, create issues, use kanban board
8. **Iterate** — fix issues, add missing features

### What You Get
- Full drag-and-drop Kanban board
- Issue CRUD with priorities, tags, assignees (single user)
- Project management with configurable statuses
- Sub-issues and issue relationships
- Issue detail panel with comments
- MCP tools for issues work with local server
- Workspace integration (create workspace from issue)

### What You Don't Get (without extra work)
- Multi-user collaboration
- Real-time notifications
- Pull request tracking (requires GitHub integration)
- File attachments in issues (needs local file storage implementation)
- ElectricSQL real-time sync

---

## Key Files Reference

### Frontend (to restore/modify)
- `packages/web-core/src/pages/kanban/ProjectKanban.tsx` — restore from pre-sunset
- `packages/web-core/src/pages/kanban/LocalProjectKanban.tsx` — restore from pre-sunset
- `packages/web-core/src/pages/kanban/ProjectSunsetPage.tsx` — remove or keep as fallback
- `packages/web-core/src/features/kanban/ui/KanbanContainer.tsx` — main kanban UI (unchanged)
- `packages/web-core/src/shared/providers/remote/OrgProvider.tsx` — org data provider
- `packages/web-core/src/shared/providers/remote/ProjectProvider.tsx` — project data provider
- `packages/web-core/src/shared/hooks/useAuth.ts` — may need local-mode bypass

### Backend (to modify)
- `crates/db/migrations/` — add new SQLite migrations
- `crates/db/src/` — add new models
- `crates/server/src/routes/remote/*.rs` — rewrite to use local DB
- `crates/server/src/routes/organizations.rs` — rewrite for local org
- `crates/local-deployment/src/lib.rs` — local deployment setup
- `crates/api-types/src/` — shared types (unchanged, reused as-is)

### Reference Implementation (to copy from)
- `crates/remote/src/db/issues.rs` — issue repository logic
- `crates/remote/src/db/projects.rs` — project repository logic
- `crates/remote/src/db/tags.rs` — tag repository logic
- `crates/remote/src/routes/issues.rs` — issue route handlers
- `crates/remote/src/routes/projects.rs` — project route handlers
- `crates/remote/migrations/20260112000000_remote-projects.sql` — full schema reference
