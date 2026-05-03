# Quickstart — using the `hoard` CLI

This walkthrough assumes you already have a running `hoard-server` and an
admin-issued bearer token. If you don't, see
[install.md](install.md) first.

## 1. Configure the client

```sh
hoard config init --server http://your-server:8080
hoard config show
```

Config lives at `~/.config/hoard/config.toml` (perms `0600` on Unix).

## 2. Log in

You need a bearer token from the server admin:

```sh
# on the server side, the admin runs:
hoard-admin --config /etc/hoard/config.toml token create alice --device 'laptop'
# → prints `hoard_v1_<64 hex>` ONCE
```

Then on your machine:

```sh
hoard login --token hoard_v1_xxxxxxxxxxxxxxxxxxxxxxxx
hoard whoami
```

The token is validated against the server before being saved.

## 3. Pick a game and create a save namespace

```sh
hoard games search stardew
hoard games show stardew-valley
hoard save create --game stardew-valley --label main
hoard save list
```

A *save* is just a labelled bucket of snapshots scoped to one game. Use
labels like `main`, `speedrun-attempt-3`, `before-final-boss`. The
`--label` defaults to `default`.

## 4. Back up

```sh
SAVE_ID=...   # from `hoard save list`
hoard backup $SAVE_ID --from ~/.config/StardewValley/Saves --remember
```

`--remember` saves the (`save_id` → local path) mapping in
`~/.local/share/hoard/state.json`. Subsequent runs can omit `--from`:

```sh
hoard backup $SAVE_ID
```

The command walks the directory, builds a multipart upload (one part per
file, paths relative to `--from`), shows a progress bar, and the server
creates a new versioned snapshot. The version number autoincrements per
save.

## 5. List snapshots

```sh
hoard snapshots list $SAVE_ID
hoard snapshots list $SAVE_ID --all   # include soft-deleted
```

## 6. Restore

To restore the latest version into the remembered local path:

```sh
hoard restore $SAVE_ID --force   # --force allows extracting into a non-empty dir
```

To restore a specific version somewhere else:

```sh
hoard restore $SAVE_ID --version 3 --to /tmp/sd-checkpoint-3
```

Each file's SHA256 is verified against the server's manifest before being
written. Pass `--no-verify` to skip (not recommended).

## 7. Soft-delete + recover

```sh
hoard snapshots delete $SAVE_ID 2 --yes   # moves to trash
hoard snapshots list $SAVE_ID --all       # shows it as TRASH
hoard snapshots undelete $SAVE_ID 2       # back to active
```

Snapshots in the trash are eventually purged by the server's cleanup task
(`retention.trash_retention_days`, default 30).

## 8. Clean up

```sh
hoard save delete $SAVE_ID --yes   # deletes the save AND all its snapshots
hoard logout                        # clears the token from local config
```

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| `authentication failed: token rejected by server (401)` | Token revoked, expired, or server pointed at the wrong DB. Run `hoard login --token ...` with a fresh one. |
| `not found (404)` on save/snapshot | The id belongs to another user, or was deleted. (We don't return 403 to avoid enumeration.) |
| `payload too large (413)` | Snapshot exceeds `storage.max_snapshot_size_mb`. Bump it on the server. |
| `bad request (400): unsafe file path` | An entry tried to escape its destination (`..`, absolute, drive prefix). Should never happen for legitimate snapshots. |
| `bad request (400): no files uploaded` | Your CLI is mismatched with the server (the upload field name must be `files`). Update the CLI. |
