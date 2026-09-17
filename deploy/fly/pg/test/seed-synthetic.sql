-- Synthetic data at roughly prod scale (16-sep-2026: 220 profiles, 2.015
-- saves, 20k versions, 245k file entries, 734k version files, 169k blobs),
-- for timing a dump and restore and proving the checksums without any real
-- row leaving Supabase. Values carry the awkward cases on purpose: NULLs,
-- quotes, tabs, newlines, non-ASCII, emoji, microseconds and bytea.
--
-- Triggers are off so 169k blob inserts do not each update profiles. Foreign
-- keys stay valid by construction, because pg_restore checks them when it
-- adds them back.

\set ON_ERROR_STOP on
SET session_replication_role = replica;

INSERT INTO auth.users (id)
SELECT md5('user' || n)::uuid FROM generate_series(1, 220) n;

INSERT INTO profiles (user_id, email, display_name, plan, storage_bytes, devices_count, created_at,
                      updated_at, lifetime_storage_bytes, email_canonical, max_versions)
SELECT md5('user' || n)::uuid,
       'user' || n || '@example.test',
       CASE WHEN n % 7 = 0 THEN NULL
            WHEN n % 5 = 0 THEN 'Jugadör "' || n || E'"\nñ'
            ELSE 'player ' || n END,
       CASE WHEN n % 9 = 0 THEN 'pro' ELSE 'free' END,
       (random() * 2e9)::bigint,
       1 + n % 2,
       timestamptz '2026-06-01' + n * interval '37 minutes 13.123456 seconds',
       now(),
       (random() * 3e9)::bigint,
       'user' || n || '@example.test',
       CASE WHEN n % 50 = 0 THEN 20 END
  FROM generate_series(1, 220) n;

INSERT INTO devices (id, user_id, device_name, device_kind, os, fingerprint, last_seen_at, created_at,
                     app_version, playing, closed_at)
SELECT md5('device' || n)::uuid,
       md5('user' || ((n - 1) % 220 + 1))::uuid,
       'PC-' || n,
       CASE n % 3 WHEN 0 THEN 'desktop' WHEN 1 THEN 'handheld' END,
       (ARRAY['linux', 'windows', 'macos'])[n % 3 + 1],
       'fp-' || n,
       now() - n * interval '1 hour',
       timestamptz '2026-06-01' + n * interval '2 hours 24 minutes',
       '1.1.' || (n % 7),
       CASE WHEN n % 11 = 0 THEN '{"game": "factorio", "since": "2026-09-15T10:00:00Z"}'::jsonb END,
       CASE WHEN n % 4 = 0 THEN now() - interval '3 hours' END
  FROM generate_series(1, 285) n;

INSERT INTO saves (id, user_id, game_slug, label, local_path_hint, client_os, latest_version_num,
                   created_at, updated_at, backup_only, archived_at)
SELECT md5('save' || n),
       md5('user' || ((n - 1) % 220 + 1))::uuid,
       'game-' || n,
       CASE WHEN n % 13 = 0 THEN 'slot 2' ELSE 'default' END,
       CASE WHEN n % 3 = 0 THEN 'C:\Users\Jugador\AppData\Local\Game ' || n END,
       (ARRAY['linux', 'windows', 'macos'])[n % 3 + 1],
       10,
       timestamptz '2026-06-01' + n * interval '53 minutes',
       now() - n * interval '7 minutes',
       n % 17 = 0,
       CASE WHEN n % 101 = 0 THEN now() - interval '2 days' END
  FROM generate_series(1, 2015) n;

INSERT INTO save_versions (id, save_id, version_num, size_bytes, sha256, r2_key, device_id, notes,
                           is_pinned, deleted_at, created_at, parent_version, file_count,
                           content_addressed, device_name, insight)
SELECT (s - 1) * 10 + v,
       md5('save' || s),
       v,
       (random() * 5e7)::bigint,
       md5('ver' || s || '-' || v) || md5('x' || s || '-' || v),
       'blobs/' || md5('save' || s) || '/' || v,
       CASE WHEN v % 4 = 0 THEN NULL ELSE md5('device' || ((s - 1) % 285 + 1))::uuid END,
       CASE WHEN v = 3 THEN E'before the boss\t''quoted'' 🎮' END,
       v = 1,
       CASE WHEN v = 2 AND s % 5 = 0 THEN now() - interval '1 day' END,
       timestamptz '2026-07-01' + ((s - 1) * 10 + v) * interval '1 minute 1.000001 seconds',
       NULLIF(v - 1, 0),
       36,
       true,
       'PC-' || s,
       CASE WHEN v % 3 = 0 THEN jsonb_build_object('files', 36, 'delta', -1.5e-3, 'label', 'ÿ') END
  FROM generate_series(1, 2015) s, generate_series(1, 10) v;

