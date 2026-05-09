# Organizations API (`/api/organizations`)

Organization management, members, and invitations.

---

## Organization object
```json
{
  "id": "uuid",
  "name": "My Org",
  "slug": "my-org",
  "is_personal": false,
  "issue_prefix": "ORG",
  "created_at": "2026-04-08T11:31:53Z",
  "updated_at": "2026-04-22T21:07:11Z"
}
```
When listing, each org also includes `user_role` (e.g., `"ADMIN"`).

---

## Organizations

### List Organizations
```
GET /api/organizations
```
Response: `{"organizations": [OrganizationWithRole]}`

### Create Organization
```
POST /api/organizations
```
Body: `{ "name": "string", "slug": "optional-string" }`

### Get Organization
```
GET /api/organizations/{org_id}
```
Response: `{"organization": Organization, "user_role": "ADMIN"}`

### Update Organization
```
PATCH /api/organizations/{org_id}
```
Body: `{ "name": "New Name" }`

### Delete Organization
```
DELETE /api/organizations/{org_id}
```
Returns 204 No Content.

---

## Members

Member roles: `"ADMIN"`, `"MEMBER"`

### List Members
```
GET /api/organizations/{org_id}/members
```
Response:
```json
{
  "members": [{
    "user_id": "uuid",
    "role": "ADMIN",
    "joined_at": "...",
    "first_name": "string?",
    "last_name": "string?",
    "username": "string?",
    "email": "string?",
    "avatar_url": "string?"
  }]
}
```

### Update Member Role
```
PATCH /api/organizations/{org_id}/members/{user_id}/role
```
Body: `{ "role": "ADMIN" }`

### Remove Member
```
DELETE /api/organizations/{org_id}/members/{user_id}
```
Returns 204 No Content.

---

## Invitations

Invitation status: `"pending"`, `"accepted"`, `"declined"`, `"expired"`

### List Invitations
```
GET /api/organizations/{org_id}/invitations
```
Response: `{"invitations": [Invitation]}` — each has `id`, `organization_id`, `email`, `role`, `status`, `token`, `expires_at`

### Create Invitation
```
POST /api/organizations/{org_id}/invitations
```
Body: `{ "email": "user@example.com", "role": "MEMBER" }`

### Revoke Invitation
```
POST /api/organizations/{org_id}/invitations/revoke
```
Body: `{ "invitation_id": "uuid" }`
Returns 204 No Content.

### Get Invitation (by token, no org context)
```
GET /api/invitations/{token}
```
Response: `{ "id": "uuid", "organization_slug": "slug", "role": "MEMBER", "expires_at": "..." }`

### Accept Invitation (by token)
```
POST /api/invitations/{token}/accept
```
Response: `{ "organization_id": "uuid", "organization_slug": "slug", "role": "MEMBER" }`
