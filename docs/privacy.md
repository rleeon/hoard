# Privacy

Hoard is self-hosted: by default, the only network calls the desktop app
makes are to **your** Hoard server. The project itself collects nothing
about you, your saves, or your machine — and the design makes it hard
to ever change that without you noticing.

This page is the precise version of that promise.

---

## What the desktop app sends — by default

The Tauri desktop client makes HTTPS calls to **one place only**: the
server URL you typed during onboarding. Every request carries your
bearer token in the `Authorization` header.

| When | Endpoint on your server | Body |
| --- | --- | --- |
| Onboarding | `GET /health` | empty |
| Onboarding | `POST /v1/auth/whoami` | empty |
| Library scan / detection | local only — **no network** | — |
| Tracking a save | `POST /v1/saves` | `{game_slug, label}` |
| A backup happens | `POST /v1/saves/{id}/snapshots` | streaming `tar.zst` of the save folder |
| Listing snapshots | `GET /v1/saves/{id}/snapshots` | empty |
| Restore | `GET /v1/saves/{id}/snapshots/{n}/download` | empty |
| Soft delete / undelete | `DELETE` / `POST /v1/.../snapshots/{n}` | empty |

The desktop app **never** contacts a Hoard project endpoint, an
analytics provider, a CDN, an ad network, an error reporter, or a
"phone home" health check. There is no global Hoard server. We don't
have access to your data because there is no place for it to land that
isn't your own machine.

---

## Anonymous telemetry — opt-in only

Settings → Privacy has a toggle: **Send anonymous usage pings.**
**It is OFF by default.** It stays off until you flip it.

When (and only when) you turn it on, future versions may POST
**event counters** — never identifiers — to a Hoard-project-controlled
endpoint. The shape we're committed to:

```jsonc
{
  "event": "backup_succeeded" | "restore_completed" | "first_run_done",
  "version": "0.2.0",
  "platform": "linux" | "windows" | "macos",
  // anonymous, randomly generated on first opt-in, never tied to your
  // username or server, regenerated if you toggle the setting off and
  // back on:
  "instance_id": "8e34..."
}
```

We will **never** send:

- your username, email, or any account ID from your server
- your server URL
- game names, save paths, file paths, file contents, or file hashes
- IP-address-resolved geolocation beyond what the standard TLS
  connection inherently exposes
- anything you typed into the app

The telemetry network sender does not exist yet in v0.2 — the toggle
is a no-op so we can ship it without a settings migration when v0.3
adds the actual sender. When we wire it up, we'll cut a release note
explicitly calling it out, and you can audit the request shape in the
source tree before deciding to leave the toggle on.

If you don't want to think about it, leave it off. Hoard works
identically either way.

---

## What's stored locally — and where

Hoard keeps three kinds of data on your machine:

1. **Credentials** — your bearer token, stored in your OS's secret
   store (Secret Service on Linux, DPAPI / Credential Manager on
   Windows, Keychain on macOS). The token is never written to a
   plaintext file.
2. **State** — `state.json` and `prefs.json`, listing which saves
   you're tracking, last-backed-up version, paths, and your settings.
   Stored under your platform's config dir
   (`~/.config/hoard/` on Linux).
3. **Logs** — daily-rotating text files under your platform's cache
   dir (`~/.cache/hoard/logs/` on Linux). Logs record file *counts*
   and *sizes*, never file contents.

Uninstalling the app does not wipe these by default —
`docs/install-client.md` lists the exact paths if you want to remove
them.

---

## What your server stores

The Hoard server is yours. What it keeps is up to you and is documented
in `docs/install.md` and the admin CLI's help text. In short: per-user
snapshots (versioned tarballs), an audit log of create/delete/restore
events, and a per-user quota counter. There is no per-server "phone
home" beacon — your server doesn't know that the Hoard project exists.

---

## Reporting a privacy issue

Found a place where the app does something this page doesn't describe?
That's a bug — please file a security advisory through GitHub
(Security tab → Report a vulnerability) and we'll patch it as a P0.
