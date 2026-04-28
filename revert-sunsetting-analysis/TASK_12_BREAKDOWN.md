# Task 12 Breakdown — E2E Integration Verification

Generated: 2026-04-28
Source: [LOCAL_KANBAN_IMPLEMENTATION_PLAN-tasks.md](LOCAL_KANBAN_IMPLEMENTATION_PLAN-tasks.md) Task 12

---

## Why This Breakdown

Task 12 was a single monolithic quality gate with 15 steps. This breakdown splits it into 7 focused tasks that can run in parallel batches, reducing wall-clock time and enabling multiple agents/developers to verify independently.

## Parallelization Strategy

### Batch Dependency Diagram

```
Batch 0: CI Gate
  │    ┌──────────────────────┐
  │    │      Task 12a        │
  │    │  CI Gate (Build &    │
  │    │  Lint)               │
  │    └──────────┬───────────┘
  │               │
Batch 1: Foundation (parallel)
  │    ┌──────────┴───────────┐
  │    │                      │
  │  ┌─▼──────────┐  ┌───────▼──────────┐
  │  │  Task 12b  │  │    Task 12g      │
  │  │  Test      │  │  Frontend Smoke  │
  │  │  Harness   │  │  Test (Manual)   │
  │  └─────┬──────┘  └──────────────────┘
  │        │
Batch 2: Core Integration Tests (parallel)
  │   ┌────┴─────┐
  │   │          │
  │ ┌─▼───────┐ ┌▼──────────┐
  │ │Task 12c │ │ Task 12d  │
  │ │First-   │ │ Issue     │
  │ │Boot &   │ │ CRUD      │
  │ │Auth     │ │ Tests     │
  │ └────┬────┘ └─────┬─────┘
  │      │            │
Batch 3: Extended Tests (parallel)
  │   ┌──┴────────────┴──┐
  │   │                   │
  │ ┌─▼─────────┐ ┌──────▼───────┐
  │ │ Task 12e  │ │  Task 12f    │
  │ │ Entity    │ │ Persistence  │
  │ │ Mutation  │ │ & Idempotency│
  │ │ Tests     │ │ Tests       │
  │ └───────────┘ └──────────────┘
```

### Summary Table

| # | Batch | Title | Target Files | Deps | Parallel With |
|---|-------|-------|-------------|------|---------------|
| 12a | 0 | CI Gate | None (run commands only) | None | -- |
| 12b | 1 | Test Harness | `crates/server/tests/kanban_integration/{mod,helpers}.rs` | 12a | 12g |
| 12g | 1 | Frontend Smoke Test | `revert-sunsetting-analysis/TASK_12G_FRONTEND_SMOKE_TEST.md` | 12a | 12b |
| 12c | 2 | First-Boot & Auth Tests | `crates/server/tests/kanban_integration/first_boot_test.rs` | 12b | 12d |
| 12d | 2 | Issue CRUD Tests | `crates/server/tests/kanban_integration/issues_test.rs` | 12b | 12c |
| 12e | 3 | Entity Mutation Tests | `crates/server/tests/kanban_integration/entities_test.rs` | 12b, 12d | 12f |
| 12f | 3 | Persistence & Idempotency Tests | `crates/server/tests/kanban_integration/persistence_test.rs` | 12b, 12d | 12e |

### Merge Order Rules

1. **Batch 0 → Batch 1**: CI must be green before writing tests (broken build = wasted effort).
2. **Batch 1 → Batch 2**: Test harness must compile and provide `setup_test_app()` before integration tests can be written.
3. **Batch 2 → Batch 3**: Issue CRUD tests must pass (confirming `simple_id` counter works) before testing entity mutations on issues and persistence scenarios.
4. **Task 12g** is independent of 12b-12f — a human can execute the manual checklist while agents write automated tests.

### Developer Assignment Tracks (2 developers/agents)

**Track A — Backend Test Automation**

| Phase | Tasks | Estimate |
|-------|-------|----------|
| Step 1 | 12a (CI gate) | 10 min |
| Step 2 | 12b (test harness) | 2-3 hrs |
| Step 3 | 12c (first-boot tests) | 1-2 hrs |
| Step 4 | 12d (issue CRUD tests) | 2-3 hrs |
| Step 5 | 12e (entity mutation tests) | 2-3 hrs |
| Step 6 | 12f (persistence tests) | 1-2 hrs |

