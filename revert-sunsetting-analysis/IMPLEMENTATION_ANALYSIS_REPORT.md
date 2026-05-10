# Local Kanban Implementation Analysis Report

**Date:** 2026-05-10
**Branch:** `analysis/find-out-feasibility-of-local-run`
**Analyst:** Claude Code (Opus)

---

## Executive Summary

The local kanban re-enablement follows **Option A (Local SQLite)** from KANBAN_LOCAL_FEASIBILITY_REPORT.md. All 5 phases are implemented correctly. However, **the local kanban_v1 routes are UNREACHABLE** because:

1. The compiled binary has `VK_SHARED_API_BASE=https://api.vibekanban.com` baked in via `option_env!` at compile time
2. `/api/info` returns `shared_api_base: "https://api.vibekanban.com"` 
3. Frontend detects non-null `shared_api_base`, switches to **remote mode**
4. ALL kanban API calls go directly to `https://api.vibekanban.com/v1/...` (cloud PostgreSQL)
5. The local SQLite kanban tables and routes are bypassed entirely
6. Data syncs across computers because it's stored on the **cloud**, not locally

**The implementation is correct, but the build configuration prevents it from being used.**

---

## 1. Database Verification

### Two Databases Exist

| Database | Location | Status |
|----------|----------|--------|
| Local SQLite | `~/Library/Application Support/ai.bloop.vibe-kanban/db.v2.sqlite` | Contains only seed data (1 org, 1 project, 0 issues) |
| Cloud PostgreSQL | `https://api.vibekanban.com` | Contains ALL user data (2 projects, 37+ issues, 2 workspaces) |

### Local SQLite State (verified via sqlite3)

```
organizations:    1 row  — "My Workspace" (first-boot seed)
projects:         1 row  — "Main Project" (first-boot seed)
project_statuses: 6 rows — seed defaults
kanban_tags:      4 rows — seed defaults
issues:           0 rows — no user data
```

### Cloud PostgreSQL State (verified via MCP tools)

```
organizations: 1 row  — "kmmuntasir@gmail.com's Org" (real account)
projects:      2 rows — "nano-quiz" (May 9), "cattle-brokerage" (May 9)
issues:        37+   — KMM-179 to KMM-215, created May 9-10
workspaces:    2     — Created May 10
```

### Why Two Databases?

The **same binary** serves both local and remote data:
- Workspaces/sessions → local SQLite (no remote dependency)
- Kanban/projects/issues → cloud API (because `shared_api_base` is configured)
- The MCP server (`vibe-kanban-mcp`) connects to the cloud API directly

---

## 2. Approach Analysis: What Was Implemented

### Selected Approach: Option A (Local SQLite)

All 5 phases are implemented in the codebase:

| Phase | Files | Status |
|-------|-------|--------|
| 1. Frontend Restoration | ProjectKanban.tsx restored, ProjectSunsetPage.tsx deleted | Done |
| 2. SQLite Migrations | `20260426000000_local_kanban_tables.sql` (9 tables) | Done |
| 3. Rust DB Models | 8 model files + project.rs extension (`crates/db/src/models/`) | Done |
| 4. Route Handlers | 9 files in `crates/server/src/routes/kanban_v1/`, mounted at `/api/remote/v1/` | Done |
| 5. Auth Bypass | `get_login_status()` auto-returns local-user when remote_client fails | Done |

**The code is correct. The build is wrong.**

---

## 3. Critical Deviation: Build Configuration

### The Bug

The backend binary is compiled with `option_env!("VK_SHARED_API_BASE")` which bakes the cloud API URL into the binary at **compile time**.

```rust
// crates/local-deployment/src/lib.rs:176-178
let api_base = std::env::var("VK_SHARED_API_BASE")
    .ok()
    .or_else(|| option_env!("VK_SHARED_API_BASE").map(|s| s.to_string()));
//                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//                        BAKED INTO BINARY AT COMPILE TIME
//                        Returns Some("https://api.vibekanban.com")
```

This causes the following chain reaction:

```
1. binary compiled with VK_SHARED_API_BASE=https://api.vibekanban.com
   ↓
2. option_env!("VK_SHARED_API_BASE") → Some("https://api.vibekanban.com")
   ↓
3. api_base = Some("https://api.vibekanban.com")
   ↓
4. remote_info.set_api_base("https://api.vibekanban.com")
   ↓
5. remote_client succeeds (not RemoteClientNotConfigured)
   ↓
6. get_login_status() goes through OAuth flow (credentials.json has valid token)
   ↓
7. /api/info returns shared_api_base: "https://api.vibekanban.com"
   ↓
8. Frontend ConfigProvider calls setRemoteApiBase("https://api.vibekanban.com")
   ↓
9. isLocalMode() returns false  ← THE CRITICAL SWITCH
   ↓
10. ALL kanban requests go to https://api.vibekanban.com/v1/...
   ↓
11. Local kanban_v1 routes at /api/remote/v1/... are NEVER CALLED
```

