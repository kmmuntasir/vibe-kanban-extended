# Attachments API (`/api/attachments`)

File upload and download. Workspace-specific attachment routes also exist (see [Workspaces](workspaces.md#workspace-attachments)).

---

## Global Attachment Routes

### Upload Attachment
```
POST /api/attachments/upload
```
Multipart form upload. File goes to global attachment storage.

Response:
```json
{
  "success": true,
  "data": { "id": "uuid", "filename": "file.png", "size": 12345, ... }
}
```

### Download Attachment
```
GET /api/attachments/{id}/file
```
Response: binary file stream.

### Delete Attachment
```
DELETE /api/attachments/{id}
```

---

## Workspace Attachment Routes

See [Workspaces API](workspaces.md#workspace-attachments) for:
- `GET /api/workspaces/{id}/attachments` — list
- `POST /api/workspaces/{id}/attachments/upload` — upload scoped to workspace
- `GET /api/workspaces/{id}/attachments/file/{*path}` — download
- `POST /api/workspaces/{id}/attachments/associate` — associate existing attachments
- `POST /api/workspaces/{id}/attachments/import-issue-attachments` — import from issue
