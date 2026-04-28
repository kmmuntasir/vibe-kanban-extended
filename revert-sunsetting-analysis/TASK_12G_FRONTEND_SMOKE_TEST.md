# Task 12G: Frontend Smoke Test Plan

Manual test checklist for verifying the local kanban frontend after reverting
the sunsetting changes. No E2E framework exists — execute each step by hand.

---

## 1. Environment Setup

### Prerequisites

- Node.js (v18+)
- pnpm (`corepack enable && corepack prepare pnpm@latest --activate`)
- Rust toolchain (`rustup`)

### Start dev environment

```bash
pnpm i
pnpm run dev
```

The dev script starts the Vite frontend and Rust backend with auto-assigned
ports. The frontend URL is printed to console, typically:

```
http://localhost:{FRONTEND_PORT}
```

Override ports with env vars:

```bash
FRONTEND_PORT=3000 BACKEND_PORT=3001 pnpm run dev
```

### Key locations

| Item | Path |
|------|------|
| SQLite database | `~/.vibe-kanban/db.v2.sqlite` |
| Vite proxy config | `packages/local-web/vite.config.ts` (`/api/*` → backend) |
| Local auth logic | `crates/local-deployment/src/lib.rs:410` |
| First-boot seed | `crates/server/src/first_boot.rs` |

---

## 2. Test Checklist

### 1. App loads without login prompt

- **Action**: Open dev URL in browser.
- **Expected**: App renders the kanban board immediately. No login page, no
  auth redirect. User identity area shows username `local-user`.
- **Reason**: `LocalDeployment::get_login_status()` returns `LoggedIn` with
  username `"local-user"` when no remote is configured.
- [ ] Pass / Fail

### 2. Board renders 4 visible columns

- **Action**: Observe the board column headers.
- **Expected**: Exactly 4 columns displayed: **To do**, **In progress**,
  **In review**, **Done**. Columns **Backlog** and **Cancelled** are hidden
  (`sortedStatuses.filter(s => !s.hidden)`).
- [ ] Pass / Fail

### 3. No sunset page or deprecation banner

- **Action**: Check the full viewport and any notification/banner area.
- **Expected**: No sunset notice, deprecation warning, or "product discontinued"
  banner visible anywhere on the page.
- [ ] Pass / Fail

### 4. Create issue "Test issue"

- **Action**: Click the issue creation button (usually `+` in "To do" column
  or top-right). Enter title "Test issue" and submit.
- **Expected**: New issue card appears in the **To do** column with title
  "Test issue" and ID **VK-1** (prefix set by `organizations.issue_prefix` =
  `"VK"`).
- [ ] Pass / Fail

### 5. Drag VK-1 to "In progress"

- **Action**: Drag the VK-1 card from "To do" to "In progress" column.
- **Expected**: Card moves to "In progress" column. Refresh the page (F5).
  Card remains in "In progress" after refresh.
- [ ] Pass / Fail

### 6. Open issue detail panel

- **Action**: Click on the VK-1 card.
- **Expected**: Detail panel / modal opens showing title "Test issue", ID VK-1,
  current status "In progress".
- [ ] Pass / Fail

### 7. Add comment to VK-1

- **Action**: In the detail panel, type a comment (e.g. "smoke test comment")
  and submit.
- **Expected**: Comment appears in the activity feed / comments section of the
  detail panel immediately.
- [ ] Pass / Fail

### 8. Assign local user to VK-1

- **Action**: In the detail panel, open the assignee section and assign
  `local-user`.
- **Expected**: Avatar and/or username "local-user" shown in the assignee
  area of both the detail panel and the card.
- [ ] Pass / Fail

### 9. Create and assign tag "urgent"

- **Action**: Create a new tag named "urgent" (via tag management). Then assign
  it to VK-1 from the detail panel.
- **Expected**: Tag badge labeled "urgent" appears on the VK-1 card on the
  board.
- [ ] Pass / Fail

### 10. Create second issue

- **Action**: Create another issue titled "Second test issue".
- **Expected**: New issue appears in "To do" with ID **VK-2**. VK-1 still
  exists in "In progress".
- [ ] Pass / Fail

### 11. Data persistence across restart

- **Action**: Stop the dev server (`Ctrl+C`). Run `pnpm run dev` again. Open
  the dev URL.
- **Expected**: VK-1 is in "In progress", VK-2 is in "To do", comments and
  assignments are retained.
- [ ] Pass / Fail

### 12. No duplicate seed data after restart

- **Action**: After the restart from step 11, check the project selector and
  column count.
- **Expected**: Still exactly 1 project ("Main Project"), 4 visible columns.
  No duplicate statuses, tags, or projects.
- [ ] Pass / Fail

---

## 3. Cleanup — Reset Database

Delete the SQLite database to reset all state to a clean first-boot state:

```bash
rm ~/.vibe-kanban/db.v2.sqlite
```

Then restart:

```bash
pnpm run dev
```

First-boot seed runs again: 1 org, 1 project ("Main Project"), 6 statuses
(4 visible), 4 tags, issue counter reset to 1.

---

## 4. Known Issues & Notes

| Item | Detail |
|------|--------|
| DB location | Resolved via `utils::assets::asset_dir()` — typically `~/.vibe-kanban/db.v2.sqlite` |
| Port conflicts | Dev script auto-assigns ports. Override with `FRONTEND_PORT` / `BACKEND_PORT` env vars |
| Vite proxy | `/api/*` requests proxy to backend; if backend isn't ready, API calls fail until it starts |
| Issue ID prefix | Controlled by `organizations.issue_prefix`, set to `"VK"` in `first_boot.rs:42` |
| Hidden statuses | `Backlog` and `Cancelled` are seeded with `hidden = true` — filtered out of board columns |
