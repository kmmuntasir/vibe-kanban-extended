# Kanban Board API (`/api/remote/v1`)

Local kanban board: projects, statuses, issues, tags, assignees, relationships.

---

## Conventions

- All responses wrapped: `{"success": true, "data": ...}` or `{"success": false, "error": "..."}`
- Mutations return: `{"data": <resource>}` (MutationResponse)
- Deletes return: `{"data": {"id": <uuid>}}` (DeleteResponse)

---

## Projects

### List Projects
```
GET /api/remote/v1/projects?organization_id=<uuid>
```
Response: `ListProjectsResponse`
```json
{
  "success": true,
  "data": { "projects": [{ "id": "uuid", "organization_id": "uuid", "name": "string", "color": "string", "sort_order": 0, "created_at": "...", "updated_at": "..." }] }
}
```

### Get Project
```
GET /api/remote/v1/projects/{project_id}
```
Response: `Project`

### Create Project
```
POST /api/remote/v1/projects
```
Body:
```json
{ "id": "optional-uuid", "organization_id": "uuid", "name": "string", "color": "262 52% 47%" }
```
Response: `MutationResponse<Project>`

### Update Project
```
PATCH /api/remote/v1/projects/{project_id}
```
Body (all fields optional):
```json
{ "name": "string?", "color": "string?", "sort_order": 0 }
```

### Bulk Update Projects
```
POST /api/remote/v1/projects/bulk
```
Body:
```json
{ "updates": [{ "id": "uuid", "name": "string?", "color": "string?", "sort_order": 0 }] }
```
Response: `BulkUpdateProjectsResponse`

### Delete Project
```
DELETE /api/remote/v1/projects/{project_id}
```

### Fallback: List Projects
```
GET /api/remote/v1/fallback/projects?organization_id=<uuid>
```
Returns cached/fallback data when remote is unavailable.

---

## Project Statuses

### List Statuses
```
GET /api/remote/v1/project_statuses?project_id=<uuid>
```
Response: `ListProjectStatusesResponse`
```json
{
  "success": true,
  "data": {
    "project_statuses": [
      { "id": "uuid", "project_id": "uuid", "name": "To do", "color": "217 91% 60%", "sort_order": 1, "hidden": false, "created_at": "..." }
    ]
  }
}
```

### Create Status
```
POST /api/remote/v1/project_statuses
```
Body:
```json
{ "id": "optional-uuid", "project_id": "uuid", "name": "In progress", "color": "38 92% 50%", "sort_order": 2, "hidden": false }
```

### Update Status
```
PATCH /api/remote/v1/project_statuses/{status_id}
```
Body (all fields optional): `{ "name": "string?", "color": "string?", "sort_order": 0?, "hidden": false? }`

### Bulk Update Statuses
```
POST /api/remote/v1/project_statuses/bulk
```
Body:
```json
{ "updates": [{ "id": "uuid", "name": "string?", "color": "string?", "sort_order": 0?, "hidden": false? }] }
```

### Delete Status
```
DELETE /api/remote/v1/project_statuses/{status_id}
```

### Fallback: List Statuses
```
GET /api/remote/v1/fallback/project_statuses?project_id=<uuid>
```

---

## Tags

### List Tags
```
GET /api/remote/v1/tags?project_id=<uuid>
```
Response: `ListTagsResponse` — each tag has: `id`, `project_id`, `name`, `color`

### Create Tag
```
POST /api/remote/v1/tags
```
Body:
```json
{ "id": "optional-uuid", "project_id": "uuid", "name": "bug", "color": "0 84% 60%" }
```

### Update Tag
```
PATCH /api/remote/v1/tags/{tag_id}
```
Body (all fields optional): `{ "name": "string?", "color": "string?" }`

### Delete Tag
```
DELETE /api/remote/v1/tags/{tag_id}
```

### Fallback: List Tags
```
GET /api/remote/v1/fallback/tags?project_id=<uuid>
```

---

## Issues

Issue priorities: `"urgent"`, `"high"`, `"medium"`, `"low"`

#### `Issue` object
| Field | Type | Required |
|-------|------|----------|
| `id` | UUID | yes |
| `project_id` | UUID | yes |
| `issue_number` | i32 | yes |
| `simple_id` | string | yes (e.g., `"KMM-178"`) |
| `status_id` | UUID | yes |
| `title` | string | yes |
| `description` | string? | no |
| `priority` | priority? | no |
| `start_date` | ISO 8601? | no |
| `target_date` | ISO 8601? | no |
| `completed_at` | ISO 8601? | no |
| `sort_order` | f64 | yes |
| `parent_issue_id` | UUID? | no |
| `extension_metadata` | JSON value | yes |
| `creator_user_id` | UUID? | no |
| `created_at` | ISO 8601 | yes |
| `updated_at` | ISO 8601 | yes |

