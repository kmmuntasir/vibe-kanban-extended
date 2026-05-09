# Auth API (`/api/auth`)

Authentication: methods, login, logout, handoff, token management.

---

## Endpoints

### List Auth Methods
```
GET /api/auth/methods
```
Response: available authentication methods for this server instance.

### OAuth Handoff Init
```
POST /api/auth/handoff/init
```
Body: `{ "provider": "github" }`
Starts OAuth flow. Returns URL to redirect user to.

### OAuth Handoff Complete
```
GET /api/auth/handoff/complete?code=<code>&state=<state>
```
Completes OAuth handshake. Returns HTML page.

### Local Login
```
POST /api/auth/local/login
```
Body: `{ "email": "user@example.com", "password": "..." }`
Response: user profile.

### Logout
```
POST /api/auth/logout
```
Returns 204 No Content. Clears session.

### Get Auth Status
```
GET /api/auth/status
```
Response: current authentication state.

### Get Auth Token
```
GET /api/auth/token
```
Response: `{ "token": "..." }` — current session token.

### Get Current User
```
GET /api/auth/user
```
Response:
```json
{
  "id": "uuid",
  "email": "user@example.com",
  "first_name": "string?",
  "last_name": "string?",
  "username": "string?",
  "avatar_url": "string?"
}
```