### Frontend Decision Tree (remoteApi.ts)

```typescript
export const makeRequest = async (path, options, retryOn401) => {
  if (isLocalMode() && isKanbanPath(path)) {
    return localApiRequest(path, options);  // → /api/remote/v1/... (local SQLite)
  }
  return makeAuthenticatedRequest(getRemoteApiUrl(), path, options, retryOn401);
  //                              ↑ "https://api.vibekanban.com"
  // → direct cloud API call with OAuth token
};
```

When `shared_api_base` is set (from `/api/info`), `isLocalMode()` is `false`, and all kanban paths go directly to the cloud.

### Evidence

```
$ curl http://localhost:61436/api/info
{
  "data": {
    "shared_api_base": "https://api.vibekanban.com",  ← NOT null
    "login_status": { "status": "loggedin" },           ← remote auth
    "version": "0.1.44"
  }
}
```

Running processes:
```
PID 15156: npx-cli/dist/macos-arm64/vibe-kanban     ← locally-built binary
          → TCP 172.16.20.174:62404 → 104.26.15.40:443 (api.vibekanban.com)
PID 20610: .vibe-kanban/bin/v0.1.44/.../vibe-kanban-mcp --mode global
```

The backend has an active HTTPS connection to `104.26.15.40` (Cloudflare IP for `api.vibekanban.com`).

---

## 4. Cross-Computer Data Sync: Root Cause (REVISED)

### Previous Finding (INCORRECT)
> "Each computer independently generates identical seed data (first-boot), creating an illusion of sync."

### Correct Finding

**Data IS genuinely synced across computers** because all kanban data goes to the cloud PostgreSQL server at `https://api.vibekanban.com`.

The flow:
1. User signs in with Google OAuth (`kmmuntasir@gmail.com`)
2. OAuth token stored in `~/Library/Application Support/ai.bloop.vibe-kanban/credentials.json`
3. Binary connects to `https://api.vibekanban.com` (baked into binary)
4. All project/issue CRUD goes to cloud PostgreSQL
5. When user switches to another computer, same OAuth credentials + same cloud API = same data

The MCP tools also connect to the cloud API (via `vibe-kanban-mcp` binary), confirming:
- **nano-quiz** project: 1 issue (KMM-179), created May 9
- **cattle-brokerage** project: 36 issues (KMM-180 to KMM-215), created May 9-10
- **2 workspaces**: d427-T3-implement-get, 487f-T2-create-userservice, created May 10

All created on home desktop, visible on work laptop — because they're on the **cloud**, not local.

### The Local SQLite Is Unused

The local kanban tables (organizations, projects, issues, etc.) contain ONLY first-boot seed data and are never written to during normal use because all kanban traffic goes to the cloud.

---

## 5. Architecture Diagram (Current Actual State)

```
┌──────────────────────────────────────────────────────────────┐
│                     FRONTEND (Browser)                        │
│                                                               │
│  makeRequest("/v1/issues", ...)                               │
│    │                                                          │
│    isLocalMode() → getRemoteApiUrl() !== ""                   │
│    → "https://api.vibekanban.com"  (from /api/info)           │
│    → RETURNS FALSE                                            │
│    │                                                          │
│    └── makeAuthenticatedRequest(                              │
│          "https://api.vibekanban.com",                         │
│          "/v1/issues",                                        │
│          OAuth token                                           │
│        )                                                      │
│        → DIRECT CLOUD CALL (bypasses local server entirely)    │
│                                                               │
└───────────────────────────────────────────────────────────────┘
                       │ HTTPS
                       ▼
┌──────────────────────────────────────────────────────────────┐
│         api.vibekanban.com (104.26.15.40:443)                 │
│                                                               │
│         Cloud PostgreSQL                                      │
│         - organizations (kmmuntasir@gmail.com's Org)          │
│         - projects (nano-quiz, cattle-brokerage)              │
│         - issues (KMM-179 to KMM-215)                         │
│         - workspaces, sessions, etc.                          │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│              LOCAL SERVER (port 61436)                         │
│                                                               │
│  /api/remote/v1  → kanban_v1::router()                        │
│    → local SQLite (UNUSED — frontend never calls these)       │
│                                                               │
│  /api/info       → returns shared_api_base (cloud URL)        │
│                                                               │
│  /api/remote     → remote::router() (old proxy, UNUSED)       │
│                                                               │
└──────────────────────┬───────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│  Local SQLite: ai.bloop.vibe-kanban/db.v2.sqlite              │
│  Seed data only: 1 org, 1 project, 6 statuses, 4 tags        │
│  0 user-created issues                                        │
└──────────────────────────────────────────────────────────────┘
```

