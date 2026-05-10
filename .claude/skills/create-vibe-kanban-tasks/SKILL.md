---
name: create-vibe-kanban-tasks
description: Create Vibe Kanban tasks from a task breakdown markdown file. Use when user wants to create Kanban issues from a task plan file.
---

# Create Vibe Kanban Tasks Skill

Read task breakdown markdown file(s). Create Vibe Kanban issues with correct tags, priority, dependency relationships. Write task creation report alongside source file.

## Inputs

User provides one of:

1. **Single file path** — e.g., `docs/tasks/backend/01-project-setup/01-project-setup-tasks.md`
2. **Multiple file paths** — e.g., `docs/tasks/backend/01-project-setup/*-tasks.md`
3. **Folder path** — Process all `*.md` files inside (non-recursive)

If no input, ask. Also ask for **priority** (default: `high`) and **tag name** (default: `feature`) if not obvious.

## Execution Steps

Follow exactly, in order.

### Step 1: Read the task file(s)

Resolve input to absolute file paths:

- **Single file** — use as-is
- **Multiple files** — expand glob with Glob tool
- **Folder** — Glob `folder/*.md`, exclude non-task files (README, etc.)

Read every file completely. Never truncate.

### Step 2: Resolve Vibe Kanban resource IDs

**Primary — read metadata file:**

Read `./project-metadata.local.md` at repo root. Contains pre-discovered IDs, gitignored (local only).

Parse markdown table. Extract:

| Key | Description |
|-----|-------------|
| `organization_id` | Vibe Kanban organization UUID |
| `project_id` | Vibe Kanban project UUID |
| `repo_id` | Vibe Kanban repository UUID |

If all 3 keys have non-empty UUIDs, **skip MCP discovery** — jump to Step 2f.

**Fallback — MCP discovery (metadata missing or incomplete):**

Call MCP tools **in sequence** (each depends on prior):

**2a. List organizations:**
```
mcp__vibe_kanban__list_organizations
```
No args. Pick matching org. If only one, use it. Multiple? Match by project name or ask.

**2b. List projects:**
```
mcp__vibe_kanban__list_projects
```
Arg: `organization_id` (UUID from 2a). Match project by name to current repo/working directory. Multiple? Ask.

**2c. List repositories:**
```
mcp__vibe_kanban__list_repos
```
No args. Match repo by name to current working directory.

**After MCP discovery, write `project-metadata.local.md`** at repo root:

```markdown
# Vibe Kanban Project Metadata (local)

| Key | Value | UUID |
|-----|-------|------|
| `organization_id` | `<org_name>` | `<org_uuid>` |
| `project_id` | `<project_name>` | `<project_uuid>` |
| `repo_id` | `<repo_name>` | `<repo_uuid>` |
```

**2f. List existing tags (always run):**
```
mcp__vibe_kanban__list_tags
```
Arg: `project_id` (UUID). If desired tag missing, use closest available.

**2g. Check existing issues (optional):**
```
mcp__vibe_kanban__list_issues
```
Arg: `project_id` (UUID). Check for duplicates.

### Step 3: Parse tasks from file(s)

For each task in each file, extract:

| Field | Source | Example |
|-------|--------|---------|
| **Task ID** | Heading like `### T1 — <title>` or `### Task N: <title>` | `T1` |
| **Title** | Rest of heading after task ID | `Create package.json and tsconfig.json` |
| **Batch** | Section heading or summary table | `Batch 1: Project Scaffold` |
| **Files** | Under `**Files:**` or summary table | `backend/package.json` |
| **Description** | Everything under task heading: code blocks, tables, bullets, acceptance criteria, dependency info. Copy verbatim — never summarize. | — |
| **Dependencies** | `**Dependencies:**` line or summary table `Depends On` column | `T1`, `None` |
| **Parallel with** | `**Parallel with:**` line or summary table | `T2`, `T6-T9` |

Store in ordered list. Track source file.

Report: "Found N tasks across M files."

### Step 4: Create issues

For each task, call:
```
mcp__vibe_kanban__create_issue
```

Args:

| Parameter | Value |
|-----------|-------|
| `project_id` | UUID from Step 2 |
| `title` | `{TaskID} — {Title}` (e.g., `"T1 — Create package.json and tsconfig.json"`) |
| `description` | Concise but complete. Include: batch, files, implementation details, acceptance criteria as `- [ ]` items, dependencies, parallel tasks. **MUST end with source file path** — `**Source:** \`path/to/task-file.md\``. Markdown. |
| `priority` | User-specified or default `high` |

**Rules:**
- Fire all `create_issue` calls **parallel** — single batch, single message.
- Map every returned `issue_id` to task ID.
- On failure: report error, skip task, continue.
- Max 50 parallel calls. Split larger sets into batches.

Report: "Created N issues."

### Step 5: Add tags

```
mcp__vibe_kanban__add_issue_tag
```

Args:

| Parameter | Value |
|-----------|-------|
| `issue_id` | UUID from Step 4 |
| `tag_id` | UUID from Step 2f (match by name) |

Fire all **parallel**. One call per issue.

