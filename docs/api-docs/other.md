# Other Endpoints

Miscellaneous API endpoints.

---

## Health

```
GET /api/health
```
Response: `"OK"` — used for health checks and uptime monitoring.

---

## Events (SSE)

```
GET /api/events
```
Server-Sent Events stream. Pushes real-time events to the frontend.

---

## Search (Global)

```
GET /api/search?q=<query>&repo_ids=<uuid,uuid>
```
Searches across multiple repos.

Response: `[SearchResult]`
```json
[{ "path": "src/main.rs", "is_file": true, "match_type": "FileName", "score": 0 }]
```

---

## Filesystem

### List Directory
```
GET /api/filesystem/directory?path=<absolute-path>
```

### List Git Repos in Directory
```
GET /api/filesystem/git-repos?path=<absolute-path>
```
Response: list of `DirectoryEntry` — finds all git repositories under a path.

---

## Preview Proxy

### Proxy to Dev Server
```
ANY /api/preview/{target_port}
ANY /api/preview/{target_port}/{*tail}
```
Proxies HTTP and WebSocket connections to a workspace's dev server on the given port.

### Subdomain Proxy
```
ANY *.subdomain
```
Proxies requests with subdomain matching to the corresponding workspace's dev server.

---

## Terminal

### Terminal WebSocket
```
GET /api/terminal/ws?container_ref=<ref>&path=<dir>
```
WebSocket upgrade — provides PTY terminal to the workspace container.

---

## SSH Session

### SSH WebSocket
```
GET /api/ssh-session
```
WebSocket upgrade — SSH session into the host machine.

---

## WebRTC

### Send Offer
```
POST /api/webrtc/offer
```
Body: `{ "sdp": "..." }`
Response: `{ "sdp": "..." }` — SDP answer.

### Send ICE Candidate
```
POST /api/webrtc/candidate
```
Body: `{ "candidate": "...", "sdp_mid": "...", "sdp_m_line_index": 0 }`
Returns 204 No Content.

---

## Approvals

### Approval Stream (WebSocket)
```
GET /api/approvals/stream/ws
```
WebSocket — streams pending approval requests.

### Respond to Approval
```
POST /api/approvals/{id}/respond
```
Body: `{ "approved": true, "reason": "optional" }`

---

## Releases

### Get Releases
```
GET /api/releases
```
Response: available release versions and changelogs.

---

## Scratch

Per-session scratch storage (key-value for transient data).

### List Scratch Entries
```
GET /api/scratch
```

### Get Scratch
```
GET /api/scratch/{scratch_type}/{id}
```

### Create Scratch
```
POST /api/scratch/{scratch_type}/{id}
```
Body: `{ "value": ... }`

### Update Scratch
```
PUT /api/scratch/{scratch_type}/{id}
```
Body: `{ "value": ... }`

### Delete Scratch
```
DELETE /api/scratch/{scratch_type}/{id}
```

### Scratch Stream (WebSocket)
```
GET /api/scratch/{scratch_type}/{id}/stream/ws
```
WebSocket — streams live updates to scratch data.

---

## Containers

### Get Container Info
```
GET /api/containers/info?ref=<container_ref>
```
Response: container/workspace info for a given container reference.

### Get Attempt Context
```
GET /api/containers/attempt-context?ref=<container_ref>
```
Response: `WorkspaceContext` — full workspace + repo context for the current coding attempt.

---

## Relay Auth

### Client Pairing
```
POST /api/relay-auth/client/pair
```
Body: `{ "enrollment_code": "..." }`

### List Paired Hosts
```
GET /api/relay-auth/client/hosts
```

### Remove Paired Host
```
DELETE /api/relay-auth/client/hosts/{host_id}
```

### Server: Generate Enrollment Code
```
POST /api/relay-auth/server/enrollment-code
```

### Server: List Paired Clients
```
GET /api/relay-auth/server/clients
```

### Server: Remove Paired Client
```
DELETE /api/relay-auth/server/clients/{client_id}
```

### Server: SPAKE2 Start
```
POST /api/relay-auth/server/spake2/start
```

### Server: SPAKE2 Finish
```
POST /api/relay-auth/server/spake2/finish
```

### Server: Refresh Signing Session
```
POST /api/relay-auth/server/signing-session/refresh
```

---

## Host Relay Proxy

```
ANY /api/host/{host_id}/{*tail}
```
Proxies requests to relay-connected host instances.

---

## Open Remote Editor

```
POST /api/open-remote-editor/workspace
```
Body: `{ "workspace_id": "uuid", "path": "src/main.rs" }`
Opens a remote workspace in the local editor.

---

## Web Frontend

Routes not under `/api`:

- `GET /` — serves embedded `index.html` (SPA)
- `GET /{*path}` — serves embedded static assets (JS/CSS/images) or falls back to `index.html`
