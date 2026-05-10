# Task Breakdown — Local Kanban Auth & Project Fixes

**Source Plan:** [ROOT_CAUSE_ANALYSIS.md](ROOT_CAUSE_ANALYSIS.md)
**Date:** 2026-05-11
**Branch:** `analysis/enable-local-backend-and-db`

---

## Codebase Analysis Summary

### Verified Files (all exist)

| File | Lines | Key Contents |
|------|-------|-------------|
| `crates/utils/src/jwt.rs` | 44 | `extract_expiration`, `extract_subject` — read-only JWT decode only |
| `crates/server/src/routes/oauth.rs` | 502 | `get_token` (L282), `get_current_user` (L299), `auth_methods` (L94) |
| `crates/server/src/routes/kanban_v1/projects.rs` | 227 | `CreateProjectRequest` (L27-32), `create_project` handler (L93) |
| `crates/db/src/models/project.rs` | ~160 | `Project::create` — UUID via `Uuid::new_v4()` at L94 |
| `crates/server/src/routes/relay_auth/client.rs` | 71 | `list_relay_paired_hosts` (L52), `pair_relay_host` (L27) |
| `crates/local-deployment/src/lib.rs` | ~430 | Synthetic `ProfileResponse` at L408-423 (prior art) |
| `crates/utils/Cargo.toml` | 42 | `jsonwebtoken = "10.2.0"` with `rust_crypto` feature |
| `crates/utils/src/lib.rs` | 103 | `pub mod jwt;` at L12 |

### Prior Art

- **Local user UUID** `00000000-0000-0000-0000-000000000001` already used in:
  - `crates/local-deployment/src/lib.rs:409` — synthetic profile fallback
  - `crates/server/src/routes/kanban_v1/issue_comments.rs:73` — local comment author
- **Synthetic login** in `LocalDeployment::get_login_status()` (L406-424): when `remote_client()` fails, auto-creates `ProfileResponse` with local user, sets via `auth_context.set_profile()`, returns `LoginStatus::LoggedIn`
- **Pattern to follow:** same early-return-on-Err pattern used in `get_login_status()`

### Hidden Coupling

- `jwt.rs` currently only decodes; `jsonwebtoken` crate is available for encoding
- `CreateProjectRequest` has no `id` field; UUID lives entirely in the db model layer
- `relay_hosts()?` uses `?` operator — maps to `RelayHostsNotConfigured` → 400 BadRequest
- Frontend auth endpoints (`/api/auth/*`) are NOT routed through `localApiRequest` — they use `makeAuthenticatedRequest` which requires a real token

---

## Parallelization Strategy

### Batch Model

```
BATCH 1 (no deps)          BATCH 2 (depends on Batch 1)
┌─────────────────┐        ┌─────────────────────────────┐
│ Task 1: jwt.rs   │──┐     │ Task 4: oauth.rs            │
│ (synthetic token)│  │     │ (get_token, get_current_user,│
└─────────────────┘  │     │  auth_methods fallbacks)     │
                     ├────▶└─────────────────────────────┘
┌─────────────────┐  │
│ Task 2: projects │  │     Files touched per batch:
│ (optional id)    │  │     Batch 1: jwt.rs, projects.rs,
└─────────────────┘  │               project.rs (db),
                     │               relay_auth/client.rs
┌─────────────────┐  │     Batch 2: oauth.rs
│ Task 3: relay    │  │
│ (graceful hosts) │──┘
└─────────────────┘
```