### Step 6: Create dependency relationships

For each `X depends on Y`, create **blocking** edge:
```
mcp__vibe_kanban__create_issue_relationship
```

Args:

| Parameter | Value |
|-----------|-------|
| `issue_id` | UUID of **dependent** (X — waits) |
| `related_issue_id` | UUID of **dependency** (Y — blocks) |
| `relationship_type` | `"blocking"` |

**Create edges for:**
- **Sequential within batch** — e.g., T10 → T11
- **Cross-batch** — e.g., all Batch 3 tasks block T10 (Batch 4)
- **Explicit deps** from `Dependencies` field — e.g., T12 depends on T2, T4

**Skip edges for:**
- **Parallel tasks within same batch** — no ordering
- **Implicit transitive deps** — if T10 blocks T11 and T5 blocks T10, skip T5 → T11 (redundant)

Build dependency graph:
1. Parse every task's `Dependencies` field.
2. For each dependency, create blocking edge: dependent → dependency.
3. Add batch-order edges: if Batch N depends on Batch N-1, edge from each Batch N task to referenced Batch N-1 tasks.

Fire all relationship calls **parallel**.

### Step 7: Write task creation report

Write in **same directory as source**. Name: `{original-filename}-vibe-kanban-report.md`.

Multiple input files → one report per file, or combined at folder root: `vibe-kanban-tasks-report.md`.

**Template:**

```markdown
# Vibe Kanban Task Creation Report

**Source:** `{relative-path-to-task-file}`
**Created:** {ISO timestamp}
**Project:** {project_name} (`{project_id}`)
**Repo:** {repo_name} (`{repo_id}`)
**Tag:** {tag_name} (`{tag_id}`)
**Default priority:** {priority}

---

## Task Summary

| Vibe Kanban ID | Task | Batch | Files | Depends On |
|---------------|------|-------|-------|-----------|
| `{issue_id}` | T1 — Title | 1 | file.ts | None |
| `{issue_id}` | T2 — Title | 1 | file.ts | None |
| ... | ... | ... | ... | ... |

---

## Dependency Graph

```
Batch 1: Project Scaffold (0 deps)
  T1 ─┬─ T2
      │
Batch 2: Infrastructure (depends on Batch 1)
  T3 ─┬─ T4
      │
Batch 3: Domain Entities (depends on T3)
  T5  T6  T7  T8  T9    ← all parallel
      │
Batch 4: DB Migration + Seed (depends on Batch 3)
  T10 ─→ T11
      │
Batch 5: Auth Middleware + Utils (depends on T2, T4)
  T12  T14  T15          ← parallel
      │
Batch 6: Role Guard (depends on T12)
  T13
      │
Batch 7: Integration Tests (depends on T12, T14)
  T16
```

## Blocking Relationships Created

| # | Dependent Task | Blocked By | Edge ID |
|---|---------------|-----------|---------|
| 1 | T10 | T5 | `{relationship_id}` |
| 2 | T10 | T6 | `{relationship_id}` |
| ... | ... | ... | ... |

## Merge Order

| Step | Merge Batches | Rule |
|------|--------------|------|
| 1 | Batch 1 into `main` | Foundation must exist before infra |
| 2 | Batch 2 into `main` | DataSource + utils must exist before entities |
| ... | ... | ... |

---

## Quick Reference: All Issue IDs

```
{task_id_1}: {issue_uuid_1}  # T1 — Title
{task_id_2}: {issue_uuid_2}  # T2 — Title
...
```
```

Copy existing dependency diagram and merge order sections verbatim if present in source.

## Error Handling

- **Metadata missing/incomplete** → MCP discovery (2a–2c). Write metadata after.
- **Stale/mismatched IDs** → `create_issue` fails. Delete `project-metadata.local.md`, re-run.
- **No org (MCP)** → Tell user to create org first.
- **No project (MCP)** → Tell user to create project first.
- **No repo (MCP)** → Tell user to link repo first.
- **`create_issue` fails** → Log task ID + error, skip, continue. Report failures at end.
- **`add_issue_tag` fails** → Log, continue. Add tag manually later.
- **`create_issue_relationship` fails** → Log edge, continue. Common cause: referencing nonexistent issue IDs.
- **Can't read file** → Ask user to verify path.
- **No tasks parsed** → Report "No task headings found." Check markdown structure.

## Important Rules

1. **Issue descriptions must be complete** — acceptance criteria as checkboxes, file list, implementation notes. Developer should understand task without source file. **Every description must end with `**Source:** \`path/to/task-file.md\`` line** pointing to original task breakdown file.
2. **All `create_issue` calls parallel** — single batch.
3. **Save every `issue_id`** — map to task ID immediately. Needed for tags + relationships.
4. **Title format** — `{TaskID} — {Title}`. Task ID from file (T1, T2, etc.).
5. **Dependency edges directional** — `issue_id` = dependent (waits), `related_issue_id` = blocker.
6. **Only blocking edges** — `"blocking"` type. Skip `"related"` and `"has_duplicate"` unless source explicitly calls for them.
7. **Report alongside source** — same directory. Folder batch → combined report at root.