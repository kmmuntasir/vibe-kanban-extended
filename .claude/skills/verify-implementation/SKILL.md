---
name: verify-implementation
description: Verify implementation against task/plan files, identify gaps and missing parts. Use when user requests to verify implementation completeness against task files in this project.
---

# Verify Implementation Skill

Read provided task/plan file(s). Analyze current codebase state against planned tasks. Identify gaps and missing implementations. Write comprehensive verification report.

## Inputs

User provides one of:

1. **Single file path** — e.g., `docs/tasks/01-setup-tasks.md`
2. **Multiple file paths** — e.g., `docs/tasks/*-tasks.md`
3. **Folder path** — Process all `*.md` files inside (non-recursive)

If no input, ask.

## Execution Steps

Follow exactly, in order.

### Step 1: Read the task/plan file(s)

Resolve input to absolute file paths:

- **Single file** — use as-is
- **Multiple files** — expand glob with Glob tool
- **Folder** — Glob `folder/*.md`, exclude non-task files (README, etc.)

Read every file completely. Understand:
- What tasks are defined
- Expected file outputs
- Acceptance criteria
- Dependencies

Report: "Read N tasks from M files. Analyzing against current codebase..."

### Step 2: Analyze Codebase State

Analyze current codebase to determine what's implemented vs. what was planned. Use **up to 3 parallel subagents**, splitting scope based on the project's actual structure.

**Determine split dynamically:** Inspect the task files for module/layer groupings. Common patterns:

| Pattern | Subagent Split |
|---------|---------------|
| Backend / Frontend / Shared | Split by layer |
| Core / CLI / Plugins | Split by module |
| API / Workers / Config | Split by concern |
| No clear split | Divide task list evenly (1/3 each) |

Adapt the subagent assignments to whatever structure the tasks describe. Each subagent receives:
- List of tasks from Step 1
- Their specific scope
- Instructions to check each task's file paths and verify existence, completeness

**Verification approach per file:**

For each file referenced in tasks:
1. **Does file exist?** — Check file path
2. **Is it complete?** — Read content, check for stubbed code (e.g., `// TODO`, `pass`, `return null`, `raise NotImplementedError`, empty function bodies)
3. **Does it match spec?** — Compare against acceptance criteria
4. **Are tests present?** — Check for test files if required

### Step 3: Identify Gaps

Collect subagent findings. Categorize each task as:

| Status | Definition |
|--------|-------------|
| ✅ **Implemented** | All files exist, code complete, matches spec |
| ⚠️ **Partial** | Files exist but incomplete or stubbed |
| ❌ **Missing** | Files don't exist or major parts missing |
| 🔄 **Modified** | Exists but differs from spec (note changes) |

For each gap, document:
- Task ID and title
- Missing files/features
- What acceptance criteria not met
- Suggested fix

### Step 4: Write Verification Report

Write comprehensive report in **same directory as first provided file**. Name: `{original-filename}-verification.md`.

**Template:**

```markdown
# Implementation Verification Report

**Source:** `{relative-path-to-task-file}`
**Verified:** {ISO timestamp}
**Total Tasks:** {N}
**Implemented:** {X} ({X/N}%)
**Partial:** {Y}
**Missing:** {Z}

---

## Summary

| Status | Count | Percentage |
|--------|-------|------------|
| ✅ Implemented | {N} | {X/N}% |
| ⚠️ Partial | {Y} | {Y/N}% |
| ❌ Missing | {Z} | {Z/N}% |
| 🔄 Modified | {W} | {W/N}% |

---

## Task-by-Task Results

### ✅ Implemented Tasks

| Task ID | Title | Files |
|---------|-------|-------|
| T1 | Title | path/to/file |

### ⚠️ Partial Tasks

| Task ID | Title | Missing | Notes |
|--------|-------|---------|-------|
| T2 | Title | path/to/test | Tests not written |

### ❌ Missing Tasks

| Task ID | Title | Missing Files/Features |
|--------|-------|----------------------|
| T3 | Title | path/to/expected/file |

### 🔄 Modified Tasks

| Task ID | Title | Changes |
|--------|-------|---------|
| T4 | Title | Changed signature |

---

## Detailed Gap Analysis

### {Module/Layer 1} Gaps

...

### {Module/Layer 2} Gaps

...

### {Module/Layer 3} Gaps

...

---

## Recommendations

1. Priority fixes for missing tasks
2. Suggestions for partial implementations
3. Items needing review

---

## Quick Reference: Task Status

```
T1: ✅ Implemented
T2: ⚠️ Partial (missing tests)
T3: ❌ Missing (file not created)
...
```
```

---

## Error Handling

- **Can't read file** — Ask user to verify path
- **No tasks parsed** — Report "No task headings found." Check markdown structure
- **Empty codebase** — Report all tasks as missing (expected for early-stage verification)
- **Subagent failures** — Retry individually, report which subagent failed

## Important Rules

1. **Use parallel subagents** — Up to 3 for speed and context efficiency
2. **Report alongside source** — Same directory as first provided file
3. **Be comprehensive** — Include every task, no omissions
4. **Document gaps clearly** — What's missing, where, what to fix
5. **Verify actual implementation** — Not just file existence, check completeness against acceptance criteria
6. **Adapt to project structure** — Split subagent work by whatever modules/layers the tasks define, not by assumed stack
