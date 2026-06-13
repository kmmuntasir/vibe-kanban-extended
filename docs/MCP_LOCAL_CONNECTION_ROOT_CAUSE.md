# Root Cause Analysis — Claude / vibe-kanban MCP Cannot Connect Locally

**Date:** 2026-06-13
**Branch:** `analysis/enable-local-backend-and-db`
**Scope:** diff `origin/main...HEAD` (149 files, +14,829 / −348)
**Goal:** explain why the vibe-kanban MCP server fails when the app is run locally (local backend + local DB, no cloud / `VK_SHARED_API_BASE`).

---

## TL;DR

The local-mode re-enablement fixed the **web UI** by adding new local, DB-backed routes under
`/api/remote/v1/*` (crate `kanban_v1`) and `/v1/fallback/*`, plus a synthetic-token auth path.
But the **MCP server is a separate Rust binary** (`crates/mcp`, the `vibe-kanban-mcp` executable)
that was **not touched in this diff at all**. That binary still calls the **original cloud-proxy
routes at `/api/remote/*`** (no `v1`), and every one of those handlers does
`deployment.remote_client()?` — which **always errors in local mode** (`RemoteClientNotConfigured`).

So: web app works locally, MCP does not. The MCP either fails to establish its connection (no
backend reachable / port file missing / orchestrator init bail), or establishes it but then every
kanban/project tool call returns HTTP **400 "Remote client not configured"**.

```
Frontend (fixed)   →  /api/remote/v1/*   (kanban_v1)  →  local DB   ✅
MCP binary (NOT fixed)  →  /api/remote/*   (remote)    →  remote_client()?  ❌ 400
```

---

## How the MCP Connects (the chain)

1. **Spawn (stdio).** Claude Code (or a vibe-kanban-launched executor) starts the MCP server as a
   child process over stdio. Default wiring (`crates/executors/default_mcp.json`):
   ```json
   "vibe_kanban": { "command": "npx", "args": ["-y", "vibe-kanban@latest", "--mcp"] }
   ```
   `npx-cli/src/cli.ts:216` (`runMcp`) → `extractAndRun("vibe-kanban-mcp")` → spawns the binary with
   `["--mode", "global"]` and `stdio: "inherit"` (`cli.ts:217-220`).

2. **Resolve backend URL.** `crates/mcp/src/bin/vibe_kanban_mcp.rs:100` (`resolve_base_url`), in
   priority order:
   1. `VIBE_BACKEND_URL`
   2. `MCP_HOST`/`HOST` + `MCP_PORT`/`BACKEND_PORT`/`PORT`
   3. **Port file** at `$TMPDIR/vibe-kanban/vibe-kanban.port`
      (`crates/utils/src/port_file.rs:31`, written by `crates/server/src/main.rs:123`).

3. **Startup context fetch.** `McpServer::init()` → `fetch_context_at_startup()`
   (`crates/mcp/src/task_server/mod.rs:107`) → `GET /api/containers/attempt-context?container_ref=<cwd>`.
   This endpoint reads the **local DB directly** (`crates/server/src/routes/containers.rs:40`,
   `Workspace::resolve_container_ref_by_prefix` + `load_context`), so it works in local mode and
   carries **no auth requirement**.

4. **Serve.** `server.init().await?.serve(stdio()).await` (`vibe_kanban_mcp.rs:42`).

5. **Tool calls.** On demand, tools issue HTTP requests to the backend with a **bare `reqwest::Client`
   — no `Authorization` header, no token fetch anywhere in `crates/mcp`**
   (`grep -rn Authorization crates/mcp` → empty). They target `/api/remote/*`, `/api/workspaces/*`,
   `/api/sessions`, `/api/organizations`, `/api/repos`, `/api/tags`.

---

## Evidence: MCP targets cloud-proxy `/api/remote/*`

Hardcoded paths the MCP binary calls (all live under `/api/remote`):

