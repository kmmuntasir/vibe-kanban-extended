# Vibe Kanban API Documentation

Base URL: `http://{host}:{port}` (default: `http://127.0.0.1:45679`)

## Conventions

- All API routes are prefixed with `/api`
- Request/response bodies are JSON unless noted
- Response envelope: `{"success": true, "data": ...}` or `{"success": false, "error": "..."}`
- Mutation responses are wrapped in `MutationResponse`: `{"data": <T>}`
- UUIDs use standard v4 format
- Dates are ISO 8601 with UTC timezone (`2026-05-09T01:13:14.151336Z`)

## API Groups

| Group | Prefix | Description |
|-------|--------|-------------|
| [Kanban Board](kanban-board.md) | `/api/remote/v1` | Local kanban board: projects, issues, statuses, tags, assignees |
| [Remote API](remote-api.md) | `/api/remote` | Remote-proxied issues, projects, tags, PRs |
| [Organizations](organizations.md) | `/api/organizations` | Organizations, members, invitations |
| [Repositories](repos.md) | `/api/repos` | Repository registration, branches, PRs, search |
| [Workspaces](workspaces.md) | `/api/workspaces` | Workspace CRUD, git operations, execution, attachments |
| [Sessions](sessions.md) | `/api/sessions` | Session management, follow-up prompts |
| [Auth](auth.md) | `/api/auth` | Authentication: login, logout, handoff, token |
| [Config & Info](config.md) | `/api/config`, `/api/info` | App config, profiles, agent discovery |
| [Tags](tags.md) | `/api/tags` | Global tag management |
| [Attachments](attachments.md) | `/api/attachments` | File upload and download |
| [Other Endpoints](other.md) | various | Health, events, search, filesystem, terminal, SSH, WebRTC |

## Authentication

Most endpoints require authentication. The auth flow:

1. `GET /api/auth/methods` — discover available auth methods
2. Choose method: OAuth handoff (`/api/auth/handoff/init`) or local login (`/api/auth/local/login`)
3. `GET /api/auth/token` — retrieve session token
4. Include token in subsequent requests
