---
title: "OpenCloudSaves alternative: it's archived, so here's what to use instead"
description: "OpenCloudSaves was archived in January 2025. Hoard is a maintained, open-source replacement that syncs saves automatically and keeps every version."
order: 28
updated: 2026-11-05
related: game-save-sync-comparison, ludusavi-alternative, onedrive-game-saves, syncthing-game-saves, self-host-hoard
---

OpenCloudSaves was archived on 20 January 2025. The code and the last release are still there, but the project takes no more changes, so nothing will fix it for new games, new launchers or system updates. If it was syncing your saves through a cloud drive, [Hoard](/download) is a free, open-source replacement that's actively maintained, finds your saves on its own and syncs them every time the game writes one.

## What OpenCloudSaves did well

- **Your own cloud.** Saves went to the OneDrive, Google Drive, Dropbox or Nextcloud you already had, through Rclone, with a real interface on top instead of config files.
- **Careful by default.** It ran a dry run before every sync, so you could see what would change before it did.
- **Filters and custom games.** You could sync only certain files, leave graphics settings out, and define games yourself, even across stores.
- **Every desktop.** Windows, macOS and Linux, Steam Deck included.

## What a replacement needs

- **Someone maintaining it.** Save locations move and launchers change; a sync tool nobody updates ends up syncing the wrong folder.
- **A history, not only the latest copy.** A cloud drive syncs files, so a corrupted save reaches every machine as faithfully as a good one.
- **No half-written files.** A general-purpose sync can catch a save while the game is still writing it; the [OneDrive guide](/guides/onedrive-game-saves) shows what that breaks.

## Hoard as the replacement

| | OpenCloudSaves | Hoard |
|---|---|---|
| Maintained | Archived since January 2025 | Yes, open source (AGPL-3.0) |
| Finding saves | Save definitions, plus folders you add | Automatic: community save database, Steam, Epic, GOG, Proton and Wine prefixes, 19 emulators |
| When it syncs | When a sync runs, after a dry run | Every time the game saves, and again when you quit |
| Where saves live | Your cloud drive | Hoard One, or your own server on local disk or any S3-compatible storage |
| History | Whatever the drive keeps | Every version, deduplicated |
| Ten versions of a 2 GB save | About 20 GB of drive quota | About 2 GB, because versions share what didn't change |
| Platforms | Windows, macOS, Linux | Windows, Linux, macOS and SteamOS |

The one thing Hoard won't do is write into your Google Drive or OneDrive. It syncs through a server instead, which is what gives it the history and lets the other machine be off. If what you wanted was storage you control rather than that particular drive, a self-hosted Hoard writes to a NAS, a local disk or any S3-compatible bucket such as Backblaze B2. If it has to be the drive itself, Ludusavi with an Rclone remote is the closest maintained option.

## Switching over

1. Before changing anything, copy your save folders somewhere safe.
2. Stop OpenCloudSaves from syncing those folders, so two tools never write to the same save.
3. Install Hoard and sign in to Hoard One, recommended for everyone, or point it at [your own server](/guides/self-host-hoard).
4. Open the **Library**. Hoard finds your games; add any custom folder by hand. The first version appears in the history as soon as the game saves.
5. Install Hoard on your other machines with the same account.

## Hoard One, or your own server

Hoard is free either way, and the app is the same on both:

- **Hoard One**, recommended for everyone. Nothing to set up and nothing to keep running: a managed server in the EU, free with 2 GB and three devices, no card needed. [Pro](/pricing) only adds room, 100 GB and unlimited devices, for when your saves outgrow that.
- **Your own server.** Run `hoard-server` on a PC, a NAS or a Raspberry Pi and every machine syncs through it. There's no account with us, no telemetry to us and no quota from us: nothing passes through our servers, because there's nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same history. Hoard One keeps every version for good; on your own server you decide how much history to keep.

<!-- faq -->

## Frequently asked questions

### Is OpenCloudSaves still safe to use?

The last release is still available, but the project is archived: no fixes for new games or system changes. It was also released as a beta whose author asked users to back up their saves before using it. If you stay on it, keep a manual copy.

### Can Hoard store saves in Google Drive or OneDrive?

No. Hoard syncs through a server: Hoard One, or your own on local disk or S3-compatible storage. Your `Documents` folder can still live in OneDrive; Hoard asks Windows where it really is and finds the saves there.

### Is Hoard free?

Yes. The app, the command line and the server are free and open source (AGPL-3.0), and running your own server costs nothing but the machine. Hoard One, the managed server we recommend for everyone, is free too with 2 GB and three devices; Pro adds room for €1.99 a month.

### Does self-hosting send anything to Hoard?

No. On your own server there's no account with us and no telemetry to us: your saves, your users and your logs stay on your machine and never touch ours.