| MCP tool file | Path called |
|---|---|
| `tools/remote_issues.rs` | `/api/remote/issues`, `/api/remote/issues/{id}`, `/api/remote/issues/search`, `/api/remote/pull-requests`, `/api/remote/tags`, `/api/remote/issue-tags`, `/api/remote/issue-relationships` |
| `tools/remote_projects.rs` | `/api/remote/projects` |
| `tools/issue_assignees.rs` | `/api/remote/issue-assignees`, `…/{id}` |
| `tools/issue_relationships.rs` | `/api/remote/issue-relationships`, `…/{id}` |
| `tools/issue_tags.rs` | `/api/remote/issue-tags`, `…/{id}` |
| `tools/mod.rs` | `/api/remote/project-statuses?project_id=`, `/api/remote/issues/{id}` |
| `task_server/mod.rs:155` (`fetch_remote_workspace_context`) | `/api/remote/workspaces/by-local-id/{id}` (→ also `fetch_remote_organization_id` → `/api/remote/...`) |

Server routing (`crates/server/src/routes/mod.rs`):
```
58:  .nest("/remote",       remote::router())     // ← ORIGINAL cloud proxy (MCP hits this)
75:  .nest("/remote/v1",    kanban_v1::router())   // ← user's NEW local DB routes (frontend hits this)
85:  .nest("/api", api_routes)
```

Every handler in `crates/server/src/routes/remote/*.rs` begins with
`let client = deployment.remote_client()?;`. Examples:
- `remote/issues.rs:30,39,48,57,67,76` (list/search/get/create/update/delete)
- `remote/workspaces.rs:24`
- (same pattern in `projects.rs`, `tags.rs`, `issue_tags.rs`, `issue_assignees.rs`, …)

`LocalDeployment::remote_client()` returns `Err(RemoteClientNotConfigured)` whenever
`VK_SHARED_API_BASE` is unset (`crates/local-deployment/src/lib.rs:198,203`, gated by the
`VK_SHARED_API_BASE` read at `:176`). That error maps to **HTTP 400 "Remote client not
configured"** (`crates/server/src/error.rs:100-104`).

This is an **exact match** to the captured runtime logs:

`revert-sunsetting-analysis/enabled-local-run/backend.log`:
```
INFO local_deployment: VK_SHARED_API_BASE not set; remote features disabled
INFO server: No PORT environment variable set, using port 0 for auto-assignment
INFO server: Main server on :43423, Preview proxy on :34267
```

`revert-sunsetting-analysis/enabled-local-run/frontend.log`:
```
GET http://127.0.0.1:43423/api/auth/token            400 (Bad Request)  "Remote client not configured"
GET http://127.0.0.1:43423/api/relay-auth/client/hosts 400              "Remote relay API is not configured"
Error: Not authenticated  (React Query, repeated)
```

The MCP binary issues the same class of failing requests against `/api/remote/*`.

---

## Ranked Root Causes

### #1 — CRITICAL (most likely, ~80%): MCP binary calls cloud-only `/api/remote/*`; the local fix added `/api/remote/v1/*` instead

- `git diff --name-only origin/main...HEAD -- crates/mcp crates/executors` → **empty**. The MCP
  crate and its hardcoded `/api/remote/*` paths were never repointed to the new local routes.
- The local re-enablement added **`/api/remote/v1` → `kanban_v1::router()`**, whose handlers read
  the local DB directly (`crates/server/src/routes/kanban_v1/issues.rs` uses `db::models::issue`,
  **no `remote_client()` call**). The web frontend (`packages/web-core/src/shared/lib/remoteApi.ts`)
  was repointed onto these. The MCP was not.
- Consequence: in **global mode**, the stdio connection typically *establishes* (init only needs
  `attempt-context`, which is local-DB), so Claude Code shows the server as "connected" — but **every
  kanban/project/list tool call returns 400 "Remote client not configured"**, which reads to the user
  as "MCP can't connect / is broken". `get_context` is also degraded: `fetch_remote_workspace_context`
  hits `/api/remote/workspaces/by-local-id/{id}` → 400 → returns `None`.

