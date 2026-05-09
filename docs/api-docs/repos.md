# Repositories API (`/api/repos`)

Repository registration, branches, PRs, search.

---

## Repository object
```json
{
  "id": "uuid",
  "path": "/home/user/project",
  "name": "project",
  "display_name": "My Project",
  "setup_script": "pnpm install",
  "cleanup_script": null,
  "archive_script": null,
  "copy_files": null,
  "parallel_setup_script": false,
  "dev_server_script": "pnpm run dev",
  "default_target_branch": "main",
  "default_working_dir": null,
  "created_at": "2026-05-02T17:18:03Z",
  "updated_at": "2026-05-02T17:18:19Z"
}
```

---

## Repo CRUD

### List Repos
```
GET /api/repos
```
Response: `[Repo]`

### Get Repo
```
GET /api/repos/{repo_id}
```

### Register Repo
```
POST /api/repos
```
Registers an existing directory as a repo.

### Init Repo
```
POST /api/repos/init
```
Creates and registers a new git repo.

### Get Recent Repos
```
GET /api/repos/recent
```

### Batch Repo Operations
```
POST /api/repos/batch
```

### Update Repo
```
PUT /api/repos/{repo_id}
```
Body (`UpdateRepo` — all fields optional, double-option):
```json
{
  "display_name": "New Name",
  "setup_script": "pnpm install",
  "cleanup_script": null,
  "dev_server_script": "pnpm dev",
  "default_target_branch": null,
  "default_working_dir": "/subdir",
  "parallel_setup_script": true
}
```
Double-option fields (`display_name`, setup/cleanup/archive/dev_server scripts, `copy_files`, `parallel_setup_script`, `default_target_branch`, `default_working_dir`):
- Omit key = no change
- `null` = clear value
- `"value"` = set value

### Delete Repo
```
DELETE /api/repos/{repo_id}
```

---

## Branches & Remotes

### List Branches
```
GET /api/repos/{repo_id}/branches
```
Response: `[{ "name": "main", ... }]`

### List Remotes
```
GET /api/repos/{repo_id}/remotes
```

---

## Pull Requests

### List Pull Requests for Repo
```
GET /api/repos/{repo_id}/prs
```

### Get PR Info (by number + remote)
```
GET /api/repos/pr-info?repo_id=<uuid>&number=<n>&remote_url=<url>
```
Response: `PullRequestDetail`

---

## Search

### Search Repo
```
GET /api/repos/{repo_id}/search?q=<query>
```
Response:
```json
[{ "path": "src/main.rs", "is_file": true, "match_type": "FileName", "score": 0 }]
```
Match types: `"FileName"`, `"DirectoryName"`, `"FullPath"`

---

## Open Editor

### Open Repo in Editor
```
POST /api/repos/{repo_id}/open-editor
```
Body (optional): `{ "path": "src/main.rs" }`