---

## 6. Deviations from Feasibility Report & Implementation Plan

### Deviation 1: Build Configuration Prevents Local Routes from Being Used (CRITICAL)

**Plan expectation**: `remote_client()` fails → auth bypass → local kanban routes used.

**Actual**: `option_env!("VK_SHARED_API_BASE")` baked the cloud URL into the binary. `remote_client()` succeeds. Auth uses real OAuth. Frontend sends kanban requests directly to cloud.

**Impact**: The entire implementation (Phases 1-5) has zero effect at runtime. All local code exists but is unreachable.

### Deviation 2: Hybrid Route Architecture (Design Decision)

**Plan**: Rewrite existing proxy routes in `crates/server/src/routes/remote/` to use local DB.

**Actual**: Created new parallel route tree at `/api/remote/v1/` instead. Old proxy routes preserved.

**Impact**: Clean separation. Good design. But both are bypassed because frontend calls cloud directly.

### Deviation 3: Response Shape Strategy (Correct)

Shape fallback handlers return raw JSON. Mutation routes return `MutationResponse<T>`. Matches plan.

### Deviation 4: Task 12 Incomplete

Integration tests partially written. Not the root cause of the current issue.

---

## 7. Trace: What Happens After Removing `option_env!`

### Step-by-step code path analysis

After removing `option_env!("VK_SHARED_API_BASE")` from line 178, leaving only `std::env::var("VK_SHARED_API_BASE").ok()`:

**Server-side (startup):**
```
1. VK_SHARED_API_BASE not set at runtime → std::env::var → Err
2. api_base = None
3. remote_info.get_api_base() → None
4. remote_client() → Err(RemoteClientNotConfigured)
5. tracing::info!("VK_SHARED_API_BASE not set; remote features disabled")
```

**Server-side (GET /api/info):**
```
6. get_login_status():
   → remote_client() → Err → enters early-return block
   → Returns LoginStatus::LoggedIn {
       profile: ProfileResponse {
         user_id: 00000000-0000-0000-0000-000000000001,
         username: "local-user",
         providers: [{ provider: "local", display_name: "Local User" }]
       }
     }
7. /api/info response:
   → shared_api_base: null
   → login_status: { status: "loggedin", profile: {...} }
```

**Frontend:**
```
8. ConfigProvider.setRemoteApiBase(null) → _remoteApiBase stays ""
9. LocalAuthProvider: loginStatus.status === 'loggedin' → isSignedIn = true
10. isLocalMode() → !getRemoteApiUrl() → !"" → true
```

**UI flow:**
```
11. ProjectKanban renders:
    → useAuth().isSignedIn → true
    → skips <LoginRequiredPrompt> (line 305)
    → proceeds to <OrgProvider> → <ProjectKanbanInner>

12. Sidebar (SharedAppLayout):
    → isSignedIn = true
    → useShape(PROJECTS_SHAPE) enabled
    → Electric sync tries, fails (no auth token)
    → Falls back to HTTP fallback
    → makeRequest('/v1/fallback/projects?...')
    → isLocalMode() && isKanbanPath() → true
    → fetch('/api/remote/v1/fallback/projects?...')
```

**BUT — GAP FOUND:**

```
13. useUserOrganizations fires:
    → organizationsApi.getUserOrganizations()
    → makeRemoteRequest('/v1/organizations')
    → makeRequest('/v1/organizations')
    → isLocalMode() && isKanbanPath() → true
    → fetch('/api/remote/v1/organizations')
    → kanban_v1 router: NO /organizations route defined
    → 404 NOT FOUND ← BLOCKS THE UI
```

### Route Gap Analysis

**Frontend `KANBAN_PATH_PREFIXES`** vs **`kanban_v1::router()` routes**:

| Frontend calls `/v1/...` | kanban_v1 handler | Status |
|---|---|---|
| `organizations` | — | **MISSING** |
| `projects` | `projects::router()` | OK |
| `project_statuses` | `project_statuses::router()` | OK |
| `issues` | `issues::router()` | OK |
| `tags` | `tags::router()` | OK |
| `issue_assignees` | `issue_assignees::router()` | OK |
| `issue_relationships` | `issue_relationships::router()` | OK |
| `issue_tags` | `issue_tags::router()` | OK |
| `issue_comments` | — | **MISSING** |
| `workspaces` | — | **MISSING** |

### Answer: Can You Sign In?

**No real sign-in.** After removing `option_env!`:
- Remote client fails → OAuth sign-in is impossible (no server to authenticate against)
- Auth bypass auto-authenticates as "local-user" — `isSignedIn = true` — no prompt shown
- Project/kanban feature becomes accessible (no login gate)
- But org fetch fails (404) because `/v1/organizations` route is missing from kanban_v1