### List Issues
```
GET /api/remote/v1/issues?project_id=<uuid>
```
Query params (all optional except `project_id`): `status_id`, `priority`, `parent_issue_id`, `search`, `simple_id`, `assignee_user_id`, `tag_id`, `tag_name`, `sort_field` (`sort_order`/`priority`/`created_at`/`updated_at`/`title`), `sort_direction` (`asc`/`desc`), `limit`, `offset`

Response:
```json
{
  "success": true,
  "data": {
    "issues": [...],
    "total_count": 10,
    "limit": 50,
    "offset": 0
  }
}
```

### Search Issues
```
POST /api/remote/v1/issues/search
```
Body: same filter fields as query params above (all optional including `status_ids: [UUID]`, `tag_ids: [UUID]`)

### Get Issue
```
GET /api/remote/v1/issues/{issue_id}
```

### Create Issue
```
POST /api/remote/v1/issues
```
Body:
```json
{
  "id": "optional-uuid",
  "project_id": "uuid",
  "status_id": "uuid",
  "title": "string",
  "description": "string?",
  "priority": "high?",
  "start_date": "2026-05-09T00:00:00Z?",
  "target_date": "2026-05-16T00:00:00Z?",
  "completed_at": null,
  "sort_order": 0.0,
  "parent_issue_id": null,
  "parent_issue_sort_order": null,
  "extension_metadata": null
}
```
Note: `sort_order` and `extension_metadata` are always required. `extension_metadata` can be `null`.

### Update Issue
```
PATCH /api/remote/v1/issues/{issue_id}
```
Body (all fields optional, top-level `None` = skip field):
```json
{
  "status_id": "uuid?",
  "title": "string?",
  "description": "string? (None=no change, Some(null)=clear, Some(\"v\")=set)",
  "priority": "high?",
  "sort_order": 0.0?,
  "parent_issue_id": "uuid?",
  "extension_metadata": {}
}
```
Double-option fields (`description`, `priority`, `start_date`, `target_date`, `completed_at`, `parent_issue_id`):
- Omit key = no change
- `null` = clear value
- `"value"` = set value

### Delete Issue
```
DELETE /api/remote/v1/issues/{issue_id}
```

### Bulk Update Issues
```
POST /api/remote/v1/issues/bulk
```
Body:
```json
{
  "updates": [{ "id": "uuid", "status_id": "uuid?" }]
}
```
Response: `BulkUpdateIssuesResponse` — `{"data": [...updated issues...], "txid": 0}`

### Fallback: List Issues
```
GET /api/remote/v1/fallback/issues?project_id=<uuid>
```

---

## Issue Assignees

### List Assignees
```
GET /api/remote/v1/issue_assignees?issue_id=<uuid>
```
Response: `ListIssueAssigneesResponse` — each: `id`, `issue_id`, `user_id`, `assigned_at`

### Assign User
```
POST /api/remote/v1/issue_assignees
```
Body:
```json
{ "id": "optional-uuid", "issue_id": "uuid", "user_id": "uuid" }
```

### Remove Assignee
```
DELETE /api/remote/v1/issue_assignees/{assignee_id}
```

### Fallback: List Assignees
```
GET /api/remote/v1/fallback/issue_assignees?project_id=<uuid>
```
(Returns all assignees across project)

---

## Issue Relationships

Relationship types: `"blocking"`, `"related"`, `"has_duplicate"`

### List Relationships
```
GET /api/remote/v1/issue_relationships?issue_id=<uuid>
```
Response: `ListIssueRelationshipsResponse` — each: `id`, `issue_id`, `related_issue_id`, `relationship_type`, `created_at`

### Create Relationship
```
POST /api/remote/v1/issue_relationships
```
Body:
```json
{ "id": "optional-uuid", "issue_id": "uuid", "related_issue_id": "uuid", "relationship_type": "blocking" }
```

### Delete Relationship
```
DELETE /api/remote/v1/issue_relationships/{relationship_id}
```

### Fallback: List Relationships
```
GET /api/remote/v1/fallback/issue_relationships?project_id=<uuid>
```

---

## Issue Tags

### List Issue Tags
```
GET /api/remote/v1/issue_tags?issue_id=<uuid>
```
Response: `ListIssueTagsResponse` — each: `id`, `issue_id`, `tag_id`

### Add Tag to Issue
```
POST /api/remote/v1/issue_tags
```
Body:
```json
{ "id": "optional-uuid", "issue_id": "uuid", "tag_id": "uuid" }
```

### Remove Tag from Issue
```
DELETE /api/remote/v1/issue_tags/{issue_tag_id}
```

### Fallback: List Issue Tags
```
GET /api/remote/v1/fallback/issue_tags?project_id=<uuid>
```