**Track B — Manual QA**

| Phase | Tasks | Estimate |
|-------|-------|----------|
| Step 1 | 12a (CI gate, shared with Track A) | 10 min |
| Step 2 | 12g (frontend smoke test document + manual execution) | 2-3 hrs |

**Critical path**: 12a → 12b → 12d → 12e

**Risk areas:**
1. **Test harness construction** (Task 12b): `LocalDeployment::new()` is heavyweight — reads filesystem, creates DB from disk. Must find a way to inject in-memory pool. If `Deployment` trait can't be easily mocked, may need to construct kanban sub-router directly with `DBService` state.
2. **sqlx compile-time checking**: `sqlx::query!` macros validate SQL at compile time against offline query data. Must run `pnpm run prepare-db` if adding new queries.
3. **Response shape differences**: `/api/info` returns `ApiResponse<T>` wrapper. Kanban fallback routes return raw `Json<T>`. Mutation routes return `MutationResponse<T>`. Each test must parse the correct shape.

---

## Batch 0 — CI Gate

### Task 12a: CI Gate — Build, Type-Check, Format, Lint

**Title**: Run all automated CI checks as a prerequisite gate

**Description**:

Run five commands in order from the repo root. Every command must exit with code 0 and produce zero errors. If any command fails, stop and fix before proceeding to Tasks 12b-12g.

**Commands:**

1. **Rust type-check:**
   ```bash
   cargo check --workspace
   ```
   Zero errors. Warnings (unused imports, dead code) are acceptable but should be noted.

2. **Rust tests:**
   ```bash
   cargo test --workspace
   ```
   All existing tests pass. This includes in-memory SQLite tests in `crates/local-deployment/src/first_boot.rs:93-189` and startup tests in `crates/server/src/startup.rs:423-474`.

3. **Frontend type-check:**
   ```bash
   pnpm run check
   ```
   Zero TypeScript errors across `packages/local-web/`, `packages/web-core/`, `shared/`. Also runs `cargo check --workspace`.

4. **Format check:**
   ```bash
   pnpm run format
   ```
   Zero formatting violations. Runs `cargo fmt` for Rust and Prettier for TypeScript/JSON/CSS.

5. **Lint check:**
   ```bash
   pnpm run lint
   ```
   Zero ESLint errors in web packages. Zero `cargo clippy` errors across all backend Rust workspaces.

**Acceptance Criteria:**

- [ ] `cargo check --workspace` exits 0, zero errors
- [ ] `cargo test --workspace` exits 0, all tests pass
- [ ] `pnpm run check` exits 0, zero type errors
- [ ] `pnpm run format` exits 0, no formatting changes
- [ ] `pnpm run lint` exits 0, zero clippy/ESLint errors
- [ ] No source files modified by any command

**Dependencies**: None

---

## Batch 1 — Foundation (parallel)

### Task 12b: Backend Integration Test Harness

**Title**: Create reusable test infrastructure for HTTP-level integration tests

**Description**:

Create a test utility module at `crates/server/tests/kanban_integration/` that provides an in-memory SQLite database, first-boot seeding, and an axum router suitable for `tower::ServiceExt::oneshot()` testing.

**Background:**

- Existing in-memory SQLite pattern: `crates/local-deployment/src/first_boot.rs:99-112`
- Router construction: `crates/server/src/routes/mod.rs:37-88`
- Kanban sub-router: `crates/server/src/routes/kanban_v1/mod.rs`
- `DBService` struct: `crates/db/src/lib.rs` — has `pub pool: SqlitePool`
- `Deployment` trait: `crates/deployment/src/lib.rs` — all kanban handlers use `State(deployment).db().pool`

**Key challenge**: `LocalDeployment::new()` (at `crates/local-deployment/src/lib.rs:93-304`) reads filesystem config and creates a file-backed DB. Cannot be used directly for in-memory testing. Two approaches:

**Approach A (preferred)**: Construct a `LocalDeployment` normally but replace its `DBService.pool` field with an in-memory pool after construction. Requires `pool` to be mutable/reassignable (check `DBService` struct visibility).