### Answer: Is Auth Bypass Already Implemented?

**Yes.** `crates/local-deployment/src/lib.rs:410-428` implements the full auth bypass:
```rust
let Ok(_client) = self.remote_client() else {
    let local_profile = ProfileResponse { ... };
    return LoginStatus::LoggedIn { profile: Some(local_profile) };
};
```
It has always been implemented (Task 10 in the plan). It was simply **unreachable** because `option_env!` made `remote_client()` succeed, taking the OAuth path instead.

### What's Needed to Enable Project Feature Without Sign-In

Three changes required:

1. **Remove `option_env!`** (line 178) — unblocks auth bypass
2. **Add `/v1/organizations` route to kanban_v1** — returns seeded "My Workspace" org from local SQLite
3. **Add `/v1/issue_comments` route to kanban_v1** — for comment CRUD

Optional for full functionality:
4. `/v1/workspaces` — may work via existing local workspace routes, verify

### Recommended Fix Plan

**Step 1: Fix build config** — `crates/local-deployment/src/lib.rs:178`
```rust
// Before:
let api_base = std::env::var("VK_SHARED_API_BASE")
    .ok()
    .or_else(|| option_env!("VK_SHARED_API_BASE").map(|s| s.to_string()));

// After:
let api_base = std::env::var("VK_SHARED_API_BASE").ok();
```

**Step 2: Add org route** — new file `crates/server/src/routes/kanban_v1/organizations.rs`
```rust
// GET /v1/organizations → query local SQLite organizations table
// Returns the seeded "My Workspace" org (deterministic UUID v5)
```

**Step 3: Add comment routes** — new file `crates/server/src/routes/kanban_v1/issue_comments.rs`
```rust
// CRUD routes for /v1/issue_comments (create, list, update, delete)
// DB model already exists at crates/db/src/models/issue_comment.rs
```

**Step 4: Rebuild**
```bash
pnpm run build:npx
```

---

## 8. Key File Reference

### Backend (Implemented, Largely Unused)
- `crates/db/migrations/20260426000000_local_kanban_tables.sql` — SQLite kanban schema
- `crates/db/src/models/{organization,issue,project_status,kanban_tag,issue_assignee,issue_relationship,issue_tag,issue_comment}.rs`
- `crates/server/src/routes/kanban_v1/{mod,shape_fallbacks,issues,projects,project_statuses,tags,issue_assignees,issue_relationships,issue_tags}.rs`
- `crates/server/src/routes/mod.rs:75` — `.nest("/remote/v1", kanban_v1::router())`
- `crates/local-deployment/src/lib.rs:176-178` — **THE BUG**: `option_env!` bakes cloud URL
- `crates/local-deployment/src/lib.rs:410-428` — auth bypass (works, but unreachable)
- `crates/local-deployment/src/first_boot.rs` — seed data init

### Frontend (Implemented)
- `packages/web-core/src/shared/lib/remoteApi.ts:84` — `isLocalMode()` check determines routing
- `packages/web-core/src/pages/kanban/ProjectKanban.tsx` — restored kanban UI
- `packages/local-web/src/app/providers/ConfigProvider.tsx:29` — `setRemoteApiBase()` from `/api/info`

### Infrastructure (Running)
- `npx-cli/dist/macos-arm64/vibe-kanban` — locally-built binary (PID 15156)
- `~/.vibe-kanban/bin/v0.1.44-.../vibe-kanban-mcp` — MCP server (PID 20610)
- `~/Library/Application Support/ai.bloop.vibe-kanban/credentials.json` — valid OAuth token
- `~/Library/Application Support/ai.bloop.vibe-kanban/db.v2.sqlite` — local SQLite (seed only)

---

## 9. Answers to User Questions

### Which database is being used?
**Cloud PostgreSQL** (`api.vibekanban.com`). Local SQLite exists but contains only seed data and receives zero writes during normal use.

### Which approach from feasibility report was applied?
**Option A (Local SQLite)** — fully implemented in code but **bypassed at runtime** due to build configuration.

### What is the deviation?
`option_env!("VK_SHARED_API_BASE")` bakes the cloud URL into the binary at compile time. This causes `remote_client()` to succeed, enabling remote auth and causing the frontend to route all kanban traffic to the cloud, bypassing the local SQLite implementation entirely.

### Why do projects/issues appear on multiple computers?
Because they're stored on the **cloud PostgreSQL** server at `api.vibekanban.com`. Your OAuth credentials authenticate you to the same account from any computer. This is exactly how the pre-sunset system worked — your local kanban_v1 changes are not active because the build configuration overrides them.

### How to fix?
Rebuild the binary with `VK_SHARED_API_BASE` unset, OR modify the code to remove the `option_env!` fallback and/or make frontend kanban routing independent of remote API configuration.
