---
title: "Game save sync compared: Hoard vs Ludusavi, Syncthing, OpenSave and the rest"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager, SaveState, EmuSync and more compared: what each one does best, and a side-by-side table."
order: 4
updated: 2026-11-05
related: ludusavi-alternative, steam-cloud-alternative, syncthing-game-saves, game-save-manager-alternative, opencloudsaves-alternative, savesync-alternative
---

Steam Cloud only covers games you bought on Steam, and only when the developer bothered to switch it on. Emulators, GOG, Epic, itch.io, non-Steam games, anything modded: none of that is covered. If you play on more than one machine, a desktop and a Steam Deck say, you end up copying folders by hand and hoping you grabbed the newest one.

Several tools fix this, and they don't all do the same thing. Some make local backups, some mirror folders between devices, some upload to a cloud. This page goes through them and says what each one is genuinely best at. Hoard is my project, so it comes last, with what it doesn't do, and a table you can read without trusting a word of the prose.

## Ludusavi

The best-known one, and deservedly so. Ludusavi (by mtkennerly) is a free, open-source backup tool with a GUI and a CLI, and it's built on the community save-location manifest that covers tens of thousands of games, the same manifest most of the tools here use, Hoard included. It keeps versioned local backups and can push them to your own cloud through Rclone.

**Best if:** you want local backups, full control, and no server anywhere. It's the safest default on this list and costs nothing.

**Where it stops:** cross-machine sync is a thing you assemble. Schedule a backup, configure an Rclone remote, remember to restore on the other PC *before* you play. It works, but nothing stops you forgetting the last step.

## Syncthing

Not a game tool at all: a general-purpose, peer-to-peer folder mirror, and a very good one. Point it at a save folder and it appears on your other devices.

**Best if:** you already run it and you want files in two places with no cloud in between.

**Where it stops:** it mirrors, it doesn't snapshot. A corrupted save reaches every device in seconds, exactly as fast as a good one. Its file versioning is per-file, with no idea what a play session is, so "roll back to how it was on Tuesday night" is something you reconstruct by hand. Two machines that both played offline give you conflict files, not a merge.

## OpenSave

Peer-to-peer sync built specifically for saves, in Go, MIT licensed, for Windows, Linux, Steam Deck and, since version 2.4, macOS. No account, no server: devices pair with each other and sync over the LAN or through a relay room code. It snapshots every change, has branches for parallel playthroughs, resolves conflicts by sync lineage rather than clock timestamps, and transfers only changed blocks. It can optionally mirror to Drive, Dropbox, OneDrive or WebDAV.

**Best if:** you refuse to have an account, and your devices are on together often enough to actually meet.

**Where it stops:** peer-to-peer means the save lives only on your devices. If the Deck holding the only recent copy dies and the mirror was never configured, that's it. Both devices have to be running for a sync to happen.

## OpenCloudSaves

A cross-platform GUI that syncs your save folders into a cloud you already pay for (OneDrive, Google Drive, Dropbox, Nextcloud), using Rclone underneath.

**Best if:** you want your saves in a storage account you already have, with a UI instead of Rclone config files.

**Where it stops:** the project was archived in January 2025 and is no longer maintained, so it won't get fixes for new games or system updates. On top of that, there's no content-level deduplication. Ten copies of a 2 GB save is 20 GB of your Drive quota, and cloud drives sync files, not play sessions, so what you get back is whatever the folder looked like at the time.

## Game Backup Monitor

Windows-first, and the original of this whole genre. GBM watches for a game process, and when you quit, it compresses the save with 7-Zip and keeps a numbered history.

**Best if:** you're on one Windows PC and want a compressed local archive with zero thinking.

**Where it stops:** it's a backup tool, not a sync tool. Getting the archive onto a second machine is your problem, and Steam Deck / SteamOS is not its home turf.

## GameSave Manager

A long-standing free Windows tool, closed source, with its own database of games. It backs up and restores, and its best-known feature, **Sync & Link**, moves a save folder into a cloud folder such as Dropbox or OneDrive and leaves a link behind, so the cloud client keeps it in sync.

**Best if:** you're on Windows, you already live in Dropbox or OneDrive, and you want the save simply to be in there.

**Where it stops:** Windows only, and Sync & Link means a general-purpose cloud client syncs the live folder while the game writes to it, the same pattern that makes [Syncthing risky for saves](/guides/syncthing-game-saves). Version history is whatever your cloud drive keeps.

## Game Save Manager (GSM)