**Approach B**: Build the `kanban_v1::router()` sub-router directly with a lightweight test-only state that satisfies the `Deployment` trait. Only kanban routes are tested, so only `db()` accessor matters.

**Files to create:**

#### `crates/server/tests/kanban_integration/mod.rs`

Test module root. Declares submodules and provides the shared `setup_test_app()` entrypoint.

#### `crates/server/tests/kanban_integration/helpers.rs`

Core test utilities:

```rust
use std::str::FromStr;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{SqlitePool, SqlitePoolOptions};
use db::DBService;
use local_deployment::first_boot::initialize_if_empty;

/// Creates an in-memory SQLite pool with all migrations applied.
/// Follows pattern from crates/local-deployment/src/first_boot.rs:99-112.
pub async fn setup_pool() -> SqlitePool {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .journal_mode(SqliteJournalMode::Memory);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    let migrator = sqlx::migrate!("../db/migrations");
    migrator.run(&pool).await.unwrap();
    pool
}

/// Seeds first-boot data. Returns DBService with the pool.
pub async fn seed_db(pool: SqlitePool) -> DBService {
    let db = DBService { pool };
    initialize_if_empty(&db).await.unwrap();
    db
}

/// Returns (organization_id, project_id, first_status_id) from seeded database.
pub async fn get_seeded_ids(pool: &SqlitePool) -> (Uuid, Uuid, Uuid) {
    // Query organizations, projects, project_statuses tables
}

/// Builds test app: in-memory DB + seed + axum Router.
/// Returns router ready for tower::ServiceExt::oneshot().
pub async fn setup_test_app() -> Router { /* ... */ }
```

#### Dev-dependencies to add to `crates/server/Cargo.toml`:

```toml
[dev-dependencies]
tower = "0.5"
http-body-util = "0.1"
```

#### Smoke test:

Include one smoke test in `helpers.rs` that calls `setup_test_app()`, sends `GET /fallback/projects?organization_id=<seeded_org_id>`, and asserts 200 with JSON body.

**Acceptance Criteria:**

- [ ] `crates/server/tests/kanban_integration/mod.rs` exists with module declarations
- [ ] `crates/server/tests/kanban_integration/helpers.rs` exists with `setup_pool()`, `seed_db()`, `get_seeded_ids()`, `setup_test_app()`
- [ ] `setup_pool()` creates in-memory SQLite, runs migrations, returns `SqlitePool`
- [ ] `seed_db()` calls `initialize_if_empty()` and returns `DBService`
- [ ] `setup_test_app()` returns axum `Router` processable via `tower::ServiceExt::oneshot()`
- [ ] Dev-dependencies added: `tower`, `http-body-util`
- [ ] Smoke test passes: GET `/fallback/projects` returns 200 with seeded project data
- [ ] `cargo test -p server --test kanban_integration` compiles and passes

**Dependencies**: Task 12a

---

### Task 12g: Frontend UI Smoke Test (Manual Checklist)

**Title**: Create structured manual test plan for frontend verification

**Description**:

Create a repeatable manual test checklist document. No E2E framework (Playwright/Cypress) exists in the project, so this must be human-executed.

**Background:**

- Dev server: `pnpm run dev` starts Vite frontend + Rust backend with auto-assigned ports
- Vite proxy: `packages/local-web/vite.config.ts` proxies `/api/*` to backend
- Local auth: `LocalDeployment::get_login_status()` at `crates/local-deployment/src/lib.rs:410` returns `LoggedIn` with username `"local-user"` when no remote configured
- First-boot seed: 1 org, 1 project ("Main Project"), 6 statuses, 4 tags
- Board columns: `sortedStatuses.filter(s => !s.hidden)` renders 4 visible columns (To do, In progress, In review, Done). Backlog and Cancelled are hidden.
- Issue IDs: `VK-{n}` prefix from `organizations.issue_prefix` (set to `"VK"` by `first_boot.rs:42`)
- DB location: `~/.vibe-kanban/db.v2.sqlite` (via `utils::assets::asset_dir()`)

**Target file:** `revert-sunsetting-analysis/TASK_12G_FRONTEND_SMOKE_TEST.md`

**Document structure:**

