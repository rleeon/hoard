# Hoard HTTP API reference

Base URL: `http://your-server:8080` (or whatever `[server].public_url`
resolves to).

All endpoints except `/v1/health` require a bearer token:

```
Authorization: Bearer hoard_v1_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Get a token via `hoard-admin token create <username>` on the server.

## Conventions

- All bodies are JSON unless noted.
- Timestamps are RFC 3339 UTC strings (e.g. `2026-05-03T09:40:30Z`).
- IDs are UUIDv4 strings.
- 404 is returned for both "doesn't exist" and "exists but belongs to
  another user" (deliberate, to prevent enumeration).
- Error responses look like `{ "message": "..." }` for 4xx with a string,
  or are empty for 5xx (logs hold the detail).

## Endpoints

### `GET /v1/health` (public)

Liveness probe. No auth needed.

```sh
curl http://localhost:8080/v1/health
# → {"status":"ok","version":"0.1.0","uptime_secs":3600}
```

### `GET /v1/auth/whoami`

```sh
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/v1/auth/whoami
# → {"user_id":"...","username":"alice","is_admin":false}
```

### Games

```sh
GET /v1/games?search=stardew&limit=20
GET /v1/games/:slug
GET /v1/games/:slug/known-paths
GET /v1/manifest/version
```

`GET /v1/games` and `GET /v1/games/:slug` return `Game[]` / `Game`:

```json
{
  "slug": "stardew-valley",
  "display_name": "Stardew Valley",
  "engine": "monogame",
  "save_paths_json": "..."
}
```

`GET /v1/games/:slug/known-paths` returns the structured save-path manifest
the desktop client uses to decide where to look on disk:

```json
{
  "slug": "stardew-valley",
  "display_name": "Stardew Valley",
  "steam_app_id": 413150,
  "cloud_steam": true,
  "cloud_gog": false,
  "manifest_version": "2025-01-15",
  "paths": {
    "windows": [
      {
        "path": "<winAppData>/StardewValley/Saves",
        "constraints": [{ "store": "any" }],
        "tags": ["save"]
      }
    ],
    "linux": [
      { "path": "<xdgData>/StardewValley/Saves",
        "constraints": [{ "store": "any" }], "tags": ["save"] }
    ],
    "mac": []
  }
}
```

`GET /v1/manifest/version` returns the most recent manifest import metadata
(or 404 if no import has happened on this server):

```json
{
  "source": "ludusavi-manifest",
  "manifest_version": "2025-01-15T12:34:56Z",
  "imported_at": "2025-01-15T12:34:58Z",
  "games_inserted": 11432,
  "games_updated": 56,
  "games_pruned": 12
}
```

### Saves

```sh
GET    /v1/saves?game_slug=stardew-valley
POST   /v1/saves                {"game_slug":"...","label":"main"}
GET    /v1/saves/:id
PATCH  /v1/saves/:id            {"label":"new-label"}
DELETE /v1/saves/:id
```

`Save`:

```json
{
  "id": "uuid",
  "user_id": "uuid",
  "game_slug": "stardew-valley",
  "label": "main",
  "latest_version_num": 3,
  "snapshot_count": 3,
  "total_size_bytes": 12345,
  "created_at": "2026-05-03T09:40:22Z",
  "updated_at": "2026-05-03T10:00:00Z"
}
```

### Snapshots

```sh
GET    /v1/saves/:save_id/snapshots?include_deleted=false
POST   /v1/saves/:save_id/snapshots                  # multipart upload
GET    /v1/saves/:save_id/snapshots/:version
DELETE /v1/saves/:save_id/snapshots/:version
GET    /v1/saves/:save_id/snapshots/:version/download
POST   /v1/saves/:save_id/snapshots/:version/restore
```

#### Upload (`POST .../snapshots`)

Multipart form-data with **one or more `files` parts**. Each part:

- field name = `files` (or `files[]`)
- `filename=` header = the relative path inside the snapshot
  (e.g. `sub/config.json`)
- body = the file's bytes

Server enforces:

- Total snapshot size ≤ `[storage].max_snapshot_size_mb`.
- File count ≤ 1000.
- User's `storage_used + new` ≤ `storage_quota_bytes`.
- Path safety: rejects empty, absolute, `..`, drive-letter prefixes.

curl example uploading two files:

```sh
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "files=@save01.dat;filename=save01.dat" \
  -F "files=@config.json;filename=sub/config.json" \
  http://localhost:8080/v1/saves/$SAVE_ID/snapshots
```

Response (`Snapshot`):

```json
{
  "id": "uuid",
  "version_num": 3,
  "file_count": 2,
  "total_size_bytes": 27,
  "is_pinned": false,
  "deleted_at": null,
  "created_at": "2026-05-03T09:40:30Z"
}
```

#### Detail (`GET .../snapshots/:version`)

Returns the snapshot summary plus the file manifest:

```json
{
  "id": "...", "version_num": 3, ...,
  "files": [
    { "relative_path": "save01.dat", "size_bytes": 16, "sha256": "abc..." },
    { "relative_path": "sub/config.json", "size_bytes": 11, "sha256": "def..." }
  ]
}
```

The CLI uses this to verify SHA256s on restore.

#### Download (`GET .../download`)

Streams a `.tar.zst` (zstd-compressed tar) of the snapshot. The body is a
chunked stream; clients should decode on the fly.

```sh
curl -H "Authorization: Bearer $TOKEN" \
     -o snapshot.tar.zst \
     http://localhost:8080/v1/saves/$SAVE_ID/snapshots/3/download
zstd -d snapshot.tar.zst -o snapshot.tar
tar -xf snapshot.tar
```

#### Soft delete + restore (server-side)

```sh
DELETE /v1/saves/:save_id/snapshots/:version   # → 204, moves to trash
POST   /v1/saves/:save_id/snapshots/:version/restore   # → 204, moves back
```

Soft-deleted snapshots stay in `trash/<id>/` until the cleanup task purges
them after `retention.trash_retention_days`. They can be `undeleted` at
any time before that.

## Status codes

| Code | Meaning |
|---|---|
| 200 | OK with JSON body |
| 204 | OK, no body |
| 400 | Bad request — see message |
| 401 | Missing/invalid bearer token |
| 403 | Reserved (currently unused; we use 404 for cross-user access) |
| 404 | Resource doesn't exist or doesn't belong to you |
| 409 | Conflict — e.g. duplicate `(game_slug, label)` |
| 413 | Payload too large — exceeds `max_snapshot_size_mb` |
| 500 | Internal server error — check logs |
