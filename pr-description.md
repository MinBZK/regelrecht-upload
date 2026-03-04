# Uploader Self-Service Access via Slug + Email

## Summary
- Uploaders can log in with slug + email to add documents after submission
- Privacy-focused dashboard (no name/org displayed)
- Fix: rr-button.js duplicate event listeners

## New Endpoints
- `POST /api/uploader/login` - Authenticate with slug + email
- `GET /api/uploader/me` - Get session info
- `POST /api/uploader/logout` - End session

## Security
- 4-hour sessions, SHA-256 hashed tokens, rate limiting (10/hour)
- Cookies: `HttpOnly; SameSite=Strict; Secure`

## Test plan
- [ ] Log in with slug + email → redirects to dashboard
- [ ] Upload/delete documents from dashboard
- [ ] Wrong email → error message
- [ ] 11 failed logins → rate limited
- [ ] Status page shows "Documenten toevoegen" for non-draft submissions with email

🤖 Generated with [Claude Code](https://claude.ai/code)