**Section 1: Environment Setup**
- Prerequisites: Node.js, pnpm, Rust toolchain
- Commands: `pnpm i`, `pnpm run dev`
- How to find dev URL (printed to console, typically `http://localhost:{FRONTEND_PORT}`)

**Section 2: Checklist** — 12 steps, each with action, expected result, pass/fail checkbox:

1. App loads without login prompt. User identity shows "local-user".
2. Kanban board renders 4 visible columns: To do, In progress, In review, Done.
3. No sunset page or deprecation banner visible.
4. Create issue "Test issue" — appears in "To do" with ID VK-1.
5. Drag VK-1 to "In progress" — refresh — persists.
6. Open VK-1 detail panel — title "Test issue" shown.
7. Add comment to VK-1 — appears in activity feed.
8. Assign local user to VK-1 — avatar/username in assignee section.
9. Create tag "urgent" — assign to VK-1 — tag badge on card.
10. Create second issue — receives ID VK-2.
11. Stop server (Ctrl+C), restart — all data persists (VK-1 in "In progress", VK-2 exists).
12. No duplicate seed data — still 1 project, 4 visible columns.

**Section 3: Cleanup**
- Reset command: delete `~/.vibe-kanban/db.v2.sqlite` and restart.

**Section 4: Known Issues**
- Default DB location via `asset_dir()`
- Port conflicts: dev script auto-assigns
- `FRONTEND_PORT` / `BACKEND_PORT` env vars override defaults

**Acceptance Criteria:**

- [ ] File `revert-sunsetting-analysis/TASK_12G_FRONTEND_SMOKE_TEST.md` created
- [ ] Contains all 12 checklist steps with action, expected result, pass/fail checkbox
- [ ] Steps reference correct column visibility (4 visible, not 6)
- [ ] Steps verify no sunset page and no login prompt
- [ ] Environment setup section includes exact commands
- [ ] Cleanup section explains data reset
- [ ] Valid markdown formatting

**Dependencies**: Task 12a

---

## Batch 2 — Core Integration Tests (parallel)

### Task 12c: First-Boot & Auth Integration Tests

**Title**: Test first-boot seeding and local auth bypass via HTTP

**Description**:

Write integration tests that verify the first-boot initialization produces correct seed data through the actual HTTP routes, and that local auth bypass returns logged-in status.

**Target file:** `crates/server/tests/kanban_integration/first_boot_test.rs`

**Seed data constants** (from `crates/local-deployment/src/first_boot.rs`):
- Organization: deterministic ID `Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"vibe-kanban-local")`, name "My Workspace", slug "local-workspace", issue_prefix "VK"
- Project: random `Uuid::new_v4()`, name "Main Project", color "217 91% 60%"
- 6 statuses: Backlog (hidden), To do, In progress, In review, Done, Cancelled (hidden)
- 4 tags: bug, feature, documentation, enhancement

**Tests to write:**

**Test 1: `test_login_status_local_mode`**
- GET `/api/info` (handler at `crates/server/src/routes/config.rs:106-178`)
- Parse response. Verify `login_status.status == "loggedin"`.
- Note: if `setup_test_app()` only builds kanban sub-router, this test may need `setup_full_app()` that includes the config routes. If that's impractical, skip and document as manual-only (covered by Task 12g step 1).

**Test 2: `test_fallback_projects_seeded`**
- GET `/fallback/projects?organization_id={org_id}`
- Response shape: `{ "projects": [...] }`
- Assert exactly 1 project with `name == "Main Project"`

**Test 3: `test_fallback_project_statuses_seeded`**
- GET `/fallback/project_statuses?project_id={project_id}`
- Assert exactly 6 statuses
- Verify names: Backlog, To do, In progress, In review, Done, Cancelled

**Test 4: `test_fallback_tags_seeded`**
- GET `/fallback/tags?project_id={project_id}`
- Assert exactly 4 tags
- Verify names: bug, feature, documentation, enhancement

**Test 5: `test_first_boot_idempotent`**
- Call `initialize_if_empty()` a second time on the same DBService
- Re-query all fallback endpoints
- Assert same counts: 1 org, 1 project, 6 statuses, 4 tags (no duplicates)