**Fix direction:** repoint the MCP binary at the local routes. Either
(a) change the MCP's URL builder / paths from `/api/remote/*` to `/api/remote/v1/*` **and** make the
`kanban_v1` routes serve the shapes the MCP expects (the MCP uses a different response envelope and
paths like `/api/remote/issues/search` (POST) and `/api/remote/workspaces/by-local-id/{id}` that
`kanban_v1` does not currently expose), or
(b) add **local-mode fallbacks inside the existing `/api/remote/*` handlers** (same pattern used for
`/api/auth/token` in `oauth.rs`): when `remote_client()` is `Err`, fall back to the local DB query.
Option (b) is the smallest-blast-radius fix because it requires **no MCP binary change** — the
existing MCP keeps calling `/api/remote/*`, and the server transparently serves local data.

### #2 — HIGH (~60% inside vibe-kanban-launched workspaces): orchestrator-mode init bails when no workspace context resolves

- `fetch_context_at_startup` (`mod.rs:107-120`): in **orchestrator** mode,
  `Ok(None) | Err` → `anyhow::bail!("Failed to load orchestrator MCP context from /api/containers/attempt-context")`.
  The binary exits **before** `serve(stdio())` → Claude Code reports the MCP failed to start / connect.
- `attempt-context` returns `None` when the current working dir is **not** a registered workspace
  prefix (`Workspace::resolve_container_ref_by_prefix` → `RowNotFound` → `Ok(None)` at
  `containers.rs:46-49`) **or** when the backend is unreachable at the resolved `base_url`.
- `global` mode tolerates `None` (`mod.rs:113`) so it does not bail — this path bites primarily when
  the MCP is launched with `--mode orchestrator` (task-attempt / workspace-run context).

### #3 — MEDIUM (~40%): backend URL resolution depends on a port file that the local workspace never injects via env

- Local backend binds port `0` (auto-assigned) and writes `$TMPDIR/vibe-kanban/vibe-kanban.port`
  (`main.rs:96-123`).
- The local-deployment workspace launcher injects **only** `VK_WORKSPACE_ID` and
  `VK_WORKSPACE_BRANCH` into the executor env (`crates/local-deployment/src/container.rs:1366-1367`).
  It does **not** set `BACKEND_PORT`, `MCP_PORT`, `MCP_HOST`, or `VIBE_BACKEND_URL`.
- Therefore the MCP (spawned by Claude *inside* that workspace, inheriting that env) can only find the
  backend through the **port file**. Failure modes:
  - Backend not running at the time Claude spawns the MCP → no port file → `resolve_base_url` errors
    → binary exits → genuine "cannot connect".
  - Stale port file from a previous run on a different auto-assigned port → MCP talks to a dead port.
  - `$TMPDIR` differs between the process that wrote the file and the MCP child (e.g. systemd vs user
    shell, snap/flatpak sandbox) → file not found.
- Independently, the default MCP template pulls `npx -y vibe-kanban@latest` from **npm**, i.e. the
  *published* binary, which then reads the *local* port file — version skew between the local backend
  and the published MCP binary is a secondary risk here.

### #4 — MEDIUM (~30%): published `vibe-kanban@latest` MCP vs locally-built backend skew

- `default_mcp.json:1-9` registers the MCP as `npx -y vibe-kanban@latest --mcp`. That downloads the
  published package. If the user's local backend API shape has diverged from the published binary's
  expectations, even correctly-resolved calls can 404/mismatch. Lower likelihood than #1 because the
  *same* `/api/remote/*` failure would dominate, but it amplifies debugging confusion.

### #5 — LOW (~15%): MCP uses no auth token; benign in local mode but worth noting

