# Workspaces API (`/api/workspaces`)

Workspace CRUD, git operations, execution, PRs, integration, attachments, links.

---

## Workspace object
```json
{
  "id": "uuid",
  "task_id": "uuid?",
  "container_ref": "string?",
  "branch": "vk/b7a4-nano-quiz-update",
  "setup_completed_at": "2026-05-09T01:16:32Z?",
  "created_at": "2026-05-09T01:16:32Z",
  "updated_at": "2026-05-09T01:16:32Z",
  "archived": false,
  "pinned": false,
  "name": "nano-quiz: Update CLAUDE.md",
  "worktree_deleted": false
}
```

---

## Core Workspace

### List Workspaces
```
GET /api/workspaces
```
Query (all optional): `archived`, `pinned`, `branch`, `name_search`, `limit`, `offset`

### Create Workspace
```
POST /api/workspaces
```
Body: `{ "name": "optional-name" }`

### Start Workspace (Create + Launch First Session)
```
POST /api/workspaces/start
```
Body:
```json
{
  "name": "optional-name",
  "repos": [{ "repo_id": "uuid", "target_branch": "main" }],
  "linked_issue": { "remote_project_id": "uuid", "issue_id": "uuid" },
  "executor_config": {
    "executor": "CLAUDE_CODE",
    "variant": "optional-variant",
    "model_id": "optional-model"
  },
  "prompt": "Task description for the coding agent",
  "attachment_ids": ["uuid?"]
}
```
`executor` values: `"CLAUDE_CODE"`, `"AMP"`, `"GEMINI"`, `"CODEX"`, `"OPENCODE"`, `"CURSOR_AGENT"`, `"QWEN_CODE"`, `"COPILOT"`, `"DROID"`

Response: `{"workspace": Workspace, "execution_process": ExecutionProcess}`

### Create Workspace from PR
```
POST /api/workspaces/from-pr
```

### Workspace Summaries
```
POST /api/workspaces/summaries
```
Body: `{ "workspace_ids": ["uuid"] }`

### Workspace Streams (WebSocket)
```
GET /api/workspaces/streams/ws?branch=<name>&...
```
WebSocket upgrade — streams workspace events.

### Get Workspace
```
GET /api/workspaces/{id}
```

### Update Workspace
```
PUT /api/workspaces/{id}
```
Body (all optional): `{ "archived": false, "pinned": true, "name": "New Name" }`

### Delete Workspace
```
DELETE /api/workspaces/{id}?delete_remote=true&delete_branches=false
```

### Mark Seen
```
PUT /api/workspaces/{id}/seen
```

### Get First Message
```
GET /api/workspaces/{id}/messages/first
```

---

## Workspace Git

### Git Status
```
GET /api/workspaces/{id}/git/status
```
Response: repo branch status per workspace repo

### Git Diff Stream (WebSocket)
```
GET /api/workspaces/{id}/git/diff/ws
```
WebSocket streaming diff output.

### Merge
```
POST /api/workspaces/{id}/git/merge
```

### Push
```
POST /api/workspaces/{id}/git/push
```

### Force Push
```
POST /api/workspaces/{id}/git/push/force
```

### Rebase
```
POST /api/workspaces/{id}/git/rebase
```

### Continue Rebase
```
POST /api/workspaces/{id}/git/rebase/continue
```

### Abort Conflicts
```
POST /api/workspaces/{id}/git/conflicts/abort
```

### Change Target Branch
```
PUT /api/workspaces/{id}/git/target-branch
```
Body: `{ "new_target_branch": "develop" }`

### Rename Branch
```
PUT /api/workspaces/{id}/git/branch
```

---

## Workspace Execution

### Start Dev Server
```
POST /api/workspaces/{id}/execution/dev-server/start
```

### Run Cleanup Script
```
POST /api/workspaces/{id}/execution/cleanup
```

### Run Archive Script
```
POST /api/workspaces/{id}/execution/archive
```

### Stop Execution
```
POST /api/workspaces/{id}/execution/stop
```

---

## Workspace Pull Requests

### Create PR
```
POST /api/workspaces/{id}/pull-requests
```
Body: `{ "title": "string", "body": "string?" }`

### Attach Existing PR
```
POST /api/workspaces/{id}/pull-requests/attach
```
Body: `{ "url": "https://github.com/...", "number": 42 }`

### Get PR Comments
```
GET /api/workspaces/{id}/pull-requests/comments?pr_number=<n>
```

---

## Workspace Integration

### Get Editor Path
```
GET /api/workspaces/{id}/integration/editor/path?repo_path=<path>
```

### Open in Editor
```
POST /api/workspaces/{id}/integration/editor/open
```

### Run Agent Setup
```
POST /api/workspaces/{id}/integration/agent/setup
```

### Setup GitHub CLI
```
POST /api/workspaces/{id}/integration/github/cli/setup
```

---

## Workspace Repos

### List Workspace Repos
```
GET /api/workspaces/{id}/repos
```
Response: `[{ "repo": Repo, "target_branch": "main", ... }]`

### Add Repo to Workspace
```
POST /api/workspaces/{id}/repos
```
Body: `{ "repo_id": "uuid", "target_branch": "main" }`

---

## Workspace Attachments

### List Attachments
```
GET /api/workspaces/{id}/attachments
```

### Associate Attachments
```
POST /api/workspaces/{id}/attachments/associate
```

### Import Issue Attachments
```
POST /api/workspaces/{id}/attachments/import-issue-attachments
```
Body: `{ "issue_id": "uuid" }`

### Get Attachment Metadata
```
GET /api/workspaces/{id}/attachments/metadata?attachment_id=<uuid>
```

### Upload Attachment
```
POST /api/workspaces/{id}/attachments/upload
```
Multipart form upload.

### Download Attachment
```
GET /api/workspaces/{id}/attachments/file/{*path}
```
Query: `session_id=<uuid>` (for session-scoped access)

---

## Workspace Links

### Link Workspace to Remote Issue
```
POST /api/workspaces/{id}/links
```
Body:
```json
{ "project_id": "uuid", "issue_id": "uuid" }
```

### Unlink Workspace
```
DELETE /api/workspaces/{id}/links
```
