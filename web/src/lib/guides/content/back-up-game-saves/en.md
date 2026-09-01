---
title: "How to back up your game saves automatically"
description: "Set up automatic, versioned cloud backups for your PC game saves with Hoard — so a crash, reinstall or bad mod can never wipe your progress."
order: 1
updated: 2026-09-01
---

Losing a save file means losing hours of progress. Hoard backs up your PC game saves automatically and keeps a full version history, so you can always go back.

## What Hoard backs up

Hoard detects the save folders of the games you play and copies them to your own cloud — either Hoard Cloud or a server you host yourself. Every backup is versioned, so older copies are never overwritten.

To find where each game stores its saves, Hoard reads the same community save-location database that powers Ludusavi, so detection works out of the box for thousands of titles. The difference is what happens next: instead of leaving the backup on your disk, Hoard versions it in the cloud automatically.

## Set up automatic backups

1. **Download and install Hoard** for Windows, macOS or Linux from the download page.
2. Sign in, or point the app at your self-hosted server.
3. Open the **Library**. Hoard scans for installed games and lists the saves it finds.
4. Add the games you want to protect. Hoard locates each save folder automatically; you can add a path by hand if a game isn't detected.
5. Leave **automatic mode** on. Hoard watches the save folders and backs them up after you stop playing.

From now on every session is captured without you doing anything.

## Where PC games actually keep their saves

There is no single place, which is the whole reason a tool like this exists. In practice a save ends up in one of these:

- **Inside Steam**, at `userdata/<UserID>/<AppID>/remote/` — the folder Steam Cloud itself syncs.
- **`Documents\My Games\…`**, the closest thing Windows has to a convention.
- **`%APPDATA%`, `%LOCALAPPDATA%` or `LocalLow`** — where most Unity and Unreal games write.
- **`%USERPROFILE%\Saved Games`**, used by a smaller but stubborn set of titles.
- **The game's own install folder**, which is where a surprising number of older titles still save.
- **On Linux**, `~/.local/share` or `~/.config` for native games, and inside the Proton prefix — `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…` — for Windows games.
- **On macOS**, `~/Library/Application Support`.

Where the game came from barely matters: GOG, Epic and itch titles land in the same handful of places, because it's the engine and the developer that decide, not the launcher.

## What gets backed up, and what doesn't

A save folder is rarely just saves, so Hoard sorts what it finds into three piles:

- **Save data** is backed up and restored. This is your progress.
- **Files that belong to one machine** — configuration, logs, and similar — are uploaded so they're part of the backup, but never written back over another PC's copy. Your graphics settings stay yours.
- **Junk** — caches, crash dumps, temporary files — is ignored, so a backup doesn't balloon with things you'd never want back.

## When a backup happens

Hoard watches the folder and captures it **after you stop playing**, not while a game is holding files open. If the save was written to seconds ago, it waits until things go quiet: a file being written is not a file worth capturing halfway.

Each capture is a version. Snapshots are stored by content hash, so unchanged files are stored once — ten versions of a 2 GB save cost about 2 GB, not 20.

## Backing up without our servers

If you'd rather not use anyone's cloud, run `hoard-server` yourself and point the app at it. Your saves go from your PC to your disk: no account with us, no telemetry to us, and nothing passing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip: check your history

Open a game's **History** tab to see every backup with its date and size. From there you can restore any previous version in one click. Your saves travel encrypted, are stored in the EU, and you can export or delete them whenever you want.

Already use a local backup tool like Ludusavi? You can keep it — but if you want those backups to land in the cloud and sync between machines without scripting Rclone yourself, that's exactly what Hoard automates. See [Ludusavi vs Hoard](/guides/ludusavi-alternative) for a fair comparison.

<!-- faq -->

## Frequently asked questions

### Does Hoard back up while I'm playing?

No. It waits until you stop and the save folder goes quiet, so a backup is never a half-written file.

### How much space do my saves need?

Less than you'd think. Versions are deduplicated by content hash, so only what actually changed between sessions takes new space — most save collections sit comfortably in a couple of gigabytes.

### What if one of my games isn't detected?

Point Hoard at the folder by hand and it will track it like any other. Detection covers thousands of titles, but a game that saves somewhere unusual, or one you installed by hand, sometimes needs the hint.

### Does it back up my mods?

Hoard tracks the save folder, so mods living elsewhere aren't part of the backup. That's deliberate: mods are large, they're re-downloadable, and a mod folder syncing between machines causes more problems than it solves.

### Does self-hosting change how backups work?

Not at all. Same detection, same versions, same automatic capture. Only the storage is yours.