- The MCP client never sends an `Authorization` header. Local `kanban_v1` routes have no auth
  middleware, so this is **not** a blocker by itself. It only matters if a future change adds auth to
  the local routes; the MCP would then need the same synthetic-token path the web app already uses
  (`oauth.rs` local-mode `get_token` → `utils::jwt::create_synthetic_token`).

---

## Why the existing fix missed the MCP

| Layer | What was done | Reaches cloud? | Works locally? |
|---|---|---|---|
| Frontend `remoteApi.ts` | Repointed to `/api/remote/v1/*` + `/v1/fallback/*` | No | ✅ |
| `kanban_v1` routes (`/api/remote/v1`) | New local-DB handlers + `shape_fallbacks.rs` | No | ✅ |
| Auth (`oauth.rs`, `jwt.rs`) | Synthetic token + local-mode `get_token`/`get_current_user` | No | ✅ |
| **MCP binary (`crates/mcp`)** | **Nothing — untouched** | **Yes (`/api/remote/*`)** | **❌** |

The fix concentrated on the **HTTP path the browser uses**. The MCP is a **second, independent
client** of the same backend with its **own hardcoded paths**, and it was never repointed. The
existing report (`revert-sunsetting-analysis/enabled-local-run/ROOT_CAUSE_ANALYSIS.md`) correctly
diagnosed the `/api/auth/*` + Electric shape failures for the **web app**, but did not analyse the
MCP client — which is why the MCP still fails after those fixes landed.

---

## Recommended Fixes (ranked)

### Fix A (smallest blast radius) — local-mode fallbacks inside `/api/remote/*` handlers
Keep the MCP binary unchanged. In each `crates/server/src/routes/remote/*.rs` handler, follow the
same pattern already used in `oauth.rs::get_token`:

```rust
async fn list_issues(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<ResponseJson<ApiResponse<ListIssuesResponse>>, ApiError> {
    match deployment.remote_client() {
        Ok(client) => {
            let response = client.list_issues(query.project_id).await?;
            Ok(ResponseJson(ApiResponse::success(response)))
        }
        Err(_) => {
            // LOCAL MODE: serve from the local DB (reuse kanban_v1 logic)
            let response = kanban_v1::issues::list_issues_local(&deployment.db().pool, query).await?;
            Ok(ResponseJson(ApiResponse::success(response)))
        }
    }
}
```
- Pros: one place to change (server); MCP, VSCode extension, and any other `/api/remote/*` client all
  start working in local mode for free.
- Cons: must replicate/extract the local query logic from `kanban_v1` (or call into it) and match the
  response envelope (`ListIssuesResponse`, `MutationResponse`, etc.) the `remote` handlers already
  return. Several handlers need it: `issues`, `projects`, `tags`, `issue_tags`, `issue_assignees`,
  `issue_relationships`, `project_statuses`, `pull_requests`, `workspaces`.

### Fix B (correct long-term) — repoint the MCP binary at the local routes
Update `crates/mcp/src/task_server/tools/*` paths from `/api/remote/*` → `/api/remote/v1/*`, **and**
add the missing endpoints to `kanban_v1` that the MCP needs but the frontend does not:
- `POST /api/remote/v1/issues/search`
- `GET /api/remote/v1/workspaces/by-local-id/{id}` (used by `fetch_remote_workspace_context`)
- `GET /api/remote/v1/issue-assignees`, `/issue-relationships`, `/issue-tags`, `/project-statuses`,
  `/pull-requests` as needed by the tool set.
Also unify response envelopes (`ApiResponse` vs `ApiResponseEnvelope`).

### Fix C (robustness) — eliminate port-file fragility for the MCP
In `crates/local-deployment/src/container.rs` (next to `VK_WORKSPACE_ID`), also inject the resolved
backend port so the MCP never depends on the port file:
```rust
env.insert("VIBE_BACKEND_URL", &format!("http://127.0.0.1:{}", actual_main_port));
// or: env.insert("MCP_PORT", &actual_main_port.to_string());
```
This requires the container to know `actual_main_port` (it can read the same port file or receive it
at construction). Removes failure modes #3 (stale/missing port file, `$TMPDIR` skew).

