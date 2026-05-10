# Root Cause Analysis — Local Kanban Auth & Project Failures

**Date:** 2026-05-11
**Branch:** `analysis/enable-local-backend-and-db`
**App URL:** `http://127.0.0.1:41615/`

---

## Symptoms Observed

1. **"Not Authorized"** in Settings → Projects menu
2. **Cannot create project** from left sidebar "+" icon
3. Browser console: `GET /api/auth/token` → 400 `"Remote client not configured"`
4. Browser console: `GET /api/relay-auth/client/hosts` → 400 `"Remote relay API is not configured"`
5. Frontend: `Error: Not authenticated` thrown repeatedly
6. TokenManager enters infinite recovery retry loop

---

## Root Cause #1 (CRITICAL): `/api/auth/token` fails in local mode

### The endpoint

**File:** [crates/server/src/routes/oauth.rs:282-297](crates/server/src/routes/oauth.rs#L282-L297)

```rust
async fn get_token(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TokenResponse>>, ApiError> {
    let remote_client = deployment.remote_client()?;  // ← FAILS in local mode
    let access_token = remote_client.access_token().await.map_err(ApiError::from)?;
    // ...
}
```

`deployment.remote_client()?` returns `Err(RemoteClientNotConfigured)` when `VK_SHARED_API_BASE` is not set. This is mapped to HTTP 400 `"Remote client not configured"` via [crates/server/src/error.rs:100-104](crates/server/src/error.rs#L100-L104).

### Why the plan missed this

The [LOCAL_KANBAN_COMPLETE_FIX_PLAN.md](../LOCAL_KANBAN_COMPLETE_FIX_PLAN.md) addressed:
- `option_env!` removal (Step 1)
- build.rs cleanup (Step 2)
- `/v1/organizations` route (Step 3)
- `/v1/issue_comments` routes (Step 4)

But it did **NOT** address any of the `/api/auth/*` endpoints that also call `deployment.remote_client()?`:
- `get_token` (line 285)
- `get_current_user` (line 302)
- `auth_methods` (line 97)
- `handoff_init` (line 118)
- `handoff_complete` (line 188)
- `local_login` (line 219)

### Cascade failure chain

```
/api/auth/token → 400
  ↓
tokenManager.getToken() → catch → set remote_auth_degraded → return null
  ↓
Shape Authorization header → no token → isPaused = true
  ↓
Electric sync aborted → 3s timeout → switch to fallback
  ↓
Fallback HTTP works (localApiRequest for /v1/fallback/*)
  ↓
BUT tokenManager recovery loop keeps retrying /api/auth/token every 5-60s
  ↓
Frontend shows "Not authenticated" errors continuously
```

### The `isPaused` self-block

**File:** [packages/web-core/src/shared/lib/electric/collections.ts:345-358](packages/web-core/src/shared/lib/electric/collections.ts#L345-L358)

```typescript
Authorization: async () => {
    const token = await authRuntime.getToken();
    if (!token) {
        isPaused = true;    // ← Blocks ALL shape fetch requests
        return '';
    }
    return `Bearer ${token}`;
},
```

When `isPaused = true`, the fetch client at line 305-309 throws `AbortError` for all subsequent requests. This permanently blocks the Electric sync stream for that shape. Only the fallback mechanism (which doesn't go through this fetch client) can deliver data.

---

## Root Cause #2: `/api/auth/user` also fails in local mode

**File:** [crates/server/src/routes/oauth.rs:299-309](crates/server/src/routes/oauth.rs#L299-L309)

```rust
async fn get_current_user(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CurrentUserResponse>>, ApiError> {
    let remote_client = deployment.remote_client()?;
    let access_token = remote_client.access_token().await.map_err(ApiError::from)?;
    let user_id = utils::jwt::extract_subject(&access_token)...;
    // ...
}
```

Same pattern — fails in local mode. The frontend's `AuthRuntime.configureAuthRuntime` in [Bootstrap.tsx](packages/local-web/src/app/entry/Bootstrap.tsx#L80-L85) wires `getCurrentUser` → `oauthApi.getCurrentUser()` → `GET /api/auth/user`.

---

## Root Cause #3: `/api/relay-auth/client/hosts` returns 400

**File:** [crates/server/src/routes/relay_auth/client.rs:32](crates/server/src/routes/relay_auth/client.rs#L32)

The `list_hosts` handler calls `deployment.relay_hosts()?` which returns `Err(RelayHostsNotConfigured)` in local mode. This is expected (relay pairing is a cloud feature), but the frontend still makes the call unconditionally.

---

## Root Cause #4: Shape mutation `onInsert` may fail with UUID mismatch

**File:** [crates/server/src/routes/kanban_v1/projects.rs:27-32](crates/server/src/routes/kanban_v1/projects.rs#L27-L32)

```rust
pub struct CreateProjectRequest {
    pub name: String,
    pub color: String,
    pub organization_id: Uuid,
    // NOTE: no `id` field
}
```

The frontend's `insert()` in [hooks.ts:181-186](packages/web-core/src/shared/integrations/electric/hooks.ts#L181-L186) sends `{ id: crypto.randomUUID(), ...userData }`. The backend generates its own UUID (line 94 of [project.rs](crates/db/src/models/project.rs#L94): `let id = Uuid::new_v4();`). This means:

1. The server creates the project with UUID-X (server-generated)
2. The frontend optimistic record has UUID-Y (client-generated)
3. The `persisted` promise at [hooks.ts:197-199](packages/web-core/src/shared/integrations/electric/hooks.ts#L197-L199) searches for UUID-Y in synced items → not found → returns client data
4. Next fallback refresh fetches project with UUID-X → frontend now has TWO copies (UUID-Y from optimistic insert + UUID-X from server)

**Severity:** Medium. Project IS created on the server, but the UI may show inconsistent state until a full page refresh.

---

## Root Cause #5: `CreateProjectRequest` silently ignores unknown `id` field

Since `CreateProjectRequest` does NOT have `#[serde(deny_unknown_fields)]`, the extra `id` field in the JSON body is silently dropped. This is safe in terms of not causing a 400 error, but the UUID mismatch (Root Cause #4) still applies.

---

## What DOES work correctly

| Feature | Status | Path |
|---------|--------|------|
| Dashboard renders without login | Working | `/api/info` → `login_status: "loggedin"` |
| Kanban board (existing issues) | Working | Fallback shapes → `/api/remote/v1/fallback/*` |
| Organization listing | Working | `/api/remote/v1/organizations` |
| Issue comments CRUD | Working | `/api/remote/v1/issue_comments/*` |
| Project listing (fallback) | Working | `/api/remote/v1/fallback/projects?organization_id=...` |
| Issue CRUD | Working | `/api/remote/v1/issues/*` |

These work because `makeRequest()` in [remoteApi.ts:80-89](packages/web-core/src/shared/lib/remoteApi.ts#L80-L89) detects local mode and routes kanban paths through `localApiRequest()` which does raw `fetch()` without auth.

---

## What does NOT work

| Feature | Status | Root Cause |
|---------|--------|-----------|
| `/api/auth/token` | **400** | `remote_client()?` fails |
| `/api/auth/user` | **400** | `remote_client()?` fails |
| `/api/auth/methods` | **400** | `remote_client()?` fails |
| `/api/relay-auth/client/hosts` | **400** | `relay_hosts()?` fails |
| Electric shape sync stream | **Paused** | No token → `isPaused = true` |
| Project creation | **Partial** | UUID mismatch (see #4) |
| Settings → Projects | **"Not Authorized"** | Auth gate blocks when shapes are degraded |

---

## Required Fixes

### Fix 1 (CRITICAL): Add local-mode fallback to `/api/auth/token`

**File:** `crates/server/src/routes/oauth.rs` — `get_token()` handler (line 282)

In local mode (no remote client), return a synthetic JWT token containing the local user UUID. The backend does NOT validate this token anywhere (no auth middleware on kanban_v1 routes), so any valid-looking JWT works.

```rust
async fn get_token(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TokenResponse>>, ApiError> {
    // In local mode, return a synthetic token for the local user
    let remote_client = match deployment.remote_client() {
        Ok(client) => client,
        Err(_) => {
            // Local mode: return synthetic token
            let local_user_id = "00000000-0000-0000-0000-000000000001";
            let token = utils::jwt::create_synthetic_token(local_user_id)
                .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
            return Ok(ResponseJson(ApiResponse::success(TokenResponse {
                access_token: token,
                expires_at: None, // never expires in local mode
            })));
        }
    };
    // ... existing remote path
}
```

**Requires:** A `create_synthetic_token` utility in `crates/utils/src/jwt.rs` that creates a minimal valid JWT with `sub` = local user UUID.

### Fix 2 (HIGH): Add local-mode fallback to `/api/auth/user`

**File:** `crates/server/src/routes/oauth.rs` — `get_current_user()` handler (line 299)

Same pattern — return the local user profile when no remote client.

### Fix 3 (MEDIUM): Accept optional `id` in `CreateProjectRequest`

**File:** `crates/server/src/routes/kanban_v1/projects.rs:27-32`

Add an optional `id` field and use it if provided:

```rust
pub struct CreateProjectRequest {
    pub id: Option<Uuid>,  // ← ADD THIS
    pub name: String,
    pub color: String,
    pub organization_id: Uuid,
}
```

Then use `payload.id.unwrap_or_else(Uuid::new_v4)` when creating the project. This ensures the frontend's optimistic UUID matches the server's record.

### Fix 4 (LOW): Handle `/api/relay-auth/client/hosts` gracefully

**File:** `crates/server/src/routes/relay_auth/client.rs`

Return an empty host list instead of 400 when relay is not configured. Change `relay_hosts()?` to handle the `Err` case.

### Fix 5 (LOW): `/api/auth/methods` — return empty providers list

**File:** `crates/server/src/routes/oauth.rs` — `auth_methods()` handler (line 94)

Return an empty `AuthMethodsResponse` when no remote client, instead of 400.

---

## Files to Change

| # | File | Change | Priority |
|---|------|--------|----------|
| 1 | `crates/server/src/routes/oauth.rs` | Local-mode fallback for `get_token`, `get_current_user`, `auth_methods` | **CRITICAL** |
| 2 | `crates/utils/src/jwt.rs` | Add `create_synthetic_token()` utility | **CRITICAL** |
| 3 | `crates/server/src/routes/kanban_v1/projects.rs` | Add optional `id` to `CreateProjectRequest` | HIGH |
| 4 | `crates/server/src/routes/relay_auth/client.rs` | Graceful empty response for `list_hosts` | LOW |

---

## JWT Utility State

**File:** [crates/utils/src/jwt.rs](crates/utils/src/jwt.rs) — **1.4K, exists**

Current functions (decode only):
- `extract_expiration(token) → Result<DateTime<Utc>>`
- `extract_subject(token) → Result<Uuid>`

Missing: `create_synthetic_token(user_id)` — needs to be added for Fix 1. Uses `jsonwebtoken` crate (already in dependencies). Will create unsigned JWT with `{"sub": "..."}` payload via `jsonwebtoken::encode`.

---

## Exact Code Changes Required

### Change 1: Add `create_synthetic_token` to `crates/utils/src/jwt.rs`

```rust
use jsonwebtoken::EncodingKey;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct SyntheticClaims {
    sub: String,
    exp: usize,
    iat: usize,
}

/// Create a synthetic JWT for local mode. Not cryptographically signed —
/// using HS256 with a zero-length key since the token is never validated.
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

### Change 2: Modify `get_token()` in `crates/server/src/routes/oauth.rs:282`

```rust
async fn get_token(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TokenResponse>>, ApiError> {
    // Local mode: return synthetic token for the hardcoded local user
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
    // ...existing remote path unchanged...
    let access_token = remote_client.access_token().await.map_err(ApiError::from)?;
    let creds = deployment.auth_context().get_credentials().await;
    let expires_at = creds.and_then(|c| c.expires_at);
    Ok(ResponseJson(ApiResponse::success(TokenResponse {
        access_token, expires_at,
    })))
}
```

### Change 3: Modify `get_current_user()` in `crates/server/src/routes/oauth.rs:299`

```rust
async fn get_current_user(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CurrentUserResponse>>, ApiError> {
    // Local mode: return hardcoded local user
    let remote_client = match deployment.remote_client() {
        Ok(c) => c,
        Err(_) => {
            return Ok(ResponseJson(ApiResponse::success(CurrentUserResponse {
                user_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            })));
        }
    };
    // ...existing remote path unchanged...
    let access_token = remote_client.access_token().await.map_err(ApiError::from)?;
    let user_id = utils::jwt::extract_subject(&access_token).map_err(|e| {
        tracing::error!(?e, "Failed to extract user ID from token");
        ApiError::Unauthorized
    })?;
    Ok(ResponseJson(ApiResponse::success(CurrentUserResponse { user_id })))
}
```

### Change 4 (Optional): Add `id` to `CreateProjectRequest` in `crates/server/src/routes/kanban_v1/projects.rs:27`

```rust
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    #[serde(default)]
    pub id: Option<Uuid>,   // ← added for client-generated UUID support
    pub name: String,
    pub color: String,
    pub organization_id: Uuid,
}
```

And modify `create_project` handler to use `payload.id.unwrap_or_else(Uuid::new_v4)` instead of `Uuid::new_v4()`.

### Change 5 (Optional): Handle relay hosts gracefully

In `crates/server/src/routes/relay_auth/client.rs`, wrap the `deployment.relay_hosts()?` call to return `{"hosts": []}` instead of 400 when relay is not configured.

---

## Verification Checklist

After fixes:
- [ ] `curl http://127.0.0.1:41615/api/auth/token` returns `{"success":true,"data":{"access_token":"...","expires_at":null}}` (not 400)
- [ ] `curl http://127.0.0.1:41615/api/auth/user` returns local user profile (not 400)
- [ ] Browser console: no "Not authenticated" errors
- [ ] Settings → Projects loads without "Not Authorized"
- [ ] "+" icon → Create project dialog opens and creates project successfully
- [ ] New project appears in sidebar immediately after creation
- [ ] No `GET /api/auth/token 400` in network tab
- [ ] No `GET /api/relay-auth/client/hosts 400` in network tab
