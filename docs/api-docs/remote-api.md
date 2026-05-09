# Remote API (`/api/remote`)

Remote-proxied issues, projects, tags, pull requests. Similar interface to Kanban Board API but with fewer endpoints (no bulk operations, no fallbacks).

---

## Remote Issues

Same Issue schema as [Kanban Board](kanban-board.md).

### List Issues
```
GET /api/remote/issues?project_id=<uuid>&status=<name>&priority=<name>&search=<text>&sort_field=<field>&sort_direction=<asc|desc>&limit=<n>&offset=<n>
```

### Search Issues
```
POST /api/remote/issues/search
```
Body:
```json
{
  "project_id": "uuid",
  "status_id": "uuid?",
  "status_ids": ["uuid?"],
  "priority": "high?",
  "parent_issue_id": "uuid?",
  "search": "string?",
  "simple_id": "string?",
  "assignee_user_id": "uuid?",
  "tag_id": "uuid?",
  "tag_ids": ["uuid?"],
  "sort_field": "priority?",
  "sort_direction": "asc?",
  "limit": 50,
  "offset": 0
}
```

### Get Issue
```
GET /api/remote/issues/{issue_id}
```

### Create Issue
```
POST /api/remote/issues
```
Body: Same as [Kanban Create Issue](kanban-board.md#create-issue)

### Update Issue
```
PATCH /api/remote/issues/{issue_id}
```
Body: Same as [Kanban Update Issue](kanban-board.md#update-issue)

### Delete Issue
```
DELETE /api/remote/issues/{issue_id}
```

---

## Remote Projects

### List Projects
```
GET /api/remote/projects?organization_id=<uuid>
```

### Get Project
```
GET /api/remote/projects/{project_id}
```

---

## Remote Project Statuses

### List Statuses
```
GET /api/remote/project-statuses?project_id=<uuid>
```

---

## Remote Tags

### List Tags
```
GET /api/remote/tags?project_id=<uuid>
```

### Get Tag
```
GET /api/remote/tags/{tag_id}
```

---

## Remote Issue Assignees

### List Assignees
```
GET /api/remote/issue-assignees?issue_id=<uuid>
```

### Assign User
```
POST /api/remote/issue-assignees
```
Body: `{ "id": "optional-uuid", "issue_id": "uuid", "user_id": "uuid" }`

### Get Assignee
```
GET /api/remote/issue-assignees/{issue_assignee_id}
```

### Remove Assignee
```
DELETE /api/remote/issue-assignees/{issue_assignee_id}
```

---

## Remote Issue Relationships

### List Relationships
```
GET /api/remote/issue-relationships?issue_id=<uuid>
```

### Create Relationship
```
POST /api/remote/issue-relationships
```
Body: `{ "id": "optional-uuid", "issue_id": "uuid", "related_issue_id": "uuid", "relationship_type": "blocking" }`

### Delete Relationship
```
DELETE /api/remote/issue-relationships/{relationship_id}
```

---

## Remote Issue Tags

### List Issue Tags
```
GET /api/remote/issue-tags?issue_id=<uuid>
```

### Add Tag to Issue
```
POST /api/remote/issue-tags
```
Body: `{ "id": "optional-uuid", "issue_id": "uuid", "tag_id": "uuid" }`

### Get Issue Tag
```
GET /api/remote/issue-tags/{issue_tag_id}
```

### Remove Tag from Issue
```
DELETE /api/remote/issue-tags/{issue_tag_id}
```

---

## Remote Workspaces

### Resolve by Local Workspace ID
```
GET /api/remote/workspaces/by-local-id/{local_workspace_id}
```
Returns the remote workspace that corresponds to a local workspace.

---

## Remote Pull Requests

### List PRs for Issue
```
GET /api/remote/pull-requests?issue_id=<uuid>
```
Response:
```json
{
  "success": true,
  "data": {
    "pull_requests": [{
      "id": "uuid",
      "url": "https://github.com/...",
      "number": 42,
      "status": "open",
      "merged_at": null,
      "merge_commit_sha": null,
      "target_branch_name": "main",
      "project_id": "uuid",
      "issue_id": "uuid",
      "workspace_id": "uuid?",
      "created_at": "...",
      "updated_at": "..."
    }]
  }
}
```
PR status values: `"open"`, `"merged"`, `"closed"`

### Link PR to Issue
```
POST /api/remote/pull-requests/link
```
Body:
```json
{
  "id": "optional-uuid",
  "issue_id": "uuid",
  "url": "https://github.com/owner/repo/pull/42",
  "number": 42,
  "status": "open",
  "merged_at": null,
  "merge_commit_sha": null,
  "target_branch_name": "main"
}
```