### Fix D (clarity) — run the locally-built MCP, not `npx @latest`
For local dev, register the MCP against the locally-built binary (e.g.
`target/release/vibe-kanban-mcp --mode global` with `VIBE_BACKEND_URL` set) instead of
`npx -y vibe-kanban@latest --mcp`, removing the published-vs-local skew (#4).

---

## Verification Plan

After applying Fix A/B, with the local backend running:

1. **Connection establishes**
   ```bash
   VIBE_BACKEND_URL=http://127.0.0.1:<port> ./target/release/vibe-kanban-mcp --mode global
   # must not exit; logs "Using backend URL: http://127.0.0.1:<port>"
   ```
2. **`attempt-context` resolves** (init):
   ```bash
   curl -s "http://127.0.0.1:<port>/api/containers/attempt-context?container_ref=<workspace-cwd>"
   # expect 200 + WorkspaceContext JSON, not 400
   ```
3. **MCP tool endpoints no longer 400:**
   ```bash
   curl -s "http://127.0.0.1:<port>/api/remote/issues?project_id=<id>"            # expect 200 + issues
   curl -s "http://127.0.0.1:<port>/api/remote/projects?organization_id=<id>"     # expect 200
   curl -s "http://127.0.0.1:<port>/api/remote/workspaces/by-local-id/<ws-id>"    # expect 200
   ```
   Before the fix each returns `{"success":false,... "Remote client not configured"}` (400).
4. **End-to-end:** in Claude Code (or a vibe-kanban workspace), call `mcp__vibe_kanban__list_issues`
   (or equivalent) — expect real local data, not a 400.
5. **Regression guard:** with `VK_SHARED_API_BASE` set (cloud mode), the same endpoints must still
   proxy to the remote client (Fix A's `Ok` branch untouched).

---

## File Index (evidence)

| Concern | Location |
|---|---|
| MCP entrypoint / base URL resolve | `crates/mcp/src/bin/vibe_kanban_mcp.rs:34,100` |
| MCP startup context fetch (bail in orchestrator) | `crates/mcp/src/task_server/mod.rs:107,115` |
| MCP remote-context enrichment (cloud) | `crates/mcp/src/task_server/mod.rs:152,155` |
| MCP tool paths (`/api/remote/*`) | `crates/mcp/src/task_server/tools/remote_issues.rs`, `remote_projects.rs`, `issue_assignees.rs`, `issue_relationships.rs`, `issue_tags.rs`, `tools/mod.rs` |
| Cloud-proxy handlers (`remote_client()?`) | `crates/server/src/routes/remote/{issues,projects,tags,issue_tags,issue_assignees,issue_relationships,project_statuses,workspaces}.rs` |
| New local routes (`/api/remote/v1`, local DB) | `crates/server/src/routes/kanban_v1/*.rs` |
| Route nesting | `crates/server/src/routes/mod.rs:58,75,85` |
| Local `remote_client()` = `Err` when no `VK_SHARED_API_BASE` | `crates/local-deployment/src/lib.rs:176,198,203` |
| Error → HTTP 400 mapping | `crates/server/src/error.rs:100-104` |
| Port-file write / read | `crates/server/src/main.rs:123`, `crates/utils/src/port_file.rs:31` |
| Workspace env does **not** inject backend port | `crates/local-deployment/src/container.rs:1366` |
| Default MCP registration (`npx @latest`) | `crates/executors/default_mcp.json:1-9` |
| npx MCP launcher | `npx-cli/src/cli.ts:216-231` |
| Existing web-app root-cause doc | `revert-sunsetting-analysis/enabled-local-run/ROOT_CAUSE_ANALYSIS.md` |
| Captured runtime logs | `revert-sunsetting-analysis/enabled-local-run/{backend,frontend}.log` |
