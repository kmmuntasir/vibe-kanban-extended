---
name: start-vibe-kanban-workspaces
description: Start Vibe Kanban workspaces from a task creation report file. Use when user wants to launch workspaces on their Vibe Kanban dashboard, providing issue IDs from a previously generated report.
---

# Start Vibe Kanban Workspaces Skill

Read task creation report file. Fetch issue details via MCP. Start workspace per task linked to its Vibe Kanban issue.

## Inputs

User provides:

1. **Report file path** — Markdown report containing Vibe Kanban issue IDs (e.g., `docs/tasks/backend/01-project-setup/01-project-setup-tasks-vibe-kanban-report.md`)
2. **Which tasks to start** — Task IDs (e.g., "T1, T3, T5"), batch numbers (e.g., "batch 1", "batches 1-2"), or "all"

Ask user if path unclear or tasks unspecified.

## Execution Steps

Follow these steps exactly, in order.

### Step 1: Read the report file

Read entire file with Read tool. Never truncate. Read in chunks if large.

### Step 2: Parse task-to-issue mapping from report

Find the **Task Summary** table and/or **Quick Reference: All Issue IDs** section. Extract mapping of task ID → Vibe Kanban issue UUID.

Expected formats in report:

**Task Summary table:**
```markdown
| Vibe Kanban ID | Task | Batch | Files | Depends On |
|---------------|------|-------|-------|-----------|
| `{uuid}` | T1 — Title | 1 | file.ts | None |
```

**Quick Reference section:**
```markdown
T1: {uuid}  # T1 — Title
T2: {uuid}  # T2 — Title
```

Also parse **Dependency Graph** to identify batch numbers and which tasks belong to which batch.

Store ordered list: `{ task_id, issue_uuid, title, batch }`.

Report: "Found N tasks across M batches."

### Step 3: Resolve which tasks to start

Apply user selection:

- **Task IDs** — Keep only those (e.g., "T1, T3, T5")
- **Batch number** — Keep all tasks in that batch (e.g., "batch 1" = all Batch 1 tasks)
- **Batch range** — Keep all tasks across range (e.g., "batches 1-3" = all tasks in Batches 1, 2, 3)
- **"all"** — Keep every task

When user says "batch 1" or "batches 1-2", resolve to individual task IDs using batch mapping from Step 2.

Report selected tasks. Confirm if 5+. Show compact table:

```
Tasks to start (N):

  Task    Batch  Title                              Issue ID
  ─────── ────── ────────────────────────────────── ──────────────────────────
  T1      1      Title here                         abc123...
  T2      1      Title here                         def456...
```

### Step 4: Fetch issue details

For each selected task, call `get_issue` to fetch full description:

```
mcp__vibe_kanban__get_issue
```

Arg: `issue_id` (UUID from Step 2).

Fire all `get_issue` calls **parallel**.

Save each issue's title + description. This becomes the workspace prompt.

### Step 5: Discover repositories

Call `mcp__vibe_kanban__list_repos`. Save repos (name + UUID).

Stop if no repos found.

Also get current branch: `git branch --show-current`.

### Step 6: Start workspaces

For each selected task, call `mcp__vibe_kanban__start_workspace`:

| Parameter | Value |
|-----------|-------|
| `name` | Format: `{TaskID} — {Title}` (e.g., `"T1 — Create package.json and tsconfig.json"`) |
| `prompt` | Full issue description from Step 4. Copy verbatim. Never summarize or truncate. |
| `executor` | `"CLAUDE_CODE"` unless user specifies otherwise |
| `repositories` | All repos from Step 5, each with `branch` = current git branch. Format: `[{"repo_id": "<uuid>", "branch": "<current-branch>"}]` |
| `issue_id` | UUID from Step 2 — links workspace to the Vibe Kanban issue |

**Rules:**

- Start all in parallel — fire all `start_workspace` calls in single message.
- Save every `workspace_id` for final report.
- Report after all done: "Started N workspaces."

### Step 7: Final report

Report summary with all workspace IDs.

```
Done. Started N workspaces.

Repository: <repo_name> (<repo_id>)
Branch: <branch_name>

Workspaces:
  Task   Batch  Title                              Workspace ID                Issue ID
  ────── ────── ────────────────────────────────── ──────────────────────────── ──────────────────────────
  T1     1      Title here                         <workspace_id_1>            <issue_id_1>
  T2     1      Title here                         <workspace_id_2>            <issue_id_2>
  ...

Skipped (N): <list if any>
```

Include issue IDs for traceability. List any skipped tasks with reason.

## Error Handling

- **Report file not found** → ask user to verify path.
- **No Task Summary table or Quick Reference section** → report "No issue IDs found in report." Check file is a valid task creation report.
- **Issue UUID malformed or missing** → skip task, report, continue.
- **`get_issue` fails** → skip task, report error, continue. Issue may have been deleted.
- **`start_workspace` fails** → report error, skip task, continue. Don't stop.
- **No repos found** → stop. User needs to link repository in Vibe Kanban first.
- **Task ID / batch not in report** → report, skip.

## Important Rules

1. **Never truncate** — Each workspace prompt = complete issue description fetched from Vibe Kanban.
2. **Never summarize** — Copy descriptions, acceptance criteria, code blocks verbatim.
3. **In parallel** — Start all workspaces in single batch. MCP handles concurrent calls.
4. **Save workspace IDs** — Store every returned ID.
5. **Link to issue** — Always pass `issue_id` to `start_workspace` so workspace is associated with its Vibe Kanban issue.
6. **Batch-aware** — Parse batch info from dependency graph. Users typically request workspaces by batch.
7. **Name format** — Use `{TaskID} — {Title}` matching report convention (e.g., `"T1 — Create package.json"`).
