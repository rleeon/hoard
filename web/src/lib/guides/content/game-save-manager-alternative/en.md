---
title: "GameSave Manager alternative: automatic backup and sync, Linux included"
description: "Looking for a GameSave Manager alternative? Hoard backs up every save on its own, syncs it between PCs and a Steam Deck, and runs on Linux and macOS."
order: 27
updated: 2026-11-05
related: game-save-sync-comparison, ludusavi-alternative, back-up-game-saves, transfer-game-saves-to-new-pc, sync-game-saves-across-pcs
---

Three different programs go by the name Game Save Manager, and all three back up saves on one Windows PC. If you want backups that happen on their own and follow you to your other PCs, a Steam Deck or Linux, [Hoard](/download) is free, open source and built for exactly that.

## Which Game Save Manager you have

- **GameSave Manager**, the long-running closed-source one for Windows. It has its own game database, backs up to a single archive on demand or on a schedule, and its **Sync & Link** moves a save folder into Dropbox, OneDrive or Google Drive and leaves a link behind.
- **Game Save Manager (GSM)** by dyang886, GPL-3.0, Windows. It finds installed games through the PCGamingWiki database, keeps a rolling set of local backups per game and carries your history to a new PC in a `.gsmr` archive.
- **game-save-manager** by mcthesw, AGPL-3.0, Windows, popular in Chinese communities. It backs up the folders you add on a schedule, with WebDAV for a copy off the machine.

## What they do well

- **Simple and local.** Pick games, press backup, get an archive. No account and nothing to set up.
- **Good detection on Windows.** Big game databases find most installed games without you hunting for paths.
- **A tidy move to a new PC.** One exported archive carries everything over.

If you only ever play on one Windows PC and are happy pressing a button, any of them does the job.

## Where Hoard goes further

| | Game Save Manager tools | Hoard |
|---|---|---|
| When it backs up | When you press the button, or on a schedule | Every time the game saves, and again when you quit |
| Sync between PCs | Export and import, or a cloud folder through Sync & Link | Automatic: the newest save is waiting on the next machine |
| Other PC can be off | Only through a cloud folder | Yes, the save waits on the server |
| History | Local backups, or whatever the cloud drive keeps | Every version, deduplicated, on every machine |
| Half-written saves | A cloud client can sync a linked folder while the game writes it | Never uploads a half-written file, never swaps a save under a running game |
| Platforms | Windows | Windows, Linux, macOS and SteamOS |
| Licence | Closed source, GPL-3.0 or AGPL-3.0 | AGPL-3.0, free |

The Linux row is the one people search for. None of the Game Save Manager tools runs on Linux or a Steam Deck. Hoard does, natively: the same app on all four systems, with Proton and Wine prefixes detected, so a game's save on your Windows PC and the same game's save on the Deck become two versions of one history.

## Moving over

1. Install [Hoard](/download) and sign in to Hoard One, recommended for everyone, or point it at [your own server](/guides/self-host-hoard).
2. If you used **Sync & Link**, put the save folder back in its original place first, so a cloud client and Hoard aren't both syncing the same files.
3. Open the **Library**. Hoard finds your games from the community save database and your launchers; add any folder it misses by hand.
4. Play. The first version appears in the history as soon as the game saves. Install Hoard on your other machines with the same account and [sync between PCs](/guides/sync-game-saves-across-pcs) happens on its own.

Your old Game Save Manager archives can stay where they are as a cold copy.

## Hoard One, or your own server

Hoard is free either way, and the app is the same on both:

- **Hoard One**, recommended for everyone. Nothing to set up and nothing to keep running: a managed server in the EU, free with 2 GB and three devices, no card needed. [Pro](/pricing) only adds room, 100 GB and unlimited devices, for when your saves outgrow that.
- **Your own server.** Run `hoard-server` on a PC, a NAS or a Raspberry Pi and every machine syncs through it. There's no account with us, no telemetry to us and no quota from us: nothing passes through our servers, because there's nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same history. Hoard One keeps every version for good; on your own server you decide how much history to keep.

<!-- faq -->

## Frequently asked questions

### Is there a GameSave Manager for Linux?

No: GameSave Manager is Windows only, and so are both open-source Game Save Managers. Hoard runs natively on Linux and SteamOS, finds saves inside Proton and Wine prefixes, and syncs them with your Windows PCs.

### Ludusavi or Game Save Manager?

Both are local backup tools. Ludusavi is open source, runs on Windows, Linux and macOS and is built on the community save manifest; the Game Save Managers are Windows only. If you want backup and sync without wiring up Rclone yourself, Hoard uses the same community data and syncs on its own. There's a full [Ludusavi comparison](/guides/ludusavi-alternative).

### Can I keep my old backups?

Yes. Hoard never touches them. Keep the archives as a cold copy; from the first save on, Hoard's own history takes over.

### Is Hoard free?

Yes. The app, the command line and the server are free and open source (AGPL-3.0), and running your own server costs nothing but the machine. Hoard One, the managed server we recommend for everyone, is free too with 2 GB and three devices; Pro adds room for €1.99 a month.

### Does self-hosting send anything to Hoard?

No. On your own server there's no account with us and no telemetry to us: your saves, your users and your logs stay on your machine and never touch ours.