Not the same program as the one above, despite the name. GSM, by dyang886, is a GPL-3.0 Windows app that finds installed games across Steam, Epic, Battle.net and other launchers through the PCGamingWiki database, keeps a rolling set of local backups per game up to a limit you set, and carries your whole history to a new PC in an exported `.gsmr` archive. There's even a third namesake: mcthesw's game-save-manager, popular in Chinese communities, which backs up folders you add by hand on a schedule, with WebDAV for an off-machine copy.

**Best if:** you're on one Windows PC and want versioned local backups with almost no setup, plus an easy move to a new machine.

**Where it stops:** there's no sync between devices: the second PC gets your saves when you export an archive and import it there. Windows only.

## SaveState

SaveState, by Matteo842, is a GPL-3.0 backup manager for Windows and Linux, with a separate Android app. Drop a launcher shortcut on it and it finds the save folder, and its emulator support runs deep: RetroArch, Dolphin, PCSX2 memory cards, xemu and more than twenty others. Backups are compressed `.zip` archives with rotation, and an optional cloud sync copies them to Google Drive, WebDAV, FTP, a network share or Git.

**Best if:** you want a friendly backup tool with wide emulator coverage, and you don't mind restoring by hand on the other machine.

**Where it stops:** it's built around backup archives rather than the live save. The cloud sync moves those archives between machines, but putting one into the game's folder on the other side is still a Restore you run. No macOS.

## EmuSync

EmuSync is a GPL-3.0 sync tool for PC and emulator saves, built first for a Windows PC and a Steam Deck. You map each game's save location on every device to one sync point in your own Google Drive, Dropbox or OneDrive, or a shared folder, and it syncs automatically or on demand. A Decky plugin shows sync status in Game Mode.

**Best if:** you play across a Windows PC and a Deck and want your saves in a cloud drive you already have.

**Where it stops:** the cloud drive holds the current state, and backup and restore are local to each device rather than a history shared between them. macOS isn't tested, and the latest release is from February 2026.

## Aletheia

One of the newer ones, AGPL, and it goes after the part everyone else half-covers: launchers. Heroic, itch.io, Lutris, Steam, GOG Galaxy and Xbox, across Windows, Linux and macOS.

**Best if:** your library is spread across launchers that other tools detect badly, especially Xbox/Game Pass and Heroic.

**Where it stops:** it's a young project with a deliberately narrow scope. Backup and restore is the feature set; there's no versioned cloud behind it.

## SaveSync

The commercial one, sold on Steam as a one-time purchase, Windows-focused. Its trick is that it isn't really aimed at you-on-two-PCs: it's aimed at co-op. Saves go into private, unlisted Steam Workshop entries so a friend can pull your Valheim or Factorio world, and there's LAN sync too.

**Best if:** the problem you're solving is "my friend hosts and I need their save", not "my saves follow me".

**Where it stops:** closed source, Windows, tied to Steam as the transport, and a set of supported co-op games rather than everything you own.

## Tachyon

The newest name here, in free beta on Windows. Tachyon detects saves for 5,000+ PC games, syncs them through its own cloud and keeps a version history with multiple "timelines".

**Best if:** you're on Windows, want something that works out of the box, and don't mind that it's a beta.

**Where it stops:** Windows only for now (Linux and macOS are listed as coming), so no Steam Deck yet; no published source code; no way to run your own server. Pricing after the beta hasn't been announced.

## Steam Deck plugins (Decky)

On the Deck itself, a few Decky Loader plugins cover part of the job from Game Mode. Decky Cloud Save syncs folders to OneDrive, Google Drive or Dropbox through rclone after a game closes; its author no longer maintains it and points to two successors, Decky Ludusavi (a Game Mode front end for Ludusavi) and SDH-GameSync (a fork that pulls saves when each game launches and pushes them when it closes). Steamback keeps the ten most recent local snapshots of games that use Steam Cloud, taken when you exit.

**Best if:** you play mostly on the Deck and want something you can drive without leaving Game Mode.

**Where it stops:** they're Deck-only, and the cross-device part leans on something else: rclone and your cloud drive, Ludusavi, or Steam Cloud itself. Steamback's snapshots stay on the Deck and only cover Steam Cloud games.

## A note on EmuDeck

EmuDeck comes up in these conversations, and it isn't a competitor in the normal sense: it's an emulator installer and configurator for Steam Deck, and the sync it offers is a convenience bolted onto that job (Rclone against a cloud drive, for emulator saves only). It overlaps with the tools above without being the same kind of thing: EmuDeck sets your emulators up, the tools here look after saves for the whole library. People do run EmuDeck alongside one of these, and that's a sensible setup, not a redundant one.

## Hoard

