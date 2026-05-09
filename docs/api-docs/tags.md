# Tags API (`/api/tags`)

Global tag management (global tags, not project-scoped kanban tags).

---

## Tag object
```json
{
  "id": "uuid",
  "name": "tag-name",
  "color": "262 52% 47%"
}
```

---

## Endpoints

### List Tags
```
GET /api/tags?search=<query>
```
Query (optional): `search` — filter by name.

### Create Tag
```
POST /api/tags
```
Body: `{ "name": "tag-name", "color": "262 52% 47%" }`

### Update Tag
```
PUT /api/tags/{tag_id}
```
Body: `{ "name": "new-name", "color": "0 84% 60%" }`

### Delete Tag
```
DELETE /api/tags/{tag_id}
```