**Response shape note:** Kanban fallback routes return unwrapped JSON (`{ "projects": [...] }`), NOT `ApiResponse<T>`. Kanban mutation routes return `{ "data": ..., "txid": N }`. The `/api/info` route returns `ApiResponse<UserSystemInfo>`.

**Acceptance Criteria:**

- [ ] `crates/server/tests/kanban_integration/first_boot_test.rs` exists with 4-5 test functions
- [ ] `test_fallback_projects_seeded` — returns exactly 1 project named "Main Project"
- [ ] `test_fallback_project_statuses_seeded` — returns exactly 6 statuses with correct names
- [ ] `test_fallback_tags_seeded` — returns exactly 4 tags with correct names
- [ ] `test_first_boot_idempotent` — second `initialize_if_empty()` produces no duplicates
- [ ] Org ID computed deterministically; project ID obtained dynamically (it's `Uuid::new_v4()`)
- [ ] All tests pass via `cargo test -p server --test kanban_integration first_boot`
- [ ] No filesystem side effects

**Dependencies**: Task 12b

---

### Task 12d: Issue CRUD Integration Tests

**Title**: Test issue lifecycle — create, read, update, delete, simple_id counter

**Description**:

Write integration tests for the full issue CRUD cycle, verifying the `simple_id` counter (`VK-1`, `VK-2`, ...) increments atomically within a transaction.

**Target file:** `crates/server/tests/kanban_integration/issues_test.rs`

**Route reference** (from `crates/server/src/routes/kanban_v1/issues.rs`):

| Method | Path | Handler |
|--------|------|---------|
| POST | `/issues` | create_issue |
| GET | `/issues?project_id={id}` | list_issues |
| POST | `/issues/search` | search_issues |
| GET | `/issues/{id}` | get_issue |
| PATCH | `/issues/{id}` | update_issue |
| DELETE | `/issues/{id}` | delete_issue |

**Request/response shapes** (from `crates/api-types/src/issue.rs`):

Create request:
```json
{
  "project_id": "<uuid>",
  "status_id": "<uuid>",
  "title": "Test issue",
  "sort_order": 0,
  "extension_metadata": {}
}
```

Create response (`MutationResponse<Issue>` from `api-types/src/response.rs:12-15`):
```json
{
  "data": { "simple_id": "VK-1", "issue_number": 1, "title": "Test issue", ... },
  "txid": 1714300800000
}
```

**simple_id generation logic** (from `crates/db/src/models/issue.rs`):
1. Begin transaction
2. `UPDATE projects SET issue_counter = issue_counter + 1 WHERE id = $1 RETURNING issue_counter` (atomic increment)
3. Look up `organizations.issue_prefix` via JOIN (always `"VK"` locally)
4. Format `simple_id` as `"{prefix}-{issue_number}"`: `"VK-1"`, `"VK-2"`, etc.
5. Insert issue, commit

**Tests to write:**

**Test 1: `test_create_issue`**
- POST `/issues` with seeded project_id and status_id
- Assert response status 200
- Parse as `MutationResponse<Issue>`
- Assert `data.simple_id == "VK-1"`, `data.issue_number == 1`, `data.title == "Test issue"`

**Test 2: `test_list_issues_returns_created`**
- Create an issue, then GET `/issues?project_id={id}`
- Assert `total_count == 1`, `issues[0].simple_id == "VK-1"`

**Test 3: `test_update_issue_status`**
- Create an issue, get a different status_id from the 6 seeded statuses
- PATCH `/issues/{id}` with `{ "status_id": "<new_status_id>" }`
- Assert `data.status_id == new_status_id`, `data.title` unchanged

**Test 4: `test_issue_counter_increments`**
- Create a second issue with title "Second issue"
- Assert `data.simple_id == "VK-2"`, `data.issue_number == 2`
- GET issues list, assert `total_count == 2`

**Test 5: `test_delete_issue`**
- Create an issue, then DELETE `/issues/{id}`
- Assert response shape `{ "txid": <number> }` (from `DeleteResponse`)
- GET issues list, assert count reduced
- GET `/issues/{id}` returns error (400 or 404)

**Test 6: `test_create_issue_with_optional_fields`** (stretch)
- Create issue with `description`, `priority: "high"`, `parent_issue_id`
- Verify all fields persisted in response

**Helper addition to `helpers.rs`:**
```rust
/// Returns (organization_id, project_id, first_status_id) from seeded DB.
pub async fn get_seeded_ids(pool: &SqlitePool) -> (Uuid, Uuid, Uuid) { ... }
```

**Acceptance Criteria:**

- [ ] `crates/server/tests/kanban_integration/issues_test.rs` exists with 5-6 test functions
- [ ] `test_create_issue` — POST returns `simple_id == "VK-1"`, `issue_number == 1`
- [ ] `test_list_issues_returns_created` — GET returns created issue
- [ ] `test_update_issue_status` — PATCH changes status_id, preserves title
- [ ] `test_issue_counter_increments` — second issue gets `VK-2`
- [ ] `test_delete_issue` — DELETE returns `{ "txid": N }`, subsequent GET fails
- [ ] `get_seeded_ids()` helper added to `helpers.rs`
- [ ] All tests pass via `cargo test -p server --test kanban_integration issues`
- [ ] No filesystem side effects

**Dependencies**: Task 12b

---

## Batch 3 — Extended Tests (parallel)

### Task 12e: Entity Mutation Integration Tests

**Title**: Test all non-issue kanban entity CRUD routes

**Description**:

Write integration tests covering tags, issue-tags, issue-assignees, issue-relationships, project statuses, and projects (including bulk endpoints).

**Target file:** `crates/server/tests/kanban_integration/entities_test.rs`

**Route references** (from `crates/server/src/routes/kanban_v1/`):

| Entity | File | Methods |
|--------|------|---------|
| Tags | `tags.rs` | list, get, create, update, delete |
| Issue Tags | `issue_tags.rs` | list, get, create, delete |
| Issue Assignees | `issue_assignees.rs` | list, get, create, delete |
| Issue Relationships | `issue_relationships.rs` | list, get, create, delete |
| Project Statuses | `project_statuses.rs` | list, get, create, update, delete, bulk |
| Projects | `projects.rs` | list, get, create, update, delete, bulk |

**Tests to write:**

**12e.1 Tags CRUD:**
- GET `/tags?project_id={id}` — returns 4 default tags
- POST `/tags` with `{ "project_id", "name": "urgent", "color": "0 100% 50%" }` — returns new tag
- PATCH `/tags/{id}` with `{ "color": "120 80% 40%" }` — returns updated tag
- PATCH `/tags/{id}` with `{ "color": "invalid" }` — returns 400 (color validation)
- DELETE `/tags/{id}` — returns `{ "txid": N }`

**12e.2 Issue Tags:**
- Create an issue first (use helper from 12d or direct DB call)
- POST `/issue_tags` with `{ "issue_id", "tag_id" }` — returns new issue_tag
- GET `/issue_tags?issue_id={id}` — returns list with 1 entry
- DELETE `/issue_tags/{id}` — returns `{ "txid": N }`
- POST `/issue_tags` with non-existent tag_id — returns error (FK violation)

**12e.3 Issue Assignees:**
- POST `/issue_assignees` with `{ "issue_id", "user_id": <any_uuid> }` — returns new assignee
- GET `/issue_assignees?issue_id={id}` — returns list
- DELETE `/issue_assignees/{id}` — returns `{ "txid": N }`
- POST duplicate (same issue_id + user_id) — returns 409 (UNIQUE constraint)

**12e.4 Issue Relationships:**
- Create two issues
- POST `/issue_relationships` with `{ "issue_id", "related_issue_id", "relationship_type": "blocking" }`
- GET `/issue_relationships?issue_id={id}` — returns list
- DELETE `/issue_relationships/{id}` — returns `{ "txid": N }`

**12e.5 Project Statuses:**
- GET `/project_statuses?project_id={id}` — returns 6 default statuses
- POST `/project_statuses` with `{ "project_id", "name": "Ready for QA", "color": "200 50% 50%", "sort_order": 10 }`
- PATCH `/project_statuses/{id}` with `{ "sort_order": 99, "hidden": true }`
- DELETE `/project_statuses/{id}`
- POST `/project_statuses/bulk` with array of updates — returns updated statuses

**12e.6 Projects:**
- GET `/projects?organization_id={id}` — returns 1 project
- POST `/projects` — creates project + 6 default statuses
- PATCH `/projects/{id}` with `{ "name": "Renamed" }`
- DELETE `/projects/{id}`
- POST `/projects/bulk` with array of updates

**Acceptance Criteria:**

- [ ] `crates/server/tests/kanban_integration/entities_test.rs` exists and compiles
- [ ] Tags: 5 tests covering CRUD + color validation
- [ ] Issue tags: 4 tests covering CRUD + FK constraint
- [ ] Issue assignees: 4 tests covering CRUD + UNIQUE constraint
- [ ] Issue relationships: 4 tests covering CRUD
- [ ] Project statuses: 6 tests including bulk update
- [ ] Projects: 5 tests including bulk update, verify status seeding on create
- [ ] `cargo test -p server --test kanban_integration entities` passes
- [ ] Each test function uses its own temp DB (test isolation)

**Dependencies**: Task 12b, Task 12d

---

### Task 12f: Data Persistence & Idempotency Tests

**Title**: Test data survival across simulated restarts and first-boot idempotency

**Description**:

Write integration tests that prove data survives a simulated server restart (pool drop + reconnect to same file) and that `first_boot::initialize_if_empty()` is idempotent.

**Target file:** `crates/server/tests/kanban_integration/persistence_test.rs`

**Key difference from other tests**: These tests MUST use file-backed temp SQLite (`tempfile::NamedTempFile` with `SqliteJournalMode::Delete`) to match production behavior. In-memory pools can't simulate reconnect.

**Tests to write:**

**Test 1: `test_data_persists_across_pool_reconnect`**
1. Create temp file SQLite DB, run migrations, seed first-boot data
2. Create issue VK-1, change status to "In progress", assign user via `POST /issue_assignees`
3. Drop the pool (`drop(pool)`)
4. Open fresh pool against same temp file path, run migrations (idempotent)
5. Query: VK-1 exists, status is "In progress", assignee present, `issue_counter == 1`

**Test 2: `test_issue_counter_survives_restart`**
1. Seed DB, create VK-1 (counter = 1)
2. Drop pool, reconnect to same file
3. Create second issue — assert `simple_id == "VK-2"` (NOT "VK-1" again)

**Test 3: `test_initialize_if_empty_idempotent`**
1. Seed DB via `initialize_if_empty()`
2. Create extra issues, tags
3. Call `initialize_if_empty()` again
4. Assert: 1 org (not 2), 1 project (not 2), 6 statuses (not 12), 4 tags (not 8)
5. All user-created data intact

**Test 4: `test_concurrent_initialization_safe`**
1. Fresh DB
2. Call `initialize_if_empty()` from two concurrent tasks (`tokio::join!`)
3. Verify no duplicates: 1 org, 1 project, 6 statuses, 4 tags
4. No panics

**Test 5: `test_migration_idempotent`**
1. Run migrations, insert data
2. Run migrations again
3. Verify `_sqlx_migrations` count unchanged, data intact

**File-backed pool setup pattern:**
```rust
async fn setup_file_pool() -> (SqlitePool, NamedTempFile) {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_string_lossy().to_string();
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path))
        .unwrap()
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Delete);
    let pool = SqlitePoolOptions::new()
        .connect_with(opts)
        .await
        .unwrap();
    sqlx::migrate!("../db/migrations").run(&pool).await.unwrap();
    (pool, temp)
}
```

**Acceptance Criteria:**

- [ ] `crates/server/tests/kanban_integration/persistence_test.rs` exists and compiles
- [ ] `test_data_persists_across_pool_reconnect` passes — data intact after pool drop/reconnect
- [ ] `test_issue_counter_survives_restart` passes — VK-2 not VK-1 after reconnect
- [ ] `test_initialize_if_empty_idempotent` passes — no duplicate seed data
- [ ] `test_concurrent_initialization_safe` passes — no panics, no duplicates
- [ ] `test_migration_idempotent` passes — re-running migrations doesn't corrupt data
- [ ] All tests use file-backed temp SQLite (not in-memory)
- [ ] `cargo test -p server --test kanban_integration persistence` passes
- [ ] Each test is independent with its own temp file

**Dependencies**: Task 12b, Task 12d