Hoard backs up every save as soon as the game finishes writing it, and keeps it in sync across every machine you play on. The engine runs as a background service, `hoardd`, with no window, so it keeps working in SteamOS Game Mode. It waits until the folder has been quiet for a few seconds, so it never uploads a half-written file; it takes one more copy when you quit; and it never swaps a save under a running game: whatever comes down from another machine waits until you close the game.

- **The other machine can be off.** The newest save waits on the server, so the Deck you pick up next week gets it without the PC being on. Peer-to-peer tools need both devices awake at the same time.
- **Every version, kept.** Each save is a version you can roll back to from the app or with `hoard restore`, including after a disk failure or a fresh install. Hoard One keeps every version for good; on your own server you decide how much history to keep.
- **Only what changed travels.** Uploads send only the files that changed, compressed with zstd, and versions share everything that didn't change: ten versions of a 2 GB save where little changes cost about 2 GB, not 20.
- **SHA-256 on the way up and on the way down.** Corruption is caught before it can overwrite a good save, and Hoard keeps a copy of anything it replaces.
- **Free and open source, with your own server if you want one.** The app, the CLI and the server are AGPL-3.0. Hoard One, the managed server we recommend for everyone, is free with 2 GB and three devices, and there's nothing to set up. Or run `hoard-server` with Docker on a PC, a NAS or a Raspberry Pi, on local disk or any S3-compatible storage: no account with us, no telemetry to us, no quota from us.
- **Windows, Linux, macOS and SteamOS**, with the same desktop app on all four and a CLI for servers and headless boxes.
- **Detection that covers the whole library:** Steam, Epic, GOG, Proton and Wine prefixes, thousands of games through the community save database, and presets for 19 emulators, from PCSX2, Dolphin and RPCS3 to Ryujinx, Cemu, DuckStation and RetroArch.

## The detail that decides Steam Deck ↔ PC sync

Worth knowing whichever tool you pick. A Steam game's cloud save lives in `<AppID>/remote/`, and the folder *above* it holds `remotecache.vdf`, achievement state, stats and playtime counters, all of which legitimately differ between your Deck and your desktop.

Sync the parent folder and you get a permanent conflict between two machines that never disagreed about a single save. Hoard tracks `remote/`, not the parent. Any tool pointed at a folder by hand can be told to do the same, and it's the first thing to check when a sync setup keeps flagging conflicts for no visible reason.

## What Hoard doesn't do

- **Sync without a server.** Hoard always syncs through one: Hoard One if you'd rather not run anything, or your own. That's exactly what lets the other device be off, and what leaves a copy that survives both machines. If you want no server under any circumstances, OpenSave, Syncthing or Ludusavi are built for that.
- **Co-op world sharing.** Handing a world to a friend is what SaveSync is built for; Hoard looks after your own saves.
- **Merging two saves that diverged.** No tool can merge two playthroughs into one. Hoard keeps both as versions, on every machine, and you pick.

## The Hoard One / self-host distinction

Comparisons of Hoard almost always collapse these two into one, and the result is wrong, so it's worth stating plainly:

- **Hoard One** is the managed option, and the one we recommend for everyone: you sign in, and your saves are stored on our servers, in the EU.
- **A self-hosted Hoard is entirely yours.** You run `hoard-server` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay**: nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, because none of it ever reaches us. If Hoard One shut down tomorrow, a self-hosted setup would carry on unchanged.

Same code, same detection. What changes is who owns the storage, plus one default: a self-hosted server thins out old versions over time unless you switch that off. Being exact about one detail: your own server does have logins of its own (a user and a token per device), but they live in your database, not ours.

## The table