**Merge order:** Batch 1 must be merged before Batch 2 starts (Task 4 depends on Task 1's `create_synthetic_token`).

### Summary Table

| # | Batch | Target File(s) | Dependencies | Can Parallel With |
|---|-------|---------------|-------------|-------------------|
| 1 | 1 | `crates/utils/src/jwt.rs` | None | Task 2, Task 3 |
| 2 | 1 | `crates/server/src/routes/kanban_v1/projects.rs`, `crates/db/src/models/project.rs` | None | Task 1, Task 3 |
| 3 | 1 | `crates/server/src/routes/relay_auth/client.rs` | None | Task 1, Task 2 |
| 4 | 2 | `crates/server/src/routes/oauth.rs` | Task 1 | None (sole Batch 2 task) |

### Developer Assignment Tracks

**Single developer:** Task 1 → Task 4 → Task 2 → Task 3 (critical path first)
**Two developers:**
- Dev A: Task 1 → Task 4 (auth critical path)
- Dev B: Task 2 → Task 3 (data + relay fixes)
**Three developers:**
- Dev A: Task 1 → Task 4
- Dev B: Task 2
- Dev C: Task 3

---

## Tasks

### Task 1: Add `create_synthetic_token` to JWT utils

**Priority:** CRITICAL
**Batch:** 1
**Dependencies:** None
**Target file:** `crates/utils/src/jwt.rs`

#### Description

Add a new public function `create_synthetic_token` that creates a minimal signed JWT for local mode. The function encodes `{"sub": "<user_id>", "iat": <now>, "exp": <now + 24h>}` using HS256.

**Current state of** [jwt.rs:1-44](crates/utils/src/jwt.rs):
- Only read-side functions exist: `extract_expiration`, `extract_subject`
- Uses `jsonwebtoken::dangerous::insecure_decode` for decoding without verification
- `jsonwebtoken` v10.2.0 with `rust_crypto` feature is already in `Cargo.toml`

**What to add:** After the existing `extract_subject` function (after line 43), add:

```rust
use jsonwebtoken::EncodingKey;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct SyntheticClaims {
    sub: String,
    exp: usize,
    iat: usize,
}

/// Create a synthetic JWT for local development mode.
/// The token is NOT cryptographically validated anywhere —
/// HS256 with a static dev secret is sufficient.
pub fn create_synthetic_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = SyntheticClaims {
        sub: user_id.to_string(),
        iat: now,
        exp: now + 86400, // 24 hours
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &EncodingKey::from_secret(b"local-dev-no-validation"),
    )
}
```

**Imports to add at top of file:**
- `use jsonwebtoken::EncodingKey;` (line 2 area, next to existing `use jsonwebtoken::dangerous::insecure_decode;`)
- `use serde::Serialize;` (line 3 area, next to existing `use serde::Deserialize;`)

#### Acceptance Criteria

- [ ] `create_synthetic_token("00000000-0000-0000-0000-000000000001")` returns `Ok(String)` containing a valid JWT
- [ ] Decoded JWT payload contains `"sub": "00000000-0000-0000-0000-000000000001"`
- [ ] Decoded JWT payload contains `"iat"` and `"exp"` fields
- [ ] `exp` is approximately `iat + 86400`
- [ ] `cargo build --workspace` compiles without errors
- [ ] `cargo test --workspace` passes

---

### Task 2: Accept optional client-generated `id` in CreateProjectRequest

**Priority:** HIGH
**Batch:** 1
**Dependencies:** None
**Target files:** `crates/server/src/routes/kanban_v1/projects.rs`, `crates/db/src/models/project.rs`

#### Description

The frontend sends `{ id: crypto.randomUUID(), ...userData }` in the insert payload ([hooks.ts:181-186](packages/web-core/src/shared/integrations/electric/hooks.ts#L181-L186)), but `CreateProjectRequest` has no `id` field and `Project::create` generates its own UUID via `Uuid::new_v4()`. This creates a UUID mismatch between client and server.

**Changes:**

**File 1:** [projects.rs:27-32](crates/server/src/routes/kanban_v1/projects.rs#L27-L32) — Add optional `id` field:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub name: String,
    pub color: String,
    pub organization_id: Uuid,
}
```

**File 1 continued:** [projects.rs:106-111](crates/server/src/routes/kanban_v1/projects.rs#L106-L111) — Pass `payload.id` to `Project::create`:

```rust
let project = Project::create(
    pool,
    payload.id,  // was: no id param, now: Option<Uuid>
    &payload.name,
    &payload.color,
    Some(payload.organization_id),
)
.await
.map_err(|e| db_error(e, "failed to create project"))?;
```

**File 2:** [project.rs:88-115](crates/db/src/models/project.rs#L88-L115) — Modify `Project::create` signature and UUID logic:

Change the function signature from:
```rust
pub async fn create(
    pool: &SqlitePool,
    name: &str,
    color: &str,
    organization_id: Option<Uuid>,
) -> Result<Self, sqlx::Error> {
    let id = Uuid::new_v4();
```

To:
```rust
pub async fn create(
    pool: &SqlitePool,
    id: Option<Uuid>,
    name: &str,
    color: &str,
    organization_id: Option<Uuid>,
) -> Result<Self, sqlx::Error> {
    let id = id.unwrap_or_else(Uuid::new_v4);
```

No other changes needed — the rest of the function uses `id` variable which is now set from the parameter.

**Check for other callers:** Search for `Project::create(` across the codebase. If other callers exist, update them to pass `None` for the new `id` parameter (preserving existing behavior).

#### Acceptance Criteria

- [ ] `CreateProjectRequest` accepts optional `id` field
- [ ] When client sends `id`, server uses that UUID for the project
- [ ] When client omits `id`, server generates UUID via `Uuid::new_v4()` (backward compatible)
- [ ] `cargo build --workspace` compiles without errors
- [ ] `cargo test --workspace` passes
- [ ] All existing callers of `Project::create` updated to pass `None` for `id`

---

### Task 3: Return empty host list from relay-auth when not configured

**Priority:** LOW
**Batch:** 1
**Dependencies:** None
**Target file:** `crates/server/src/routes/relay_auth/client.rs`

#### Description

In local mode, `deployment.relay_hosts()?` returns `Err(RelayHostsNotConfigured)`, causing `GET /api/relay-auth/client/hosts` to return 400. The frontend calls this unconditionally and the error clutters the console.

**Current code** at [client.rs:52-58](crates/server/src/routes/relay_auth/client.rs#L52-L58):

```rust
pub async fn list_relay_paired_hosts(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ListRelayPairedHostsResponse>>, ApiError> {
    let hosts = deployment.relay_hosts()?.list_hosts().await;
    Ok(ResponseJson(ApiResponse::success(
        ListRelayPairedHostsResponse { hosts },
    )))
}
```

**Change to:**

```rust
pub async fn list_relay_paired_hosts(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ListRelayPairedHostsResponse>>, ApiError> {
    let hosts = match deployment.relay_hosts() {
        Ok(relay) => relay.list_hosts().await,
        Err(_) => vec![],
    };
    Ok(ResponseJson(ApiResponse::success(
        ListRelayPairedHostsResponse { hosts },
    )))
}
```

This returns `{"success": true, "data": {"hosts": []}}` instead of 400 when relay is not configured.

#### Acceptance Criteria

- [ ] `GET /api/relay-auth/client/hosts` returns `{"hosts": []}` in local mode (not 400)
- [ ] `GET /api/relay-auth/client/hosts` still returns real hosts when relay IS configured
- [ ] `cargo build --workspace` compiles without errors
- [ ] `cargo test --workspace` passes

---

### Task 4: Add local-mode fallback to oauth endpoints

**Priority:** CRITICAL
**Batch:** 2
**Dependencies:** Task 1 (`create_synthetic_token` must exist)
**Target file:** `crates/server/src/routes/oauth.rs`

#### Description

Three oauth handlers fail in local mode because they all call `deployment.remote_client()?` which returns `Err(RemoteClientNotConfigured)`. Add early-return fallbacks that detect local mode and return synthetic/local data instead.

The pattern to follow is from `LocalDeployment::get_login_status()` at [local-deployment/src/lib.rs:406-424](crates/local-deployment/src/lib.rs#L406-L424) — match on `deployment.remote_client()` and handle the `Err` arm with synthetic data.

#### Change 4a: `get_token` (line 282-297)

Replace the existing function body. Current:

```rust
async fn get_token(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TokenResponse>>, ApiError> {
    let remote_client = deployment.remote_client()?;

    // This will auto-refresh the token if expired
    let access_token = remote_client.access_token().await.map_err(ApiError::from)?;

    let creds = deployment.auth_context().get_credentials().await;
    let expires_at = creds.and_then(|c| c.expires_at);

    Ok(ResponseJson(ApiResponse::success(TokenResponse {
        access_token,
        expires_at,
    })))
}
```

Replace with:

```rust
async fn get_token(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TokenResponse>>, ApiError> {
    // Local mode: return synthetic token for hardcoded local user
    let remote_client = match deployment.remote_client() {
        Ok(c) => c,
        Err(_) => {
            let token = utils::jwt::create_synthetic_token(
                "00000000-0000-0000-0000-000000000001"
            ).map_err(|e| ApiError::InternalServerError(e.to_string()))?;
            return Ok(ResponseJson(ApiResponse::success(TokenResponse {
                access_token: token,
                expires_at: None,
            })));
        }
    };

    // Existing remote path (unchanged)
    let access_token = remote_client.access_token().await.map_err(ApiError::from)?;
    let creds = deployment.auth_context().get_credentials().await;
    let expires_at = creds.and_then(|c| c.expires_at);
    Ok(ResponseJson(ApiResponse::success(TokenResponse {
        access_token,
        expires_at,
    })))
}
```

#### Change 4b: `get_current_user` (line 299-318)

Replace the existing function body:

```rust
async fn get_current_user(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CurrentUserResponse>>, ApiError> {
    // Local mode: return hardcoded local user
    let remote_client = match deployment.remote_client() {
        Ok(c) => c,
        Err(_) => {
            return Ok(ResponseJson(ApiResponse::success(CurrentUserResponse {
                user_id: "00000000-0000-0000-0000-000000000001".to_string(),
            })));
        }
    };

    // Existing remote path (unchanged)
    let access_token = remote_client.access_token().await.map_err(ApiError::from)?;
    let user_id = utils::jwt::extract_subject(&access_token)
        .map_err(|e| {
            tracing::error!("Failed to extract user ID from token: {}", e);
            ApiError::Unauthorized
        })?
        .to_string();
    Ok(ResponseJson(ApiResponse::success(CurrentUserResponse { user_id })))
}
```

#### Change 4c: `auth_methods` (line 94-100)

Replace the existing function body:

```rust
async fn auth_methods(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<AuthMethodsResponse>>, ApiError> {
    // Local mode: return empty providers list
    let client = match deployment.remote_client() {
        Ok(c) => c,
        Err(_) => {
            return Ok(ResponseJson(ApiResponse::success(AuthMethodsResponse {
                local_auth_enabled: true,
                oauth_providers: vec![],
            })));
        }
    };
    let methods = client.auth_methods().await?;
    Ok(ResponseJson(ApiResponse::success(methods)))
}

#### Acceptance Criteria

- [ ] `GET /api/auth/token` returns `{"success":true,"data":{"access_token":"...","expires_at":null}}` in local mode
- [ ] `GET /api/auth/user` returns `{"success":true,"data":{"user_id":"00000000-0000-0000-0000-000000000001"}}` in local mode
- [ ] `GET /api/auth/methods` returns `{"success":true,"data":{"local_auth_enabled":true,"oauth_providers":[]}}` in local mode
- [ ] All three endpoints still work correctly in remote/cloud mode (remote_client available)
- [ ] `cargo build --workspace` compiles without errors
- [ ] `cargo test --workspace` passes
- [ ] Browser console: no `GET /api/auth/token 400` errors
- [ ] Browser console: no `"Not authenticated"` errors
- [ ] Settings → Projects loads without "Not Authorized"

---

## Verification Checklist (Post-Implementation)

Run after all 4 tasks are merged:

- [ ] `curl http://127.0.0.1:41615/api/auth/token` → `{"success":true,"data":{"access_token":"...","expires_at":null}}`
- [ ] `curl http://127.0.0.1:41615/api/auth/user` → local user profile
- [ ] `curl http://127.0.0.1:41615/api/auth/methods` → `{"local_auth_enabled":true,"oauth_providers":[]}`
- [ ] `curl http://127.0.0.1:41615/api/relay-auth/client/hosts` → `{"hosts":[]}`
- [ ] `curl -X POST http://127.0.0.1:41615/api/remote/v1/projects -H 'Content-Type: application/json' -d '{"id":"<uuid>","name":"test","color":"200 80% 50%","organization_id":"<org-id>"}'` → project created with matching UUID
- [ ] Browser: no "Not authenticated" errors in console
- [ ] Browser: Settings → Projects loads without "Not Authorized"
- [ ] Browser: "+" icon creates project successfully
- [ ] Browser: New project appears in sidebar immediately
- [ ] `pnpm run check` passes
- [ ] `pnpm run lint` passes
- [ ] `cargo test --workspace` passes