INSERT INTO file_entries (id, save_id, relative_path, sha256, size_bytes)
SELECT (s - 1) * 120 + f,
       md5('save' || s),
       CASE WHEN f % 10 = 0 THEN 'saves/Ñandú ' || f || '.sav' ELSE 'saves/slot' || f || '.dat' END,
       sha256(convert_to('entry' || s || '-' || f, 'UTF8')),
       (random() * 1e6)::bigint
  FROM generate_series(1, 2015) s, generate_series(1, 120) f;

INSERT INTO version_files (version_id, entry_id, modified_at)
SELECT (s - 1) * 10 + v,
       (s - 1) * 120 + (v - 1) * 8 + k,
       CASE WHEN k % 9 = 0 THEN NULL ELSE 1757000000000 + s * 1000 + k END
  FROM generate_series(1, 2015) s, generate_series(1, 10) v, generate_series(1, 36) k;

INSERT INTO cloud_blobs (user_id, sha256, size_bytes, refcount, created_at, purge_after, encoding,
                         stored_bytes, last_presigned_at, compress_attempts, verified_at, integrity)
SELECT md5('user' || u)::uuid,
       sha256(convert_to('blob' || u || '-' || b, 'UTF8')),
       (random() * 2e6)::bigint,
       b % 5,
       timestamptz '2026-07-01' + b * interval '13 seconds',
       CASE WHEN b % 5 = 0 THEN now() + interval '7 days' END,
       CASE WHEN b % 3 = 0 THEN 'zstd' END,
       CASE WHEN b % 3 = 0 THEN (random() * 1e6)::bigint END,
       CASE WHEN b % 2 = 0 THEN now() - b * interval '1 second' END,
       (b % 3)::smallint,
       CASE WHEN b % 4 = 0 THEN now() END,
       CASE WHEN b % 4 = 0 THEN 'ok' END
  FROM generate_series(1, 220) u, generate_series(1, 768) b;

INSERT INTO sync_log (id, user_id, save_id, version_num, kind, bytes, device_id, metadata, at)
SELECT n,
       md5('user' || ((n - 1) % 220 + 1))::uuid,
       CASE WHEN n % 20 = 0 THEN NULL ELSE md5('save' || ((n - 1) % 2015 + 1)) END,
       n % 10 + 1,
       (ARRAY['upload', 'download', 'restore', 'delete'])[n % 4 + 1],
       (random() * 1e7)::bigint,
       CASE WHEN n % 6 = 0 THEN NULL ELSE md5('device' || ((n - 1) % 285 + 1))::uuid END,
       jsonb_build_object('app', '1.1.6', 'ms', n % 997,
                          'note', CASE WHEN n % 50 = 0 THEN E'multi\nline "q"' END),
       timestamptz '2026-08-01' + n * interval '31.5 seconds'
  FROM generate_series(1, 78565) n;

INSERT INTO client_logs (id, user_id, device_id, device_name, device_os, device_fingerprint, app_version,
                         level, target, message, fields, client_ts, received_at)
SELECT md5('log' || n)::uuid,
       md5('user' || ((n - 1) % 220 + 1))::uuid,
       CASE WHEN n % 8 = 0 THEN NULL ELSE md5('device' || ((n - 1) % 285 + 1))::uuid END,
       'PC-' || n % 285,
       'linux',
       'fp-' || n % 285,
       '1.1.' || n % 7,
       (ARRAY['INFO', 'WARN', 'ERROR', 'debug'])[n % 4 + 1],
       (ARRAY['hoard::agent', 'hoard::screen', 'hoard::telemetry'])[n % 3 + 1],
       'agent: backup throttled by bandwidth limit, waiting to retry ' || repeat(md5(n::text), 8),
       jsonb_build_object('save_id', md5('save' || n % 2015), 'attempt', n % 5,
                          'path', 'C:\Users\Jugador\Saved Games'),
       timestamptz '2026-09-01' + n * interval '17.25 seconds',
       timestamptz '2026-09-01' + n * interval '17.26 seconds'
  FROM generate_series(1, 36105) n;

INSERT INTO playtime (user_id, device_fp, day, game_slug, secs, updated_at)
SELECT md5('user' || ((n - 1) % 220 + 1))::uuid,
       'fp-' || n % 285,
       date '2026-08-01' + n % 45,
       'game-' || n,
       n * 37 % 20000,
       now()
  FROM generate_series(1, 1802) n;

SELECT setval('save_versions_id_seq', (SELECT max(id) FROM save_versions));
SELECT setval('file_entries_id_seq', (SELECT max(id) FROM file_entries));
SELECT setval('sync_log_id_seq', (SELECT max(id) FROM sync_log));

RESET session_replication_role;
ANALYZE;
