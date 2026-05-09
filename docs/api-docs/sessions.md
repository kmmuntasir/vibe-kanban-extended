# Sessions API (`/api/sessions`)

Session management. A session belongs to a workspace and contains execution processes (coding agent runs).

---

## Session object
```json
{
  "id": "uuid",
  "workspace_id": "uuid",
  "name": "optional-name",
  "executor": "CLAUDE_CODE",
  "agent_working_dir": "optional-dir",
  "created_at": "...",
  "updated_at": "..."
}
```

---

## Endpoints

### List Sessions
```
GET /api/sessions?workspace_id=<uuid>
```
Query (optional): `workspace_id` — filters sessions to a workspace.

### Create Session
```
POST /api/sessions
```
Body:
```json
{
  "workspace_id": "uuid",
  "executor": "CLAUDE_CODE",
  "name": "optional-name"
}
```

### Get Session
```
GET /api/sessions/{session_id}
```

### Update Session
```
PUT /api/sessions/{session_id}
```
Body: `{ "name": "New Name" }`

### Follow-Up (Run Coding Agent Turn)
```
POST /api/sessions/{session_id}/follow-up
```
Body:
```json
{
  "prompt": "What to do next",
  "executor_config": { "executor": "CLAUDE_CODE" }
}
```
Response: `ExecutionProcess`

### Reset Session
```
POST /api/sessions/{session_id}/reset
```

### Run Setup Script
```
POST /api/sessions/{session_id}/setup
```
Response: `ExecutionProcess`

### Start Review
```
POST /api/sessions/{session_id}/review
```
Body: `{ "scope": "full" }`

---

## Session Queue

### Get Queue
```
GET /api/sessions/{session_id}/queue
```

### Push to Queue
```
POST /api/sessions/{session_id}/queue
```
Body: `{ "messages": ["prompt 1", "prompt 2"] }`

### Clear Queue
```
DELETE /api/sessions/{session_id}/queue
```

---

## Execution Processes

### Get Execution
```
GET /api/execution-processes/{ep_id}
```
Response: `ExecutionProcess`
```json
{
  "id": "uuid",
  "session_id": "uuid",
  "run_reason": "CodingAgent",
  "executor_action": { ... },
  "status": "Running",
  "exit_code": null,
  "dropped": false,
  "started_at": "...",
  "completed_at": null,
  "created_at": "...",
  "updated_at": "..."
}
```
Status values: `"Running"`, `"Completed"`, `"Failed"`, `"Killed"`
Run reason values: `"SetupScript"`, `"CleanupScript"`, `"ArchiveScript"`, `"CodingAgent"`, `"DevServer"`

### Stop Execution
```
POST /api/execution-processes/{ep_id}/stop
```

### Get Repo States (for execution)
```
GET /api/execution-processes/{ep_id}/repo-states
```

### Raw Logs Stream (WebSocket)
```
GET /api/execution-processes/{ep_id}/raw-logs/ws
```

### Normalized Logs Stream (WebSocket)
```
GET /api/execution-processes/{ep_id}/normalized-logs/ws
```

### Session Execution Stream (WebSocket)
```
GET /api/execution-processes/stream/session/ws?session_id=<uuid>
```