| Tool | Automatic sync | Works with the other device off | Where saves live | History | Platforms | Licence |
|---|---|---|---|---|---|---|
| **Hoard** | Yes, every time the game saves | Yes | Hoard One or your own server (disk or S3-compatible) | Every version, deduplicated | Win · Linux · macOS · Deck | AGPL-3.0, free |
| **Ludusavi** | Manual, or Rclone that you wire up | Only with an Rclone remote | Local, plus your Rclone remote | Versioned local backups | Win · Linux · macOS | MIT, free |
| **Syncthing** | Yes, continuous mirror | No, both devices must be on | Your devices only | Per-file versioning | Everything | MPL-2.0, free |
| **OpenSave** | Yes, peer-to-peer | No, unless you set up a cloud mirror | Your devices, optional cloud mirror | Snapshots and branches | Win · Linux · macOS · Deck | MIT, free |
| **OpenCloudSaves** | Via your cloud drive | Yes, through the drive | OneDrive / Drive / Dropbox / Nextcloud | Whatever the drive keeps | Win · Linux · macOS | Open source, archived |
| **Game Backup Monitor** | No | No sync | Local 7-Zip archives | Numbered backups | Windows · Linux | GPL-3.0, free |
| **GameSave Manager** | Via your cloud drive (Sync & Link) | Yes, through the drive | Local, plus Dropbox / OneDrive | Whatever the drive keeps | Windows | Free, closed source |
| **Game Save Manager (GSM)** | No, export and import | No sync | Local backups | Rolling local backups | Windows | GPL-3.0, free |
| **SaveState** | Optional, of the backup archives | Yes, through your storage | Local zips, plus Drive / WebDAV / FTP / SMB / Git | Rotating archives | Windows · Linux | GPL-3.0, free |
| **EmuSync** | Yes, via your cloud drive | Yes, through the drive | Drive / Dropbox / OneDrive or a shared folder | Local backups | Windows · Linux · Deck | GPL-3.0, free |
| **Aletheia** | Backup and restore per launcher | No sync | Your storage | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Yes, and with friends | Yes, through Steam | Private Steam Workshop entries | Per the app | Windows | Paid, closed source |
| **Tachyon** | Yes, through its own cloud | Yes | Tachyon's cloud | Versions and timelines | Windows (beta) | Free beta, no published source |

## So which one

If you want backups and sync to just happen, across PCs and a Steam Deck, with every version kept, the other machine free to stay off, and the option to own the whole thing, that's Hoard. [Download it](/download), sign in to Hoard One, or read [how to self-host it with Docker](/guides/self-host-hoard) first. It's free either way.

For narrower jobs: one Windows PC and local archives only, Game Backup Monitor or GSM. Local backups with full control and no server anywhere, Ludusavi. No account under any circumstances, with devices that are usually on together, OpenSave or Syncthing. Saves that must live in a Drive folder you already pay for, Ludusavi with an Rclone remote, or EmuSync; OpenCloudSaves did that, but it's archived. Sharing a co-op world with friends, SaveSync.

If you came from OpenCloudSaves and mainly wanted your saves in storage you control, rather than in Drive as such, a self-hosted Hoard does that with history on top: it writes to local disk or any S3-compatible bucket, such as Backblaze B2 or MinIO. There's also a longer [Ludusavi comparison](/guides/ludusavi-alternative) if that's the one you're weighing it against.

## One-on-one comparisons

Each of these goes deeper than the section above, including where the other tool wins:

- [Hoard vs Ludusavi](/guides/ludusavi-alternative)
- [Hoard as a Steam Cloud alternative](/guides/steam-cloud-alternative)
- [Peer-to-peer sync vs a server you own](/guides/opensave-alternative)
- [Syncthing for game saves: what breaks](/guides/syncthing-game-saves)
- [Hoard vs GameSave Manager and the other Game Save Managers](/guides/game-save-manager-alternative)
- [Replacing OpenCloudSaves now that it's archived](/guides/opencloudsaves-alternative)
- [A free alternative to SaveSync](/guides/savesync-alternative)

<!-- faq -->

## Frequently asked questions

### Which of these tools keeps a version history?

Hoard keeps every save as a version you can roll back to, on every machine. Ludusavi, GSM and SaveState keep versioned or rotating local backups, and OpenSave keeps snapshots on your devices. Most of the rest sync or copy the current state, which means a corrupted save is faithfully propagated to your other machine.

### Which one works without any server or account?

Ludusavi with local backups, and any peer-to-peer tool. Hoard also qualifies if you self-host: no account with us, and nothing passing through our servers.

### Which one syncs when my other device is off?

Any tool that keeps the save somewhere other than your devices: Hoard, through Hoard One or your own server, and the tools that write to a cloud drive, such as EmuSync or Ludusavi with Rclone. Peer-to-peer sync (OpenSave without its mirror, Syncthing) needs both devices on at the same time.

### Which one covers games that aren't on Steam?

All the save-manager tools here do, because they locate saves through the same community database rather than through a store. Steam Cloud is the one that doesn't: it only covers Steam games whose developer enabled it.

### Do I have to pick just one?

No, and plenty of people don't. A local backup tool and a sync tool solve different halves of the problem. The only rule is never to point one tool at another's backup folder, or you end up syncing a stale mirror instead of your live save.

### What's the single detail that breaks most DIY setups?

Syncing the folder above `<AppID>/remote/` in Steam's `userdata`. The parent holds `remotecache.vdf` plus achievement and playtime files that are supposed to differ per machine, so every launch looks like a conflict even though no save moved.
