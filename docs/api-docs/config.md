# Config & Info API (`/api/config`, `/api/info`)

App configuration, profiles, editor/agent availability, MCP server config.

---

## System Info

### Get System Info
```
GET /api/info
```
Response: `UserSystemInfo` — OS, architecture, installed tools, etc.

---

## Config

### Get Config
```
GET /api/config
```

### Update Config
```
PUT /api/config
```
Body: `Config` object (full config).

---

## Profiles

### Get Profiles
```
GET /api/profiles
```
Response: executor profiles configuration.

### Update Profiles
```
PUT /api/profiles
```
Body: raw JSON string of profiles config.

---

## Sounds

### Get Sound File
```
GET /api/sounds/{sound}
```
Response: WAV audio binary.

---

## MCP Config

### Get MCP Config
```
GET /api/mcp-config
```
Query: `McpServerQuery`

### Update MCP Config
```
POST /api/mcp-config
```
Body: `UpdateMcpServersBody`

---

## Editor & Agent Discovery

### Check Editor Availability
```
GET /api/editors/check-availability?editor_type=<vscode|vim|...>
```

### Check Agent Availability
```
GET /api/agents/check-availability?agent=<CLAUDE_CODE|...>
```

### Get Agent Preset Options
```
GET /api/agents/preset-options?agent=<name>
```

### Agent Discovery Stream (WebSocket)
```
GET /api/agents/discovered-options/ws?agent=<name>
```
WebSocket — streams JSON patches as agent options are discovered.
